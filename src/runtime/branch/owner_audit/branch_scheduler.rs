use super::accepted_high_loss_diagnostic::extend_unique_diagnostics;
use super::owner_model::{OwnerChoice, OwnerDecision};
use super::run_cutpoint_recorder::RunCutpointRecorder;
use super::run_cutpoint_store::RunCutpointStore;
use super::run_deadline::RunDeadline;
use super::{owners, runner, Args, Branch, BranchStatus};

pub(super) fn prepare_branch_work(
    mut branch: Branch,
    args: Args,
    generation: usize,
    deadline: RunDeadline,
    cutpoint_store: Option<&RunCutpointStore>,
    next_branch_id: usize,
) -> (Branch, bool, Vec<OwnerChoice>) {
    let mut cutpoints =
        RunCutpointRecorder::new(cutpoint_store, args, generation, next_branch_id, &branch);
    if let Err(error) = cutpoints.capture_owner_boundary(&branch) {
        branch.status =
            BranchStatus::AdvanceFailed(format!("cutpoint persistence failed: {error}"));
        return (branch, false, Vec::new());
    }
    let mut expandable = generation < args.generations && branch.status.is_expandable_now();
    let mut choices = if expandable {
        branch_owner_choices(&branch)
    } else {
        Vec::new()
    };
    if generation < args.generations
        && (matches!(branch.status, BranchStatus::AwaitingAuto { .. })
            || (expandable && choices.is_empty()))
    {
        let advance = runner::advance_to_owner_or_gap_with_cutpoints(
            &mut branch.session,
            deadline.cap_args(args, 1),
            deadline,
            &mut cutpoints,
        );
        branch.status = advance.status;
        branch.combat_portfolio = advance.combat_portfolio;
        branch.recent_progress_journal = advance.progress_journal;
        branch.recent_planner_capture = advance.planner_capture;
        branch.combat_search = advance.combat_search;
        branch
            .combat_search_history
            .extend(branch.combat_search.clone());
        extend_unique_diagnostics(
            &mut branch.accepted_high_loss_diagnostics,
            advance.accepted_high_loss_diagnostics,
        );
        if let Err(error) = cutpoints.capture_owner_boundary(&branch) {
            branch.status =
                BranchStatus::AdvanceFailed(format!("cutpoint persistence failed: {error}"));
            return (branch, false, Vec::new());
        }
        expandable = generation < args.generations && branch.status.is_expandable_now();
        choices = if expandable {
            branch_owner_choices(&branch)
        } else {
            Vec::new()
        };
    }
    (branch, expandable, choices)
}

fn branch_owner_choices(branch: &Branch) -> Vec<OwnerChoice> {
    let BranchStatus::Running { owner, .. } = branch.status else {
        return Vec::new();
    };
    let surface = sts_simulator::eval::run_control::build_decision_surface(&branch.session);
    match owners::owner_decision(&branch.session, owner, &surface) {
        OwnerDecision::Candidates(choices) => choices,
        OwnerDecision::Routine(_) | OwnerDecision::Gap(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::rewards::BossRelicChoiceState;

    use super::super::frontier_checkpoint;
    use super::super::run_cutpoint_store::RunCutpointStore;
    use super::super::Owner;
    use super::*;

    #[test]
    fn boss_relic_cutpoint_is_captured_even_at_generation_limit() {
        let mut args = crate::runtime::branch::default_branch_args(20260713006);
        args.generations = 29;
        let (mut frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(
                args,
                std::time::Instant::now(),
            );
        let mut branch = frontier.pop_front().unwrap();
        branch.session.run_state.act_num = 2;
        branch.session.run_state.floor_num = 32;
        branch.session.run_state.current_hp = 13;
        branch.session.run_state.max_hp = 101;
        branch.session.engine_state =
            EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
                RelicId::BlackBlood,
                RelicId::CoffeeDripper,
                RelicId::PhilosopherStone,
            ]));
        branch.status = BranchStatus::Running {
            boundary: "Boss Relic".to_string(),
            owner: Owner::BossRelic,
        };
        let store = RunCutpointStore::new(unique_root().join("cutpoints"));
        let deadline = RunDeadline::new(std::time::Instant::now(), None);

        let (_branch, expandable, choices) = prepare_branch_work(
            branch,
            args,
            args.generations,
            deadline,
            Some(&store),
            next_branch_id,
        );

        assert!(!expandable);
        assert!(choices.is_empty());
        let frontier_path = store.boss_relic_frontier_path(2, 32);
        let manifest_path = store.boss_relic_manifest_path(2, 32);
        assert!(frontier_path.exists());
        assert!(manifest_path.exists());
        let checkpoint = frontier_checkpoint::load(&frontier_path).unwrap();
        let (frontier, _) = checkpoint.into_frontier().unwrap();
        let restored = frontier.front().unwrap();
        assert_eq!(restored.session.run_state.current_hp, 13);
        assert!(matches!(
            restored.status,
            BranchStatus::Running {
                owner: Owner::BossRelic,
                ..
            }
        ));
        assert_eq!(branch_owner_choices(restored).len(), 4);
    }

    fn unique_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sts_boss_relic_cutpoint_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ))
    }
}
