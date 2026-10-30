//! Rendered page image handle shared across core and OCR plugins.

use std::path::PathBuf;

/// One rendered (or preprocessed) page image on disk.
#[derive(Debug, Clone)]
pub struct PageImage {
    pub page: u32,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}
