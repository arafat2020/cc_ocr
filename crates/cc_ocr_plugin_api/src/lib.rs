//! Plugin API: shared types and traits for the cc_ocr engine.

pub mod page;
pub mod preprocess;
pub mod risk;
pub mod traits;
pub mod types;

pub use page::PageImage;
pub use preprocess::PreprocessConfig;
pub use risk::RiskPolicy;
pub use traits::{ClassifierBackend, OcrBackend, SituationAdapter};
pub use types::*;
