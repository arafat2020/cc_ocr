//! PDF/PNG → page images (page-by-page, moderate DPI).

use anyhow::{bail, Context, Result};
use cc_ocr_plugin_api::PageImage;
use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Render `source` (PDF or PNG) into `tmp_dir` at `dpi`.
pub fn render_pages(source: &Path, tmp_dir: &Path, dpi: u32) -> Result<Vec<PageImage>> {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    fs::create_dir_all(tmp_dir)?;

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => render_raster(source, tmp_dir),
        "pdf" => render_pdf_pymupdf(source, tmp_dir, dpi),
        other => bail!("unsupported input extension: .{other}"),
    }
}

fn render_raster(source: &Path, tmp_dir: &Path) -> Result<Vec<PageImage>> {
    let img = image::open(source).with_context(|| format!("open image {}", source.display()))?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let out = tmp_dir.join("page_0001.png");
    rgb.save(&out)
        .with_context(|| format!("save {}", out.display()))?;
    Ok(vec![PageImage {
        page: 1,
        path: out,
        width: w,
        height: h,
    }])
}

fn render_pdf_pymupdf(source: &Path, tmp_dir: &Path, dpi: u32) -> Result<Vec<PageImage>> {
    let helper = workspace_root()
        .map(|r| r.join("python/render_pages.py"))
        .filter(|p| p.exists());

    let status = if let Some(script) = helper {
        Command::new("python3")
            .arg(&script)
            .arg(source)
            .arg(tmp_dir)
            .arg(dpi.to_string())
            .status()
            .context("run python/render_pages.py")?
    } else {
        let code = format!(
            r#"
import sys
from pathlib import Path
import pymupdf
src, out, dpi = Path(sys.argv[1]), Path(sys.argv[2]), float(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)
doc = pymupdf.open(src)
zoom = dpi / 72.0
mat = pymupdf.Matrix(zoom, zoom)
for i, page in enumerate(doc):
    pix = page.get_pixmap(matrix=mat, alpha=False)
    pix.save(out / f"page_{{i+1:04d}}.png")
print(doc.page_count)
"#
        );
        Command::new("python3")
            .arg("-c")
            .arg(code)
            .arg(source)
            .arg(tmp_dir)
            .arg(dpi.to_string())
            .status()
            .context("run pymupdf render inline")?
    };

    if !status.success() {
        bail!("PDF render failed (install pymupdf: pip install pymupdf). status={status}");
    }

    collect_page_pngs(tmp_dir)
}

fn collect_page_pngs(tmp_dir: &Path) -> Result<Vec<PageImage>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(tmp_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
                && p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("page_"))
                    .unwrap_or(false)
        })
        .collect();
    paths.sort();

    let mut pages = Vec::with_capacity(paths.len());
    for (i, path) in paths.into_iter().enumerate() {
        let img = image::open(&path).with_context(|| format!("open {}", path.display()))?;
        let (w, h) = img.dimensions();
        pages.push(PageImage {
            page: (i as u32) + 1,
            path,
            width: w,
            height: h,
        });
    }
    if pages.is_empty() {
        bail!("no page_*.png produced in {}", tmp_dir.display());
    }
    Ok(pages)
}

fn workspace_root() -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    for _ in 0..6 {
        if cur.join("Cargo.toml").exists() && cur.join("python").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

/// Remove rendered page images (best-effort).
pub fn cleanup_pages(pages: &[PageImage]) {
    for p in pages {
        let _ = fs::remove_file(&p.path);
    }
}
