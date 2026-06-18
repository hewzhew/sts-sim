mod audit;
mod candidate;
mod card_reward_adapter;
mod compiler;
mod delta;
mod ledger;
mod retention;
mod run_debt;
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
    add_run_debt_candidate_deltas_v1, add_run_debt_pressure_to_ledger,
    add_startup_profile_pressure_to_ledger, ledger_from_snapshot, PressureHorizon, PressureItem,
    PressureKind, PressureLedger, RunDebtCandidateSignalsV1, StrategicBossTax, StrategicDebt,
    StrategicJob,
};
pub use retention::{
    compact_branch_signature, compact_branch_signature_data, format_compact_branch_signature,
    BranchSignature, BranchSignatureCompact, RetentionBucket,
};
pub use run_debt::{
    run_debt_ledger_for_relics_v1, run_debt_ledger_v1, run_debt_projection_for_relic_v1,
    run_debt_tag_rank_adjustment_v1, RunDebtContractKindV1, RunDebtContractV1, RunDebtLedgerV1,
    RunDebtProjectionV1,
};
pub use shop_adapter::strategic_trace_for_shop;
pub use snapshot::{StrategicDeckFacts, StrategicRouteFacts, StrategicSnapshot};

#[cfg(test)]
mod tests;
