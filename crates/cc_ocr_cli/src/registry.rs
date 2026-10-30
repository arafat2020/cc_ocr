//! Compile-time plugin registry: profile IDs → concrete plugins.

use anyhow::{bail, Result};
use cc_ocr_core::profile::SituationProfile;
use cc_ocr_core::{Engine, EngineOpts, Reporter};
use cc_ocr_plugin_api::{ClassifierBackend, OcrBackend, SituationAdapter};
use cc_ocr_plugin_fiverr::FiverrAdapter;
use cc_ocr_plugin_generic::GenericAdapter;
use cc_ocr_plugin_tesseract::TesseractOcr;
use cc_ocr_plugin_tf_sidecar::TfSidecar;
use std::sync::Arc;

/// Build an [`Engine`] from a situation profile + runtime CLI opts.
pub fn build_engine(
    profile: &SituationProfile,
    mut opts: EngineOpts,
    reporter: Arc<dyn Reporter>,
) -> Result<Engine> {
    // Profile DPI wins unless CLI already overrode via opts (caller sets dpi).
    if opts.dpi == cc_ocr_plugin_api::DEFAULT_DPI {
        opts.dpi = profile.params.dpi;
    }

    let situation: Arc<dyn SituationAdapter> = match profile.situation.as_str() {
        "fiverr" => {
            let p = profile.params.fiverr.clone().unwrap_or_default();
            Arc::new(FiverrAdapter::new(
                p.crop_left_frac,
                p.crop_right_frac,
                p.min_width_for_crop,
            ))
        }
        "generic" => {
            let aliases = profile
                .params
                .generic
                .as_ref()
                .map(|g| g.self_aliases.clone())
                .unwrap_or_default();
            Arc::new(GenericAdapter::new(aliases))
        }
        other => bail!("unknown situation plugin id: {other} (register in registry.rs)"),
    };

    let ocr: Arc<dyn OcrBackend> = match profile.ocr.as_str() {
        "tesseract" => Arc::new(TesseractOcr),
        other => bail!("unknown ocr plugin id: {other} (register in registry.rs)"),
    };

    let classifier: Arc<dyn ClassifierBackend> = match profile.classifier.as_str() {
        "tf_sidecar" => Arc::new(TfSidecar::new(profile.params.python.clone())),
        other => bail!("unknown classifier plugin id: {other} (register in registry.rs)"),
    };

    Ok(Engine::with_reporter(
        situation, ocr, classifier, opts, reporter,
    ))
}
