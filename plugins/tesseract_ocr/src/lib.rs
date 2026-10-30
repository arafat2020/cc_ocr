//! Tesseract OCR backend (system CLI, TSV + vertical tiling).

use anyhow::{bail, Context, Result};
use cc_ocr_plugin_api::{OcrBackend, OcrLine, PageImage};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = "1";
const MAX_TILE_HEIGHT: u32 = 3500;
const TILE_OVERLAP: u32 = 80;

#[derive(Debug, Default, Clone)]
pub struct TesseractOcr;

impl OcrBackend for TesseractOcr {
    fn id(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> &str {
        VERSION
    }

    fn ocr_pages(&self, pages: &[PageImage]) -> Result<Vec<OcrLine>> {
        ensure_tesseract()?;
        let mut all = Vec::new();
        for page in pages {
            all.append(&mut ocr_page(page)?);
        }
        Ok(all)
    }
}

fn ensure_tesseract() -> Result<()> {
    if which::which("tesseract").is_err() {
        bail!("tesseract not found on PATH; install Tesseract OCR");
    }
    Ok(())
}

fn ocr_page(page: &PageImage) -> Result<Vec<OcrLine>> {
    if page.height <= MAX_TILE_HEIGHT {
        return ocr_image_file(&page.path, page.page, 0);
    }

    let img = image::open(&page.path)
        .with_context(|| format!("open {}", page.path.display()))?
        .to_rgb8();
    let (w, h) = (img.width(), img.height());
    let mut y: u32 = 0;
    let mut all = Vec::new();
    let mut tile_idx = 0u32;
    while y < h {
        let y1 = (y + MAX_TILE_HEIGHT).min(h);
        let tile = image::imageops::crop_imm(&img, 0, y, w, y1 - y).to_image();
        let tile_path = page.path.with_file_name(format!(
            "{}_tile{tile_idx}.png",
            page.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("page")
        ));
        tile.save(&tile_path)
            .with_context(|| format!("save tile {}", tile_path.display()))?;
        all.append(&mut ocr_image_file(&tile_path, page.page, y)?);
        let _ = fs::remove_file(&tile_path);
        tile_idx += 1;
        if y1 >= h {
            break;
        }
        y = y1.saturating_sub(TILE_OVERLAP);
    }
    Ok(all)
}

fn ocr_image_file(path: &Path, page: u32, y_offset: u32) -> Result<Vec<OcrLine>> {
    let out_base = path.with_extension("");
    let status = Command::new("tesseract")
        .arg(path)
        .arg(&out_base)
        .arg("--psm")
        .arg("6")
        .arg("tsv")
        .status()
        .context("spawn tesseract")?;

    if !status.success() {
        bail!("tesseract failed on {} ({status})", path.display());
    }

    let tsv_path = PathBuf::from(format!("{}.tsv", out_base.display()));
    let mut lines = parse_tsv(&tsv_path, page)?;
    if y_offset > 0 {
        for line in &mut lines {
            if let Some(b) = line.bbox.as_mut() {
                b[1] = b[1].saturating_add(y_offset);
                b[3] = b[3].saturating_add(y_offset);
            }
        }
    }
    let _ = fs::remove_file(&tsv_path);
    Ok(lines)
}

fn parse_tsv(path: &Path, page: u32) -> Result<Vec<OcrLine>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut lines = Vec::new();

    #[derive(Default)]
    struct Acc {
        texts: Vec<String>,
        confs: Vec<f32>,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        init: bool,
    }

    let mut map: std::collections::BTreeMap<(i32, i32, i32), Acc> =
        std::collections::BTreeMap::new();

    for (idx, row) in raw.lines().enumerate() {
        if idx == 0 {
            continue;
        }
        let cols: Vec<&str> = row.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        let level: i32 = cols[0].parse().unwrap_or(0);
        let block: i32 = cols[2].parse().unwrap_or(0);
        let par: i32 = cols[3].parse().unwrap_or(0);
        let line: i32 = cols[4].parse().unwrap_or(0);
        let left: u32 = cols[6].parse().unwrap_or(0);
        let top: u32 = cols[7].parse().unwrap_or(0);
        let width: u32 = cols[8].parse().unwrap_or(0);
        let height: u32 = cols[9].parse().unwrap_or(0);
        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        let text = cols[11..].join("\t").trim().to_string();

        if level == 5 {
            if text.is_empty() {
                continue;
            }
            let acc = map.entry((block, par, line)).or_default();
            if !acc.init {
                acc.x0 = left;
                acc.y0 = top;
                acc.x1 = left + width;
                acc.y1 = top + height;
                acc.init = true;
            } else {
                acc.x0 = acc.x0.min(left);
                acc.y0 = acc.y0.min(top);
                acc.x1 = acc.x1.max(left + width);
                acc.y1 = acc.y1.max(top + height);
            }
            acc.texts.push(text);
            if conf >= 0.0 {
                acc.confs.push(conf);
            }
        } else if level == 4 && !text.is_empty() {
            lines.push(OcrLine {
                text,
                conf,
                bbox: Some([left, top, left + width, top + height]),
                page,
            });
        }
    }

    for (_, acc) in map {
        if acc.texts.is_empty() {
            continue;
        }
        let conf = if acc.confs.is_empty() {
            -1.0
        } else {
            acc.confs.iter().sum::<f32>() / acc.confs.len() as f32
        };
        lines.push(OcrLine {
            text: acc.texts.join(" "),
            conf,
            bbox: Some([acc.x0, acc.y0, acc.x1, acc.y1]),
            page,
        });
    }

    lines.sort_by(|a, b| {
        let ay = a.bbox.map(|b| b[1]).unwrap_or(0);
        let by = b.bbox.map(|b| b[1]).unwrap_or(0);
        ay.cmp(&by)
            .then_with(|| {
                let ax = a.bbox.map(|b| b[0]).unwrap_or(0);
                let bx = b.bbox.map(|b| b[0]).unwrap_or(0);
                ax.cmp(&bx)
            })
            .then_with(|| a.text.cmp(&b.text))
    });

    Ok(lines)
}
