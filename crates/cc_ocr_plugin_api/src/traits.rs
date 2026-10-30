//! Plugin traits: situation, OCR, classifier.

use crate::page::PageImage;
use crate::preprocess::PreprocessConfig;
use crate::risk::RiskPolicy;
use crate::types::{LabeledMessage, Message, OcrLine};
use anyhow::Result;

/// Platform / layout adapter: crop, chrome filter, message structure, risk.
pub trait SituationAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn preprocess_config(&self) -> PreprocessConfig;
    fn structure(&self, lines: &[OcrLine]) -> Vec<Message>;
    fn rule_label(&self, msg: &Message) -> Option<LabeledMessage>;
    fn risk_policy(&self) -> RiskPolicy;
    /// Bump to invalidate OCR cache when heuristics change.
    fn cache_fingerprint(&self) -> &str;
}

/// Character recognition backend.
pub trait OcrBackend: Send + Sync {
    fn id(&self) -> &str;
    fn ocr_pages(&self, pages: &[PageImage]) -> Result<Vec<OcrLine>>;
    fn version(&self) -> &str;
}

/// Text intent classifier backend.
pub trait ClassifierBackend: Send + Sync {
    fn id(&self) -> &str;
    fn classify(&self, texts: &[String]) -> Result<Vec<LabeledMessage>>;
}
