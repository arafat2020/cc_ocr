//! Grayscale, contrast, and optional chat-pane crop (config from situation).

use anyhow::{Context, Result};
use cc_ocr_plugin_api::{PageImage, PreprocessConfig};
use image::imageops;
use image::{GrayImage, ImageBuffer, Luma, RgbImage};
use std::path::PathBuf;

/// Preprocess a page image (writes sibling `*_prep.png`).
pub fn preprocess_page(page: &PageImage, cfg: &PreprocessConfig) -> Result<PageImage> {
    let img = image::open(&page.path)
        .with_context(|| format!("open {}", page.path.display()))?
        .to_rgb8();

    let cropped = crop_chat_pane(&img, cfg);
    let enhanced = if cfg.grayscale {
        let gray = to_gray_contrast(&cropped, cfg.contrast);
        gray_to_rgb(&gray)
    } else {
        adjust_contrast_rgb(&cropped, cfg.contrast)
    };

    let out_path = prep_path(&page.path);
    enhanced
        .save(&out_path)
        .with_context(|| format!("save {}", out_path.display()))?;

    Ok(PageImage {
        page: page.page,
        path: out_path,
        width: enhanced.width(),
        height: enhanced.height(),
    })
}

fn prep_path(src: &std::path::Path) -> PathBuf {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
    src.with_file_name(format!("{stem}_prep.png"))
}

fn crop_chat_pane(img: &RgbImage, cfg: &PreprocessConfig) -> RgbImage {
    let (w, h) = img.dimensions();
    if w < cfg.min_width_for_crop {
        return img.clone();
    }
    if cfg.crop_left_frac <= 0.0 && cfg.crop_right_frac >= 1.0 {
        return img.clone();
    }
    let x0 = ((w as f32) * cfg.crop_left_frac).round() as u32;
    let x1 = ((w as f32) * cfg.crop_right_frac).round() as u32;
    let x1 = x1.min(w).max(x0 + 1);
    imageops::crop_imm(img, x0, 0, x1 - x0, h).to_image()
}

fn to_gray_contrast(img: &RgbImage, contrast: f32) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut out: GrayImage = ImageBuffer::new(w, h);
    for (x, y, pixel) in img.enumerate_pixels() {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        let mut yv = 0.299 * r + 0.587 * g + 0.114 * b;
        yv = ((yv - 128.0) * contrast + 128.0).clamp(0.0, 255.0);
        out.put_pixel(x, y, Luma([yv as u8]));
    }
    out
}

fn gray_to_rgb(gray: &GrayImage) -> RgbImage {
    let (w, h) = gray.dimensions();
    let mut out = RgbImage::new(w, h);
    for (x, y, p) in gray.enumerate_pixels() {
        let v = p[0];
        out.put_pixel(x, y, image::Rgb([v, v, v]));
    }
    out
}

fn adjust_contrast_rgb(img: &RgbImage, contrast: f32) -> RgbImage {
    let (w, h) = img.dimensions();
    let mut out = RgbImage::new(w, h);
    for (x, y, pixel) in img.enumerate_pixels() {
        let mut rgb = [0u8; 3];
        for i in 0..3 {
            let v = pixel[i] as f32;
            rgb[i] = ((v - 128.0) * contrast + 128.0).clamp(0.0, 255.0) as u8;
        }
        out.put_pixel(x, y, image::Rgb(rgb));
    }
    out
}
