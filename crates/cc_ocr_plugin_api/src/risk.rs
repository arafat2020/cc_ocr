//! Risk rollup policy provided by a situation adapter.

/// Intents and keywords that raise conversation-level risk flags.
#[derive(Debug, Clone, Default)]
pub struct RiskPolicy {
    pub risk_intents: Vec<String>,
    pub risk_keywords: Vec<String>,
}

impl RiskPolicy {
    pub fn new(
        risk_intents: impl IntoIterator<Item = impl Into<String>>,
        risk_keywords: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            risk_intents: risk_intents.into_iter().map(Into::into).collect(),
            risk_keywords: risk_keywords.into_iter().map(Into::into).collect(),
        }
    }
}
