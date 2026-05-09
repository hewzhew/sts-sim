use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionMetadata {
    pub source: &'static str,
    pub rationale_key: Option<&'static str>,
    pub confidence: Option<f32>,
    pub fallback_used: bool,
}

impl DecisionMetadata {
    pub const fn new(
        source: &'static str,
        rationale_key: Option<&'static str>,
        confidence: Option<f32>,
        fallback_used: bool,
    ) -> Self {
        Self {
            source,
            rationale_key,
            confidence,
            fallback_used,
        }
    }
}
