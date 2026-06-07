use crate::content::relics::RelicId;
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

#[derive(Clone, Debug)]
pub(crate) struct BossRelicBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
}

pub(crate) fn boss_relic_branch_options(
    session: &RunControlSession,
) -> Option<Vec<BossRelicBranchOption>> {
    let EngineState::BossRelicSelect(choice) = &session.engine_state else {
        return None;
    };
    let options = choice
        .relics
        .iter()
        .enumerate()
        .map(|(idx, relic)| BossRelicBranchOption {
            label: boss_relic_label(*relic),
            command: format!("relic {idx}"),
        })
        .collect::<Vec<_>>();
    (!options.is_empty()).then_some(options)
}

fn boss_relic_label(relic: RelicId) -> String {
    format!("{relic:?}")
}
