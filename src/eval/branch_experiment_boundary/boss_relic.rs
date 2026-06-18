use crate::ai::boss_relic_policy_v1::build_boss_relic_decision_context_v1;
use crate::ai::strategic::RunDebtProjectionV1;
use crate::content::relics::RelicId;
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

#[derive(Clone, Debug)]
pub(crate) struct BossRelicBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) effect_kind: String,
}

pub(crate) fn boss_relic_branch_options(
    session: &RunControlSession,
) -> Option<Vec<BossRelicBranchOption>> {
    let EngineState::BossRelicSelect(choice) = &session.engine_state else {
        return None;
    };
    let context = build_boss_relic_decision_context_v1(&session.run_state, choice.relics.clone());
    let options = context
        .candidates
        .into_iter()
        .map(|candidate| BossRelicBranchOption {
            label: boss_relic_label(candidate.relic, &candidate.debt_projection),
            command: format!("relic {}", candidate.index),
            effect_kind: format!("boss_relic:{:?}", candidate.relic),
        })
        .collect::<Vec<_>>();
    (!options.is_empty()).then_some(options)
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
