//! Generic chat SituationAdapter — no platform crop, weak speaker heuristics.

use cc_ocr_plugin_api::{
    LabeledMessage, Message, OcrLine, PreprocessConfig, RiskPolicy, SituationAdapter,
};
use regex::Regex;
use std::sync::OnceLock;

const FINGERPRINT: &str = "generic-v1";

#[derive(Debug, Clone)]
pub struct GenericAdapter {
    /// Names treated as "self" / local user (default: me, you).
    pub self_aliases: Vec<String>,
}

impl Default for GenericAdapter {
    fn default() -> Self {
        Self {
            self_aliases: vec!["me".into(), "you".into()],
        }
    }
}

impl GenericAdapter {
    pub fn new(self_aliases: Vec<String>) -> Self {
        let self_aliases = if self_aliases.is_empty() {
            Self::default().self_aliases
        } else {
            self_aliases
        };
        Self { self_aliases }
    }
}

impl SituationAdapter for GenericAdapter {
    fn id(&self) -> &str {
        "generic"
    }

    fn preprocess_config(&self) -> PreprocessConfig {
        PreprocessConfig::no_crop()
    }

    fn structure(&self, lines: &[OcrLine]) -> Vec<Message> {
        structure_messages(lines, &self.self_aliases)
    }

    fn rule_label(&self, msg: &Message) -> Option<LabeledMessage> {
        if msg.speaker == "system" {
            Some(LabeledMessage {
                text: msg.text.clone(),
                intent: "system".into(),
                confidence: 1.0,
                speaker: Some(msg.speaker.clone()),
                time: msg.time.clone(),
            })
        } else {
            None
        }
    }

    fn risk_policy(&self) -> RiskPolicy {
        RiskPolicy::new(
            ["dispute", "refund"],
            ["dispute", "refund", "scam", "scammed", "legal", "chargeback"],
        )
    }

    fn cache_fingerprint(&self) -> &str {
        FINGERPRINT
    }
}

fn re_time() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2}(?:,\s*\d{4})?,?\s+\d{1,2}:\d{2}\s*(?:AM|PM)\b|\b\d{1,2}:\d{2}\s*(?:AM|PM)\b",
        )
        .expect("time regex")
    })
}

fn re_speaker_header() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^(?:@\s*)?([A-Za-z][A-Za-z0-9._-]{1,})\s+((?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2}(?:,\s*\d{4})?,?(?:\s+\d{1,2}:\d{2}\s*(?:AM|PM))?|\d{1,2}:\d{2}\s*(?:AM|PM))$",
        )
        .expect("generic speaker header")
    })
}

fn normalize_speaker(raw: &str, self_aliases: &[String]) -> String {
    let s = raw.trim();
    if self_aliases
        .iter()
        .any(|a| a.eq_ignore_ascii_case(s))
    {
        "self".into()
    } else if s.eq_ignore_ascii_case("system") {
        "system".into()
    } else {
        "other".into()
    }
}

fn structure_messages(lines: &[OcrLine], self_aliases: &[String]) -> Vec<Message> {
    let mut messages = Vec::new();
    let mut cur_speaker: Option<String> = None;
    let mut cur_time: Option<String> = None;
    let mut cur_body: Vec<String> = Vec::new();

    let flush = |speaker: &mut Option<String>,
                 time: &mut Option<String>,
                 body: &mut Vec<String>,
                 out: &mut Vec<Message>| {
        let text = body.join("\n").trim().to_string();
        body.clear();
        if text.is_empty() {
            return;
        }
        out.push(Message {
            speaker: speaker.clone().unwrap_or_else(|| "unknown".into()),
            time: time.clone(),
            text,
        });
    };

    for line in lines {
        let text = line.text.trim();
        if text.is_empty() || text.len() <= 1 {
            continue;
        }

        if let Some(caps) = re_speaker_header().captures(text) {
            flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            cur_speaker = Some(normalize_speaker(name, self_aliases));
            cur_time = caps.get(2).map(|m| m.as_str().to_string());
            continue;
        }

        // Strip trailing timestamp if present; otherwise keep as body.
        let body_text = if let Some(m) = re_time().find(text) {
            if m.end() == text.len() && m.start() > 0 {
                text[..m.start()].trim().to_string()
            } else if m.start() == 0 && m.end() == text.len() {
                if cur_time.is_none() {
                    cur_time = Some(text.to_string());
                }
                continue;
            } else {
                text.to_string()
            }
        } else {
            text.to_string()
        };

        if body_text.is_empty() {
            continue;
        }
        if cur_speaker.is_none() {
            cur_speaker.replace("unknown".into());
        }
        cur_body.push(body_text);
    }

    flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
    messages
}
