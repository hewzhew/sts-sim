mod audit;
mod candidate;
mod card_reward_adapter;
mod compiler;
mod delta;
mod ledger;
mod retention;
mod shop_adapter;
mod snapshot;

pub use audit::{audit_delta_coverage, StrategicAuditReport};
pub use candidate::{CandidateAction, StrategicDecisionSite};
pub use card_reward_adapter::strategic_trace_for_card_reward;
pub use compiler::{
    compile_decision, AcquisitionVerdict, CompiledDecision, StrategicDecisionTrace,
};
pub use delta::{
    CandidateDelta, CandidateRole, LedgerDelta, OpportunityCost, StrategicContraindication,
    VerdictHint,
};
pub use ledger::{
    ledger_from_snapshot, PressureHorizon, PressureItem, PressureKind, PressureLedger,
    StrategicBossTax, StrategicDebt, StrategicJob,
};
pub use retention::{
    compact_branch_signature, compact_branch_signature_data, format_compact_branch_signature,
    BranchSignature, BranchSignatureCompact, RetentionBucket,
};
pub use shop_adapter::strategic_trace_for_shop;
pub use snapshot::{StrategicDeckFacts, StrategicRouteFacts, StrategicSnapshot};

#[cfg(test)]
mod tests;
