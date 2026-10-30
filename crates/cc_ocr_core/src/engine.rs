//! Pluggable pipeline engine.

use crate::cache::{self, ensure_out_dirs, CacheKeyParts};
use crate::preprocess;
use crate::progress::{default_reporter, Reporter, Stage};
use crate::render;
use crate::rollup;
use anyhow::{Context, Result};
use cc_ocr_plugin_api::{
    CachedConversation, ClassifierBackend, LabeledMessage, OcrBackend, PageImage, PipelineResult,
    SituationAdapter,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Runtime options that are not part of the situation profile plugins.
#[derive(Debug, Clone)]
pub struct EngineOpts {
    pub dpi: u32,
    pub force: bool,
    pub keep_tmp: bool,
}

impl Default for EngineOpts {
    fn default() -> Self {
        Self {
            dpi: cc_ocr_plugin_api::DEFAULT_DPI,
            force: false,
            keep_tmp: false,
        }
    }
}

/// Core engine: situation + OCR + classifier plugs.
pub struct Engine {
    pub situation: Arc<dyn SituationAdapter>,
    pub ocr: Arc<dyn OcrBackend>,
    pub classifier: Arc<dyn ClassifierBackend>,
    pub opts: EngineOpts,
    pub reporter: Arc<dyn Reporter>,
}

impl Engine {
    pub fn new(
        situation: Arc<dyn SituationAdapter>,
        ocr: Arc<dyn OcrBackend>,
        classifier: Arc<dyn ClassifierBackend>,
        opts: EngineOpts,
    ) -> Self {
        Self::with_reporter(situation, ocr, classifier, opts, default_reporter(false))
    }

    pub fn with_reporter(
        situation: Arc<dyn SituationAdapter>,
        ocr: Arc<dyn OcrBackend>,
        classifier: Arc<dyn ClassifierBackend>,
        opts: EngineOpts,
        reporter: Arc<dyn Reporter>,
    ) -> Self {
        Self {
            situation,
            ocr,
            classifier,
            opts,
            reporter,
        }
    }

    fn supported_input(source: &Path) -> Result<()> {
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "pdf" | "png" | "jpg" | "jpeg" | "webp" => Ok(()),
            other => anyhow::bail!("unsupported input extension: .{other}"),
        }
    }

    /// Stages 1–4: render → preprocess → OCR → structure → JSON cache.
    pub fn run_ocr(&self, source: &Path) -> Result<(CachedConversation, PathBuf)> {
        Self::supported_input(source)?;
        let (cache_dir, _labels_dir, tmp_root) = ensure_out_dirs()?;
        let key = cache::cache_key(CacheKeyParts {
            source,
            dpi: self.opts.dpi,
            situation_fingerprint: self.situation.cache_fingerprint(),
            ocr_id: self.ocr.id(),
            ocr_version: self.ocr.version(),
        })?;

        if !self.opts.force {
            if let Some(hit) = cache::load(&cache_dir, &key)? {
                let path = cache::cache_path(&cache_dir, &key);
                self.reporter.info(
                    Stage::Cache,
                    &format!("hit → {} (skip render/ocr)", path.display()),
                );
                return Ok((hit, path));
            }
        }

        let job_tmp = tmp_root.join(&key);
        if job_tmp.exists() {
            let _ = fs::remove_dir_all(&job_tmp);
        }
        fs::create_dir_all(&job_tmp)?;

        // --- render ---
        let mut render_tick = self.reporter.begin_spinner(
            Stage::Render,
            &format!(
                "{} @ {} DPI ({}+{})",
                source.display(),
                self.opts.dpi,
                self.situation.id(),
                self.ocr.id()
            ),
        );
        let pages = render::render_pages(source, &job_tmp, self.opts.dpi)?;
        render_tick.finish(&format!("{} page(s)", pages.len()));

        // --- preprocess ---
        let prep_cfg = self.situation.preprocess_config();
        let mut prepared: Vec<PageImage> = Vec::with_capacity(pages.len());
        let mut prep_tick = self.reporter.begin_bar(
            Stage::Preprocess,
            pages.len() as u64,
            "crop / enhance",
        );
        for page in &pages {
            if page.width >= 32 && page.height >= 32 {
                prepared.push(preprocess::preprocess_page(page, &prep_cfg)?);
            }
            prep_tick.inc(1);
        }
        prep_tick.finish(&format!("{} page(s) ready", prepared.len()));

        // --- ocr (one page at a time so the bar advances; OCR dominates runtime) ---
        let mut lines = Vec::new();
        let mut ocr_tick = self.reporter.begin_bar(
            Stage::Ocr,
            prepared.len() as u64,
            &format!("tesseract · {}", self.ocr.id()),
        );
        for page in &prepared {
            ocr_tick.set_message(&format!("page {}", page.page));
            let mut page_lines = self.ocr.ocr_pages(std::slice::from_ref(page))?;
            lines.append(&mut page_lines);
            ocr_tick.inc(1);
        }
        ocr_tick.finish(&format!("{} line(s)", lines.len()));

        // --- structure ---
        self.reporter
            .info(Stage::Structure, "grouping lines into messages…");
        let messages = self.situation.structure(&lines);
        self.reporter.info(
            Stage::Structure,
            &format!("{} message(s)", messages.len()),
        );

        let conv = CachedConversation {
            cache_key: key.clone(),
            source: source.to_path_buf(),
            dpi: self.opts.dpi,
            situation_id: self.situation.id().to_string(),
            situation_fingerprint: self.situation.cache_fingerprint().to_string(),
            ocr_backend: self.ocr.id().to_string(),
            ocr_version: self.ocr.version().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            pages: pages.len() as u32,
            lines,
            messages,
        };

        let path = cache::save(&cache_dir, &conv)?;
        self.reporter
            .info(Stage::Write, &format!("cache → {}", path.display()));

        if !self.opts.keep_tmp {
            render::cleanup_pages(&pages);
            render::cleanup_pages(&prepared);
            let _ = fs::remove_dir_all(&job_tmp);
        }

        Ok((conv, path))
    }

    fn classify_messages(
        &self,
        messages: &[cc_ocr_plugin_api::Message],
    ) -> Result<Vec<LabeledMessage>> {
        let mut labeled = Vec::with_capacity(messages.len());
        let mut need_model: Vec<(usize, &cc_ocr_plugin_api::Message)> = Vec::new();

        for (i, msg) in messages.iter().enumerate() {
            if let Some(lm) = self.situation.rule_label(msg) {
                labeled.push((i, lm));
            } else {
                need_model.push((i, msg));
            }
        }

        let ruled = labeled.len();
        if !need_model.is_empty() {
            let texts: Vec<String> = need_model.iter().map(|(_, m)| m.text.clone()).collect();
            let mut tick = self.reporter.begin_spinner(
                Stage::Classify,
                &format!(
                    "{} via model, {} rule-labeled ({})",
                    texts.len(),
                    ruled,
                    self.classifier.id()
                ),
            );
            let model_out = self.classifier.classify(&texts)?;
            if model_out.len() != need_model.len() {
                anyhow::bail!(
                    "classifier returned {} results for {} texts",
                    model_out.len(),
                    need_model.len()
                );
            }
            for ((i, msg), mut item) in need_model.into_iter().zip(model_out) {
                if item.speaker.is_none() {
                    item.speaker = Some(msg.speaker.clone());
                }
                if item.time.is_none() {
                    item.time = msg.time.clone();
                }
                labeled.push((i, item));
            }
            tick.finish(&format!("{} labeled", labeled.len()));
        } else {
            self.reporter.info(
                Stage::Classify,
                &format!("all {} message(s) rule-labeled", ruled),
            );
        }

        labeled.sort_by_key(|(i, _)| *i);
        Ok(labeled.into_iter().map(|(_, lm)| lm).collect())
    }

    /// Classify a cached conversation JSON (or media via OCR first).
    pub fn run_classify(&self, input: &Path) -> Result<(PipelineResult, PathBuf)> {
        let (cache_dir, labels_dir, _tmp) = ensure_out_dirs()?;

        let conv = if input
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
        {
            self.reporter.info(
                Stage::Cache,
                &format!("load conversation JSON {}", input.display()),
            );
            let raw =
                fs::read_to_string(input).with_context(|| format!("read {}", input.display()))?;
            serde_json::from_str(&raw).context("parse cached conversation JSON")?
        } else {
            self.run_ocr(input)?.0
        };

        let labeled = self.classify_messages(&conv.messages)?;
        let source_str = conv.source.to_string_lossy().to_string();
        self.reporter.info(Stage::Rollup, "aggregating intents / risk…");
        let policy = self.situation.risk_policy();
        let roll = rollup::rollup(&source_str, &labeled, &policy);

        let result = PipelineResult {
            conversation: conv.clone(),
            classifier_id: self.classifier.id().to_string(),
            labeled,
            rollup: roll,
        };

        let out = cache::labels_path(&labels_dir, &conv.cache_key);
        cache::save_json(&out, &result)?;
        let _ = cache::save(&cache_dir, &conv);
        self.reporter
            .info(Stage::Write, &format!("labels → {}", out.display()));
        self.reporter.info(
            Stage::Done,
            &format!(
                "intents={:?} risk={:?}",
                result.rollup.intent_histogram, result.rollup.risk_flags
            ),
        );
        Ok((result, out))
    }

    /// Full pipeline: OCR + classify + rollup.
    pub fn run_full(&self, source: &Path) -> Result<(PipelineResult, PathBuf)> {
        let (conv, _) = self.run_ocr(source)?;
        let labeled = self.classify_messages(&conv.messages)?;
        let source_str = conv.source.to_string_lossy().to_string();
        self.reporter.info(Stage::Rollup, "aggregating intents / risk…");
        let policy = self.situation.risk_policy();
        let roll = rollup::rollup(&source_str, &labeled, &policy);
        let result = PipelineResult {
            conversation: conv.clone(),
            classifier_id: self.classifier.id().to_string(),
            labeled,
            rollup: roll,
        };
        let (_cache_dir, labels_dir, _) = ensure_out_dirs()?;
        let out = cache::labels_path(&labels_dir, &conv.cache_key);
        cache::save_json(&out, &result)?;
        self.reporter
            .info(Stage::Write, &format!("labels → {}", out.display()));
        self.reporter.info(
            Stage::Done,
            &format!(
                "intents={:?} risk={:?}",
                result.rollup.intent_histogram, result.rollup.risk_flags
            ),
        );
        Ok((result, out))
    }

    /// Recompute rollup from labels JSON using this engine's situation risk policy.
    pub fn run_rollup(&self, labels_json: &Path) -> Result<cc_ocr_plugin_api::ConversationRollup> {
        self.reporter.info(
            Stage::Rollup,
            &format!("recompute from {}", labels_json.display()),
        );
        let raw = fs::read_to_string(labels_json)
            .with_context(|| format!("read {}", labels_json.display()))?;
        let result: PipelineResult = serde_json::from_str(&raw).context("parse labels JSON")?;
        let source = result.rollup.source.clone();
        let policy = self.situation.risk_policy();
        Ok(rollup::rollup(&source, &result.labeled, &policy))
    }
}
