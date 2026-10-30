//! Situation TOML profiles (plug-and-play config).

use anyhow::{Context, Result};
use cc_ocr_plugin_api::DEFAULT_DPI;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Top-level situation profile loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct SituationProfile {
    pub id: String,
    pub situation: String,
    pub ocr: String,
    pub classifier: String,
    #[serde(default)]
    pub params: ProfileParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileParams {
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    #[serde(default = "default_python")]
    pub python: String,
    #[serde(default)]
    pub fiverr: Option<FiverrParams>,
    #[serde(default)]
    pub generic: Option<GenericParams>,
}

impl Default for ProfileParams {
    fn default() -> Self {
        Self {
            dpi: DEFAULT_DPI,
            python: default_python(),
            fiverr: None,
            generic: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FiverrParams {
    #[serde(default = "default_crop_left")]
    pub crop_left_frac: f32,
    #[serde(default = "default_crop_right")]
    pub crop_right_frac: f32,
    #[serde(default = "default_min_width")]
    pub min_width_for_crop: u32,
}

impl Default for FiverrParams {
    fn default() -> Self {
        Self {
            crop_left_frac: default_crop_left(),
            crop_right_frac: default_crop_right(),
            min_width_for_crop: default_min_width(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GenericParams {
    #[serde(default)]
    pub self_aliases: Vec<String>,
}

fn default_dpi() -> u32 {
    DEFAULT_DPI
}
fn default_python() -> String {
    "python3".into()
}
fn default_crop_left() -> f32 {
    0.22
}
fn default_crop_right() -> f32 {
    0.72
}
fn default_min_width() -> u32 {
    1200
}

/// Load a situation profile from a TOML file.
pub fn load_profile(path: &Path) -> Result<SituationProfile> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let profile: SituationProfile =
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    Ok(profile)
}

/// Default profile path when `--situation` is omitted.
pub fn default_profile_path() -> std::path::PathBuf {
    // Prefer CWD, then walk up, then relative to this crate.
    let candidates = [
        std::path::PathBuf::from("situations/fiverr_inbox.toml"),
        std::env::current_dir()
            .ok()
            .map(|c| c.join("situations/fiverr_inbox.toml"))
            .unwrap_or_default(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../situations/fiverr_inbox.toml"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    std::path::PathBuf::from("situations/fiverr_inbox.toml")
}
