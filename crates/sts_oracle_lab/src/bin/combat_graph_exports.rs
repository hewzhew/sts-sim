//! Persistence orchestration for selected local combat graph paths.

use std::path::{Path, PathBuf};

use sts_combat_planner::TurnOptionAction;
use sts_oracle_runtime::eval::combat_case::CombatCase;

use super::combat_replay_tools::{export_descendant_combat_case, save_combat_inputs};

pub(super) struct LocalGraphExportPaths<'a> {
    pub(super) witness_actions: Option<&'a Path>,
    pub(super) deepest_survival_case: Option<&'a Path>,
    pub(super) deepest_progress_case: Option<&'a Path>,
}

pub(super) struct LocalGraphExportActions<'a> {
    pub(super) witness: Option<&'a [TurnOptionAction]>,
    pub(super) deepest_survival: &'a [TurnOptionAction],
    pub(super) deepest_progress: &'a [TurnOptionAction],
}

pub(super) struct LocalGraphExports {
    pub(super) witness_actions: Option<PathBuf>,
    pub(super) deepest_survival_case: Option<PathBuf>,
    pub(super) deepest_survival_actions: Option<PathBuf>,
    pub(super) deepest_progress_case: Option<PathBuf>,
    pub(super) deepest_progress_actions: Option<PathBuf>,
}

pub(super) fn export_local_graph_paths(
    base: &CombatCase,
    paths: LocalGraphExportPaths<'_>,
    actions: LocalGraphExportActions<'_>,
    max_engine_steps_per_transition: usize,
) -> Result<LocalGraphExports, String> {
    let witness_actions = match (paths.witness_actions, actions.witness) {
        (Some(path), Some(actions)) => {
            save_combat_inputs(path, actions.iter().map(|action| action.input.clone()))?;
            Some(path.to_path_buf())
        }
        _ => None,
    };
    let (deepest_survival_case, deepest_survival_actions) =
        if let Some(path) = paths.deepest_survival_case {
            let actions_path = export_descendant_combat_case(
                base,
                actions.deepest_survival,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_survival",
            )?;
            (Some(path.to_path_buf()), Some(actions_path))
        } else {
            (None, None)
        };
    let (deepest_progress_case, deepest_progress_actions) =
        if let Some(path) = paths.deepest_progress_case {
            let actions_path = export_descendant_combat_case(
                base,
                actions.deepest_progress,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_progress",
            )?;
            (Some(path.to_path_buf()), Some(actions_path))
        } else {
            (None, None)
        };

    Ok(LocalGraphExports {
        witness_actions,
        deepest_survival_case,
        deepest_survival_actions,
        deepest_progress_case,
        deepest_progress_actions,
    })
}
