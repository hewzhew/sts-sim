use sts_simulator::eval::run_control::RunControlSession;

use super::run_cutpoint::{RunCutpointKind, RunCutpointSnapshot};
use super::run_cutpoint_store::{RunCutpointHandle, RunCutpointStore};
use super::{Args, Branch, BranchStatus, Owner};

pub(super) struct RunCutpointRecorder<'a> {
    store: Option<&'a RunCutpointStore>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    branch_template: Branch,
    active_pre_combat: Option<RunCutpointHandle>,
}

impl<'a> RunCutpointRecorder<'a> {
    pub(super) fn new(
        store: Option<&'a RunCutpointStore>,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        branch_template: &Branch,
    ) -> Self {
        Self {
            store,
            args,
            generation,
            next_branch_id,
            branch_template: branch_template.clone(),
            active_pre_combat: None,
        }
    }

    pub(super) fn before_combat_search(
        &mut self,
        session: &RunControlSession,
    ) -> Result<(), String> {
        if session.active_combat.is_none() || self.active_pre_combat.is_some() {
            return Ok(());
        }
        let Some(store) = self.store else {
            return Ok(());
        };
        let mut branch = self.branch_template.clone();
        branch.session = session.clone();
        branch.status = BranchStatus::AwaitingAuto {
            boundary: "Combat".to_string(),
            reason: "resume pre-combat search cutpoint".to_string(),
        };
        let snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::PreCombatSearch,
            self.generation,
            &branch,
        )?;
        self.active_pre_combat =
            Some(store.write_pre_combat_inflight(self.args, self.next_branch_id, snapshot)?);
        Ok(())
    }

    pub(super) fn after_combat_search(&mut self, status: &BranchStatus) -> Result<(), String> {
        let Some(handle) = self.active_pre_combat.take() else {
            return Ok(());
        };
        let store = self.store.expect("a cutpoint handle requires a store");
        if matches!(
            status,
            BranchStatus::CombatGap { .. }
                | BranchStatus::BudgetGap { .. }
                | BranchStatus::AwaitingAuto { .. }
        ) {
            store.retain_pre_combat_gap(handle)
        } else {
            store.discard_pre_combat(handle)
        }
    }

    pub(super) fn retain_on_error(&mut self) -> Result<(), String> {
        let Some(handle) = self.active_pre_combat.take() else {
            return Ok(());
        };
        self.store
            .expect("a cutpoint handle requires a store")
            .retain_pre_combat_gap(handle)
    }

    pub(super) fn capture_owner_boundary(&self, branch: &Branch) -> Result<(), String> {
        let BranchStatus::Running {
            owner: Owner::BossRelic,
            ..
        } = branch.status
        else {
            return Ok(());
        };
        let Some(store) = self.store else {
            return Ok(());
        };
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, self.generation, branch)?;
        store.write_boss_relic(self.args, self.next_branch_id, snapshot)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    use super::super::frontier_checkpoint;
    use super::super::run_cutpoint_store::RunCutpointStore;
    use super::super::{Args, Branch, BranchStatus, Owner};
    use super::*;

    #[test]
    fn combat_gap_promotes_the_pre_search_session_not_the_mutated_session() {
        let (store, args, mut branch) = active_combat_branch();
        let expected_gold = branch.session.run_state.gold;
        let mut recorder = RunCutpointRecorder::new(Some(&store), args, 17, branch.id + 1, &branch);
        recorder.before_combat_search(&branch.session).unwrap();
        branch.session.run_state.gold += 99;

        recorder
            .after_combat_search(&BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: "no accepted win".to_string(),
            })
            .unwrap();

        let checkpoint =
            frontier_checkpoint::load(&store.latest_pre_combat_frontier_path()).unwrap();
        let (frontier, _) = checkpoint.into_frontier().unwrap();
        assert_eq!(
            frontier.front().unwrap().session.run_state.gold,
            expected_gold
        );
        assert!(matches!(
            frontier.front().unwrap().status,
            BranchStatus::AwaitingAuto { .. }
        ));
    }

    #[test]
    fn successful_search_removes_its_inflight_pair() {
        let (store, args, branch) = active_combat_branch();
        let mut recorder = RunCutpointRecorder::new(Some(&store), args, 17, branch.id + 1, &branch);
        recorder.before_combat_search(&branch.session).unwrap();

        recorder
            .after_combat_search(&BranchStatus::Running {
                boundary: "Card Reward".to_string(),
                owner: Owner::CardReward,
            })
            .unwrap();

        assert!(!store.inflight_pre_combat_frontier_path(branch.id).exists());
        assert!(!store.latest_pre_combat_frontier_path().exists());
    }

    fn active_combat_branch() -> (RunCutpointStore, Args, Branch) {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (mut frontier, _) = super::super::branch_runtime::BranchRuntime::initial_frontier(
            args,
            std::time::Instant::now(),
        );
        let mut branch = frontier.pop_front().unwrap();
        let combat = crate::test_support::blank_test_combat();
        branch.session.engine_state = EngineState::CombatPlayerTurn;
        branch.session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        branch.status = BranchStatus::AwaitingAuto {
            boundary: "Combat".to_string(),
            reason: "test".to_string(),
        };
        (
            RunCutpointStore::new(unique_root("recorder").join("cutpoints")),
            args,
            branch,
        )
    }

    fn unique_root(label: &str) -> PathBuf {
        static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ordinal = NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "sts_run_cutpoint_recorder_{label}_{}_{}_{ordinal}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ))
    }
}
