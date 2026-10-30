//! Content-hash JSON cache for structured OCR output.

use anyhow::{Context, Result};
use cc_ocr_plugin_api::CachedConversation;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Inputs that must match for a cache hit.
#[derive(Debug, Clone)]
pub struct CacheKeyParts<'a> {
    pub source: &'a Path,
    pub dpi: u32,
    pub situation_fingerprint: &'a str,
    pub ocr_id: &'a str,
    pub ocr_version: &'a str,
}

/// Workspace-relative output roots (resolved from CWD).
pub fn out_dirs() -> (PathBuf, PathBuf, PathBuf) {
    let root = PathBuf::from("out");
    (root.join("cache"), root.join("labels"), root.join("tmp"))
}

/// Ensure `out/cache`, `out/labels`, `out/tmp` exist.
pub fn ensure_out_dirs() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let (cache, labels, tmp) = out_dirs();
    for d in [&cache, &labels, &tmp] {
        fs::create_dir_all(d).with_context(|| format!("create {}", d.display()))?;
    }
    Ok((cache, labels, tmp))
}

/// Hash of path + size + mtime + dpi + situation + OCR backend fingerprint.
pub fn cache_key(parts: CacheKeyParts<'_>) -> Result<String> {
    let meta = fs::metadata(parts.source)
        .with_context(|| format!("stat {}", parts.source.display()))?;
    let abs = fs::canonicalize(parts.source).unwrap_or_else(|_| parts.source.to_path_buf());
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut hasher = Sha256::new();
    hasher.update(abs.to_string_lossy().as_bytes());
    hasher.update(b"|");
    hasher.update(meta.len().to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(mtime.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(parts.dpi.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(parts.situation_fingerprint.as_bytes());
    hasher.update(b"|");
    hasher.update(parts.ocr_id.as_bytes());
    hasher.update(b"|");
    hasher.update(parts.ocr_version.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

pub fn cache_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir.join(format!("{key}.json"))
}

pub fn labels_path(labels_dir: &Path, key: &str) -> PathBuf {
    labels_dir.join(format!("{key}.json"))
}

pub fn load(cache_dir: &Path, key: &str) -> Result<Option<CachedConversation>> {
    let path = cache_path(cache_dir, key);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let parsed: CachedConversation =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(parsed))
}

pub fn save(cache_dir: &Path, conv: &CachedConversation) -> Result<PathBuf> {
    let path = cache_path(cache_dir, &conv.cache_key);
    let raw = serde_json::to_string_pretty(conv)?;
    fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub fn save_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
