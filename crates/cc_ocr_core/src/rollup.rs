//! Conversation-level aggregation using a situation [`RiskPolicy`].

use cc_ocr_plugin_api::{ConversationRollup, LabeledMessage, RiskPolicy};
use std::collections::BTreeMap;

/// Aggregate labeled messages into a conversation rollup.
pub fn rollup(source: &str, messages: &[LabeledMessage], policy: &RiskPolicy) -> ConversationRollup {
    let mut intent_histogram: BTreeMap<String, u32> = BTreeMap::new();
    let mut risk_flags: Vec<String> = Vec::new();

    for m in messages {
        *intent_histogram.entry(m.intent.clone()).or_insert(0) += 1;

        if policy.risk_intents.iter().any(|i| i == &m.intent) {
            let flag = format!("intent:{}", m.intent);
            if !risk_flags.contains(&flag) {
                risk_flags.push(flag);
            }
        }

        let lower = m.text.to_ascii_lowercase();
        for kw in &policy.risk_keywords {
            if lower.contains(&kw.to_ascii_lowercase()) {
                let flag = format!("keyword:{kw}");
                if !risk_flags.contains(&flag) {
                    risk_flags.push(flag);
                }
            }
        }
    }

    ConversationRollup {
        source: source.to_string(),
        messages: messages.to_vec(),
        intent_histogram,
        risk_flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_dispute_language() {
        let policy = RiskPolicy::new(["dispute"], ["fiverr support"]);
        let msgs = vec![LabeledMessage {
            text: "I will contact Fiverr Support".into(),
            intent: "dispute".into(),
            confidence: 0.9,
            speaker: Some("buyer".into()),
            time: None,
        }];
        let r = rollup("test.pdf", &msgs, &policy);
        assert!(r.risk_flags.iter().any(|f| f.contains("dispute")));
        assert!(r.risk_flags.iter().any(|f| f.contains("fiverr support")));
    }
}
