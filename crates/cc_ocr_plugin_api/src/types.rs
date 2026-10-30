//! Shared pipeline types (serde JSON contracts).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const DEFAULT_DPI: u32 = 150;

/// One OCR line (optionally with a bounding box).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrLine {
    pub text: String,
    pub conf: f32,
    /// `[x0, y0, x1, y1]` in page-image pixels when available.
    pub bbox: Option<[u32; 4]>,
    pub page: u32,
}

/// Structured chat message after line grouping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub speaker: String,
    pub time: Option<String>,
    pub text: String,
}

/// Message with intent label from the classifier (or rules).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledMessage {
    pub text: String,
    pub intent: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/// Conversation-level aggregate for search / review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRollup {
    pub source: String,
    pub messages: Vec<LabeledMessage>,
    pub intent_histogram: BTreeMap<String, u32>,
    pub risk_flags: Vec<String>,
}

/// Cached structured OCR output for one input file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedConversation {
    pub cache_key: String,
    pub source: PathBuf,
    pub dpi: u32,
    pub situation_id: String,
    pub situation_fingerprint: String,
    pub ocr_backend: String,
    pub ocr_version: String,
    pub created_at: String,
    pub pages: u32,
    pub lines: Vec<OcrLine>,
    pub messages: Vec<Message>,
}

/// Full pipeline result (OCR cache + labels + rollup).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub conversation: CachedConversation,
    pub classifier_id: String,
    pub labeled: Vec<LabeledMessage>,
    pub rollup: ConversationRollup,
}
