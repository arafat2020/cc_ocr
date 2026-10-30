//! Fiverr inbox SituationAdapter.

use cc_ocr_plugin_api::{
    LabeledMessage, Message, OcrLine, PreprocessConfig, RiskPolicy, SituationAdapter,
};
use regex::Regex;
use std::sync::OnceLock;

const FINGERPRINT: &str = "fiverr-v1";

#[derive(Debug, Clone)]
pub struct FiverrAdapter {
    pub crop_left_frac: f32,
    pub crop_right_frac: f32,
    pub min_width_for_crop: u32,
}

impl Default for FiverrAdapter {
    fn default() -> Self {
        Self {
            crop_left_frac: 0.22,
            crop_right_frac: 0.72,
            min_width_for_crop: 1200,
        }
    }
}

impl FiverrAdapter {
    pub fn new(crop_left_frac: f32, crop_right_frac: f32, min_width_for_crop: u32) -> Self {
        Self {
            crop_left_frac,
            crop_right_frac,
            min_width_for_crop,
        }
    }
}

impl SituationAdapter for FiverrAdapter {
    fn id(&self) -> &str {
        "fiverr"
    }

    fn preprocess_config(&self) -> PreprocessConfig {
        PreprocessConfig {
            grayscale: true,
            contrast: 1.15,
            min_width_for_crop: self.min_width_for_crop,
            crop_left_frac: self.crop_left_frac,
            crop_right_frac: self.crop_right_frac,
        }
    }

    fn structure(&self, lines: &[OcrLine]) -> Vec<Message> {
        structure_messages(lines)
    }

    fn rule_label(&self, msg: &Message) -> Option<LabeledMessage> {
        if msg.speaker == "system" {
            return Some(LabeledMessage {
                text: msg.text.clone(),
                intent: "system".into(),
                confidence: 1.0,
                speaker: Some(msg.speaker.clone()),
                time: msg.time.clone(),
            });
        }
        let lower = msg.text.to_ascii_lowercase();
        if lower.contains("your order was delivered")
            || lower.contains("order delivered")
            || lower.starts_with("delivery")
        {
            return Some(LabeledMessage {
                text: msg.text.clone(),
                intent: "system".into(),
                confidence: 1.0,
                speaker: Some(msg.speaker.clone()),
                time: msg.time.clone(),
            });
        }
        None
    }

    fn risk_policy(&self) -> RiskPolicy {
        RiskPolicy::new(
            ["dispute", "refund"],
            [
                "dispute",
                "refund",
                "scam",
                "scammed",
                "fiverr support",
                "legal",
                "chargeback",
                "lawyer",
                "police",
            ],
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
            r"(?i)^(?:[A-Za-z]?\(?[A-Za-z0-9]\)\s*)?(?:@\s*)?(Me|[A-Za-z][A-Za-z0-9._-]{2,})\s+((?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2}(?:,\s*\d{4})?,?(?:\s+\d{1,2}:\d{2}\s*(?:AM|PM))?)$",
        )
        .expect("speaker header regex")
    })
}

fn re_standalone_time() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\d{1,2}:\d{2}\s*(?:AM|PM)$").expect("standalone time"))
}

fn is_chrome(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    const NOISE: &[&str] = &[
        "dashboard",
        "my business",
        "growth & marketing",
        "analytics",
        "all messages",
        "last seen",
        "on fiverr since",
        "orders with you",
        "translate to english",
        "about ",
        "saved",
        "from canada",
        "from united",
        "banned for now",
        "local time",
        "fiverr.",
        "total (",
        "online",
        "messages saved",
        "view order",
        "offer submitted",
        "custom offer",
    ];
    if NOISE.iter().any(|n| lower.contains(n)) {
        if t.len() > 80 {
            return false;
        }
        return true;
    }
    t.len() <= 2
}

fn normalize_speaker(raw: &str) -> String {
    let s = raw.trim();
    if s.eq_ignore_ascii_case("me") || s.eq_ignore_ascii_case("you") {
        "seller".into()
    } else if s.eq_ignore_ascii_case("delivery")
        || s.eq_ignore_ascii_case("system")
        || s.to_ascii_lowercase().contains("fiverr")
    {
        "system".into()
    } else {
        "buyer".into()
    }
}

fn structure_messages(lines: &[OcrLine]) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();
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
        if text.is_empty() || is_chrome(text) {
            continue;
        }

        if let Some(caps) = re_speaker_header().captures(text) {
            flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            let time = caps.get(2).map(|m| m.as_str().to_string());
            cur_speaker = Some(normalize_speaker(name));
            cur_time = time;
            continue;
        }

        if re_standalone_time().is_match(text) {
            if cur_time.is_none() {
                cur_time = Some(text.to_string());
            }
            continue;
        }

        if text.eq_ignore_ascii_case("delivery")
            || text.eq_ignore_ascii_case("order delivered")
            || text.to_ascii_lowercase().starts_with("your order was")
        {
            flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
            cur_speaker = Some("system".into());
            cur_time = None;
            if !text.eq_ignore_ascii_case("delivery") {
                cur_body.push(text.to_string());
            }
            continue;
        }

        if looks_like_bare_speaker(text) {
            flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
            cur_speaker = Some(normalize_speaker(
                text.trim_start_matches("@ ").trim_start_matches('@'),
            ));
            cur_time = None;
            continue;
        }

        let body_text = strip_trailing_time(text);
        if body_text.is_empty() {
            continue;
        }
        if cur_speaker.is_none() {
            cur_speaker = Some("unknown".into());
        }
        cur_body.push(body_text);
    }

    flush(&mut cur_speaker, &mut cur_time, &mut cur_body, &mut messages);
    messages
}

fn looks_like_bare_speaker(text: &str) -> bool {
    let t = text.trim();
    if t.contains(' ') && !t.starts_with('@') {
        return false;
    }
    let name = t.trim_start_matches('@').trim();
    if name.eq_ignore_ascii_case("me") || name.eq_ignore_ascii_case("you") {
        return true;
    }
    Regex::new(r"^[A-Za-z][A-Za-z0-9._-]{2,24}$")
        .ok()
        .map(|r| r.is_match(name))
        .unwrap_or(false)
}

fn strip_trailing_time(text: &str) -> String {
    if let Some(m) = re_time().find(text) {
        if m.end() == text.len() && m.start() > 0 {
            return text[..m.start()].trim().to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> OcrLine {
        OcrLine {
            text: text.into(),
            conf: 90.0,
            bbox: None,
            page: 1,
        }
    }

    #[test]
    fn groups_speaker_headers() {
        let adapter = FiverrAdapter::default();
        let lines = vec![
            line("Dashboard My Business"),
            line("ahmadaoosaq123 Aug 07, 12:16 PM"),
            line("Honestly, I feel like I got scammed."),
            line("Me Aug 07, 1:12 PM"),
            line("Kindly allow me some time."),
        ];
        let msgs = adapter.structure(&lines);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].speaker, "buyer");
        assert_eq!(msgs[1].speaker, "seller");
    }

    #[test]
    fn tolerates_ocr_noisy_headers() {
        let adapter = FiverrAdapter::default();
        let lines = vec![
            line("CA) ahmadaoosaqi23 Aug 07,"),
            line("Bro, I'd really prefer to resolve this."),
            line("@ Me Aug 07"),
            line("Kindly allow me some time to get back with an update."),
        ];
        let msgs = adapter.structure(&lines);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].speaker, "buyer");
        assert_eq!(msgs[1].speaker, "seller");
    }
}
