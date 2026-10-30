//! Python TensorFlow classifier sidecar backend.

use anyhow::{bail, Context, Result};
use cc_ocr_plugin_api::{ClassifierBackend, LabeledMessage};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct TfSidecar {
    pub python_bin: String,
    pub script_path: Option<PathBuf>,
}

impl TfSidecar {
    pub fn new(python_bin: impl Into<String>) -> Self {
        Self {
            python_bin: python_bin.into(),
            script_path: None,
        }
    }
}

impl Default for TfSidecar {
    fn default() -> Self {
        Self::new("python3")
    }
}

impl ClassifierBackend for TfSidecar {
    fn id(&self) -> &str {
        "tf_sidecar"
    }

    fn classify(&self, texts: &[String]) -> Result<Vec<LabeledMessage>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let script = self
            .script_path
            .clone()
            .or_else(find_serve_script)
            .context("locate python/classifier/serve.py")?;
        let payload = serde_json::json!({ "texts": texts });

        let mut child = Command::new(&self.python_bin)
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn {} {}", self.python_bin, script.display()))?;

        {
            let stdin = child.stdin.as_mut().context("sidecar stdin")?;
            stdin.write_all(payload.to_string().as_bytes())?;
        }

        let output = child.wait_with_output().context("wait sidecar")?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("classifier sidecar failed: {err}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: ServeResponse =
            serde_json::from_str(stdout.trim()).context("parse classifier JSON")?;
        Ok(parsed
            .results
            .into_iter()
            .map(|item| LabeledMessage {
                text: item.text,
                intent: item.intent,
                confidence: item.confidence,
                speaker: None,
                time: None,
            })
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct ServeResponse {
    results: Vec<ServeItem>,
}

#[derive(Debug, Deserialize)]
struct ServeItem {
    text: String,
    intent: String,
    confidence: f32,
}

fn find_serve_script() -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = cur.join("python/classifier/serve.py");
        if candidate.exists() {
            return Some(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    let alt = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python/classifier/serve.py");
    if alt.exists() {
        Some(alt)
    } else {
        None
    }
}
