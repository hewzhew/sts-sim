use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelicCompatibility {
    HardReject,
    HighRisk,
    Neutral,
    StrongFit,
}

impl RelicCompatibility {
    pub(crate) const fn bucket(self) -> i32 {
        match self {
            Self::HardReject => 0,
            Self::HighRisk => 1,
            Self::Neutral => 2,
            Self::StrongFit => 3,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RelicJudgement {
    pub compatibility: RelicCompatibility,
    pub upside: i32,
    pub downside: i32,
    pub volatility: i32,
    pub confidence: i32,
    pub primary_reason: &'static str,
    pub positive_tags: Vec<&'static str>,
    pub negative_tags: Vec<&'static str>,
}

impl RelicJudgement {
    pub(crate) fn new(
        compatibility: RelicCompatibility,
        upside: i32,
        downside: i32,
        volatility: i32,
        confidence: i32,
        primary_reason: &'static str,
        positive_tags: Vec<&'static str>,
        negative_tags: Vec<&'static str>,
    ) -> Self {
        Self {
            compatibility,
            upside,
            downside,
            volatility,
            confidence,
            primary_reason,
            positive_tags,
            negative_tags,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BossRelicCandidate {
    pub index: usize,
    pub relic_id: String,
    pub compatibility: RelicCompatibility,
    pub rank_score: i32,
    pub upside: i32,
    pub downside: i32,
    pub volatility: i32,
    pub confidence: i32,
    pub primary_reason: &'static str,
    pub positive_tags: Vec<&'static str>,
    pub negative_tags: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BossRelicDecisionDiagnostics {
    pub chosen_index: Option<usize>,
    pub top_candidates: Vec<BossRelicCandidate>,
}
