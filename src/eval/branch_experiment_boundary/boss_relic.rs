use crate::ai::boss_relic_policy_v1::{
    build_boss_relic_decision_context_v1, BossRelicCandidateEvidenceV1,
};
use crate::ai::strategic::RunDebtProjectionV1;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment::{
    BranchExperimentBossRelicCandidateEntryV1, BranchExperimentBossRelicCandidatePoolV1,
};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

#[derive(Clone, Debug)]
pub(crate) struct BossRelicBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) effect_kind: String,
}

#[derive(Clone, Debug)]
pub(crate) struct BossRelicBranchOptionSelection {
    pub(crate) options: Vec<BossRelicBranchOption>,
    pub(crate) candidate_pool: BranchExperimentBossRelicCandidatePoolV1,
}

pub(crate) fn boss_relic_branch_options(
    session: &RunControlSession,
) -> Option<Vec<BossRelicBranchOption>> {
    boss_relic_branch_selection(session).map(|selection| selection.options)
}

pub(crate) fn boss_relic_branch_selection(
    session: &RunControlSession,
) -> Option<BossRelicBranchOptionSelection> {
    let EngineState::BossRelicSelect(choice) = &session.engine_state else {
        return None;
    };
    let context = build_boss_relic_decision_context_v1(&session.run_state, choice.relics.clone());
    let options = context
        .candidates
        .iter()
        .map(|candidate| BossRelicBranchOption {
            label: boss_relic_label(candidate.relic, &candidate.debt_projection),
            command: format!("relic {}", candidate.index),
            effect_kind: format!("boss_relic:{:?}", candidate.relic),
        })
        .collect::<Vec<_>>();
    (!options.is_empty()).then(|| BossRelicBranchOptionSelection {
        candidate_pool: boss_relic_candidate_pool_from_context(&context.candidates),
        options,
    })
}

fn boss_relic_candidate_pool_from_context(
    candidates: &[BossRelicCandidateEvidenceV1],
) -> BranchExperimentBossRelicCandidatePoolV1 {
    BranchExperimentBossRelicCandidatePoolV1 {
        depth: 0,
        frontier_key: String::new(),
        boundary_title: String::new(),
        candidate_count: candidates.len(),
        branch_option_count: candidates.len(),
        candidates: candidates
            .iter()
            .map(boss_relic_candidate_entry_v1)
            .collect(),
    }
}

fn boss_relic_candidate_entry_v1(
    candidate: &BossRelicCandidateEvidenceV1,
) -> BranchExperimentBossRelicCandidateEntryV1 {
    BranchExperimentBossRelicCandidateEntryV1 {
        candidate_id: format!("boss_relic:{}:{:?}", candidate.index, candidate.relic),
        command: format!("relic {}", candidate.index),
        label: boss_relic_label(candidate.relic, &candidate.debt_projection),
        relic: format!("{:?}", candidate.relic),
        class: format!("{:?}", candidate.class),
        support_gate: format!("{:?}", candidate.support_gate),
        added_debt: candidate
            .debt_projection
            .added_contracts
            .iter()
            .map(|contract| contract.kind.label().to_string())
            .collect(),
        compounding_tags: candidate.debt_projection.compounding_tags.clone(),
        branch_admission: "selected".to_string(),
        evidence: candidate.evidence.clone(),
        risks: candidate.risks.clone(),
    }
}

fn boss_relic_label(relic: RelicId, projection: &RunDebtProjectionV1) -> String {
    let mut label = format!("{relic:?}");
    if !projection.added_contracts.is_empty() {
        let added = projection
            .added_contracts
            .iter()
            .map(|contract| contract.kind.label())
            .collect::<Vec<_>>()
            .join(",");
        label.push_str(&format!(" | adds debt {added}"));
    }
    if !projection.compounding_tags.is_empty() {
        label.push_str(&format!(
            " | compounds {}",
            projection.compounding_tags.join(",")
        ));
    }
    label
}
