use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionBucket {
    BestImmediateSurvival,
    BestBossPrepared,
    BestCleanDeck,
    BestCoreEngine,
    BestResourceConverted,
    BestHighVariance,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct BranchSignature {
    pub boss_readiness: f32,
    pub clean_score: f32,
    pub engine_score: f32,
    pub cycle_debt: f32,
    pub setup_debt: f32,
    pub economy_conversion: f32,
    pub package_coherence: f32,
    pub buckets: Vec<RetentionBucket>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct BranchSignatureCompact {
    pub present: bool,
    pub boss_readiness_milli: i32,
    pub clean_score_milli: i32,
    pub engine_score_milli: i32,
    pub cycle_debt_milli: i32,
    pub setup_debt_milli: i32,
    pub economy_conversion_milli: i32,
    pub package_coherence_milli: i32,
}

impl BranchSignatureCompact {
    pub fn is_empty(&self) -> bool {
        !self.present
    }
}

pub fn compact_branch_signature(signature: &BranchSignature) -> String {
    format_compact_branch_signature(&compact_branch_signature_data(signature))
}

pub fn compact_branch_signature_data(signature: &BranchSignature) -> BranchSignatureCompact {
    if signature == &BranchSignature::default() {
        return BranchSignatureCompact::default();
    }
    BranchSignatureCompact {
        present: true,
        boss_readiness_milli: signal_milli(signature.boss_readiness),
        clean_score_milli: signal_milli(signature.clean_score),
        engine_score_milli: signal_milli(signature.engine_score),
        cycle_debt_milli: signal_milli(signature.cycle_debt),
        setup_debt_milli: signal_milli(signature.setup_debt),
        economy_conversion_milli: signal_milli(signature.economy_conversion),
        package_coherence_milli: signal_milli(signature.package_coherence),
    }
}

pub fn format_compact_branch_signature(signature: &BranchSignatureCompact) -> String {
    if signature.is_empty() {
        return String::new();
    }
    format!(
        "boss:{} clean:{} eng:{} debt:{}/{} pkg:{}",
        render_signal_1dp(signature.boss_readiness_milli),
        render_signal_1dp(signature.clean_score_milli),
        render_signal_1dp(signature.engine_score_milli),
        render_signal_1dp(signature.cycle_debt_milli),
        render_signal_1dp(signature.setup_debt_milli),
        render_signal_1dp(signature.package_coherence_milli),
    )
}

fn signal_milli(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as i32
}

fn render_signal_1dp(value_milli: i32) -> String {
    let value_milli = value_milli.clamp(0, 1000);
    let tenths = (value_milli + 50) / 100;
    format!("{}.{}", tenths / 10, tenths % 10)
}
