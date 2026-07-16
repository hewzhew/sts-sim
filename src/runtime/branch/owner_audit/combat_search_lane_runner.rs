use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlHpLossLimit, RunControlSession, RunProgressOutcome,
};

use super::accepted_high_loss_diagnostic::{
    accepted_high_loss_diagnostic, capture_active_combat, AcceptedHighLossDiagnosticDraft,
};
use super::combat_search_incumbent::{CombatSearchCandidateFacts, CombatSearchCandidateTier};
use super::combat_search_lane_commit::lane_commits;
use super::combat_search_lanes::{CombatSearchLane, CombatSearchRequest};
use super::combat_search_report::{
    combat_portfolio_attempt_report, CombatSearchLaneReport, CombatSearchLaneReportInput,
};
use super::combat_search_survival::owner_audit_hp_loss_limit;
use super::combat_search_trace_actions::complete_search_action_keys;
use super::{boundary_router, BranchStatus};

pub(super) struct CombatSearchLaneAttempt {
    trial_session: Option<RunControlSession>,
    pub(super) outcome: Option<RunProgressOutcome>,
    pub(super) status: BranchStatus,
    pub(super) label: &'static str,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy:
        Option<sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy>,
    pub(super) max_potions_used: Option<u32>,
    pub(super) action_keys: Vec<String>,
    pub(super) internal_no_win_rescue_enabled: bool,
    pub(super) applicable: bool,
    pub(super) selected: bool,
    pub(super) incumbent_reason: &'static str,
    pub(super) candidate_facts: Option<CombatSearchCandidateFacts>,
    pub(super) engine_fingerprint: String,
    pub(super) accepted_high_loss_diagnostic: Option<AcceptedHighLossDiagnosticDraft>,
}

pub(super) fn run_lane_attempt(
    session: &RunControlSession,
    request: &CombatSearchRequest,
    lane: CombatSearchLane,
) -> Result<CombatSearchLaneAttempt, String> {
    let combat_capture = capture_active_combat(session)?;
    let mut trial = session.clone();
    let options = lane.options(request, session);
    let owner_hp_loss_limit = match owner_audit_hp_loss_limit(session) {
        RunControlHpLossLimit::Limit(limit) => Some(limit),
        RunControlHpLossLimit::Unlimited => None,
    };
    let root_potion_count = visible_potion_count(session);
    let profile_config = options.search.profile.map(|profile| profile.to_config());
    let engine_fingerprint = options
        .search
        .profile
        .map(|profile| profile.engine_fingerprint())
        .unwrap_or_else(|| "manual_default".to_string());
    let max_nodes = options
        .search
        .max_nodes
        .or_else(|| profile_config.as_ref().map(|config| config.max_nodes))
        .unwrap_or_default();
    let wall_ms = options
        .search
        .wall_ms
        .or_else(|| {
            profile_config
                .as_ref()
                .and_then(|config| config.wall_time.map(|duration| duration.as_millis() as u64))
        })
        .unwrap_or_default();
    let potion_policy = options
        .search
        .potion_policy
        .or_else(|| profile_config.as_ref().map(|config| config.potion_policy));
    let max_potions_used = options.search.max_potions_used.or_else(|| {
        profile_config
            .as_ref()
            .and_then(|config| config.max_potions_used)
    });
    let internal_no_win_rescue_enabled =
        !options.search.disable_no_win_rescue || options.search.allow_smoke_bomb_survival_fallback;
    let outcome = match trial.apply_combat_search(options.search) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(CombatSearchLaneAttempt {
                trial_session: None,
                outcome: None,
                status: BranchStatus::AdvanceFailed(err),
                label: lane.label(),
                max_nodes,
                wall_ms,
                potion_policy,
                max_potions_used,
                action_keys: Vec::new(),
                internal_no_win_rescue_enabled,
                applicable: false,
                selected: false,
                incumbent_reason: "invalid_result",
                candidate_facts: None,
                engine_fingerprint,
                accepted_high_loss_diagnostic: None,
            });
        }
    };
    let status = lane_status(&trial, &outcome);
    let action_keys = complete_search_action_keys(&outcome.trace_annotations);
    let applicable = lane_commits(lane.commit_policy(), &status);
    let candidate_facts = applicable.then(|| {
        candidate_facts(
            &trial,
            &status,
            &outcome,
            owner_hp_loss_limit,
            root_potion_count,
            action_keys.len(),
        )
    });
    let accepted_high_loss_diagnostic = combat_capture.and_then(|capture| {
        accepted_high_loss_diagnostic(
            capture,
            lane.label(),
            &outcome.trace_annotations,
            applicable,
            owner_hp_loss_limit,
        )
    });
    Ok(CombatSearchLaneAttempt {
        trial_session: applicable.then_some(trial),
        outcome: Some(outcome),
        status,
        label: lane.label(),
        max_nodes,
        wall_ms,
        potion_policy,
        max_potions_used,
        action_keys,
        internal_no_win_rescue_enabled,
        applicable,
        selected: false,
        incumbent_reason: if applicable {
            "not_evaluated"
        } else {
            "invalid_result"
        },
        candidate_facts,
        engine_fingerprint,
        accepted_high_loss_diagnostic,
    })
}

impl CombatSearchLaneAttempt {
    pub(super) fn commit_into(&mut self, session: &mut RunControlSession) -> Result<(), String> {
        if !self.applicable {
            return Err(format!("lane {} has no applicable trial", self.label));
        }
        let trial = self
            .trial_session
            .take()
            .ok_or_else(|| format!("lane {} trial session already consumed", self.label))?;
        *session = trial;
        self.selected = true;
        Ok(())
    }

    pub(super) fn duplicate_engine_suppressed(
        session: &RunControlSession,
        request: &CombatSearchRequest,
        lane: CombatSearchLane,
    ) -> Self {
        let options = lane.options(request, session);
        let profile_config = options.search.profile.map(|profile| profile.to_config());
        Self {
            trial_session: None,
            outcome: None,
            status: BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: "duplicate_engine_suppressed".to_string(),
            },
            label: lane.label(),
            max_nodes: options
                .search
                .max_nodes
                .or_else(|| profile_config.as_ref().map(|config| config.max_nodes))
                .unwrap_or_default(),
            wall_ms: options
                .search
                .wall_ms
                .or_else(|| {
                    profile_config.as_ref().and_then(|config| {
                        config.wall_time.map(|duration| duration.as_millis() as u64)
                    })
                })
                .unwrap_or_default(),
            potion_policy: options.search.potion_policy,
            max_potions_used: profile_config
                .as_ref()
                .and_then(|config| config.max_potions_used),
            action_keys: Vec::new(),
            internal_no_win_rescue_enabled: !options.search.disable_no_win_rescue,
            applicable: false,
            selected: false,
            incumbent_reason: "duplicate_engine_suppressed",
            candidate_facts: None,
            engine_fingerprint: options
                .search
                .profile
                .map(|profile| profile.engine_fingerprint())
                .unwrap_or_else(|| "manual_default".to_string()),
            accepted_high_loss_diagnostic: None,
        }
    }

    #[cfg(test)]
    pub(super) fn synthetic_for_test(
        root: &RunControlSession,
        label: &'static str,
        candidate_facts: CombatSearchCandidateFacts,
    ) -> Self {
        let mut trial = root.clone();
        trial.run_state.current_hp = candidate_facts.run_hp;
        Self {
            trial_session: Some(trial),
            outcome: None,
            status: BranchStatus::AwaitingAuto {
                boundary: label.to_string(),
                reason: "synthetic accepted line".to_string(),
            },
            label,
            max_nodes: 0,
            wall_ms: 0,
            potion_policy: None,
            max_potions_used: None,
            action_keys: Vec::new(),
            internal_no_win_rescue_enabled: false,
            applicable: true,
            selected: false,
            incumbent_reason: "not_evaluated",
            candidate_facts: Some(candidate_facts),
            engine_fingerprint: "synthetic".to_string(),
            accepted_high_loss_diagnostic: None,
        }
    }
}

fn candidate_facts(
    trial: &RunControlSession,
    status: &BranchStatus,
    outcome: &RunProgressOutcome,
    owner_hp_loss_limit: Option<u32>,
    root_potion_count: u32,
    fallback_action_count: usize,
) -> CombatSearchCandidateFacts {
    let best_win =
        sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
            .find_map(|summary| summary.best_win);
    let tier = candidate_tier(
        best_win.as_ref().map(|summary| summary.hp_loss),
        owner_hp_loss_limit,
    );
    let run_hp = trial.visible_player_hp().0;
    CombatSearchCandidateFacts {
        terminal_run_victory: matches!(
            status,
            BranchStatus::Terminal(super::TerminalOutcome::Victory)
        ),
        tier,
        combat_final_hp: best_win
            .as_ref()
            .map(|summary| summary.final_hp)
            .unwrap_or(run_hp),
        run_hp,
        potions_used: best_win
            .as_ref()
            .map(|summary| summary.potions_used)
            .unwrap_or_else(|| root_potion_count.saturating_sub(visible_potion_count(trial))),
        potions_discarded: best_win
            .as_ref()
            .map(|summary| summary.potions_discarded)
            .unwrap_or_default(),
        turns: best_win
            .as_ref()
            .map(|summary| summary.turns)
            .unwrap_or_default(),
        action_count: best_win
            .as_ref()
            .map(|summary| summary.action_count)
            .unwrap_or(fallback_action_count),
    }
}

fn visible_potion_count(session: &RunControlSession) -> u32 {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            active
                .combat_state
                .entities
                .potions
                .iter()
                .flatten()
                .count()
        })
        .unwrap_or_else(|| session.run_state.potions.iter().flatten().count()) as u32
}

fn candidate_tier(
    hp_loss: Option<i32>,
    owner_hp_loss_limit: Option<u32>,
) -> CombatSearchCandidateTier {
    match hp_loss {
        None => CombatSearchCandidateTier::SurvivalFallback,
        Some(hp_loss) if owner_hp_loss_limit.is_some_and(|limit| hp_loss.max(0) as u32 > limit) => {
            CombatSearchCandidateTier::RelaxedCompleteWin
        }
        Some(_) => CombatSearchCandidateTier::ReserveCompliantCompleteWin,
    }
}

pub(super) fn combat_search_summaries(
    attempt: &CombatSearchLaneAttempt,
) -> Vec<CombatSearchTraceSummary> {
    let Some(outcome) = attempt.outcome.as_ref() else {
        return Vec::new();
    };
    let mut summaries =
        sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
            .collect::<Vec<_>>();
    for summary in &mut summaries {
        summary.lane = Some(attempt.label.to_string());
        summary.profile_id = Some(attempt.label.to_string());
        summary.profile_max_nodes = Some(attempt.max_nodes);
        summary.profile_wall_ms = Some(attempt.wall_ms);
        summary.profile_potion_policy =
            Some(potion_policy_label(attempt.potion_policy).to_string());
        summary.profile_max_potions_used = attempt.max_potions_used;
        summary.profile_internal_no_win_rescue_enabled =
            Some(attempt.internal_no_win_rescue_enabled);
        summary.engine_fingerprint = Some(attempt.engine_fingerprint.clone());
        summary.portfolio_candidate_tier = attempt
            .candidate_facts
            .map(|facts| facts.tier.as_str().to_string());
        summary.portfolio_selected = Some(attempt.selected);
        summary.portfolio_decision = Some(attempt.incumbent_reason.to_string());
    }
    summaries
}

pub(super) fn lane_attempt_report(attempt: &CombatSearchLaneAttempt) -> CombatSearchLaneReport {
    combat_portfolio_attempt_report(CombatSearchLaneReportInput {
        label: attempt.label,
        status: attempt.status.clone(),
        max_nodes: attempt.max_nodes,
        wall_ms: attempt.wall_ms,
        potion_policy: attempt.potion_policy,
        max_potions_used: attempt.max_potions_used,
        action_keys: attempt.action_keys.clone(),
        engine_fingerprint: attempt.engine_fingerprint.clone(),
        candidate_tier: attempt
            .candidate_facts
            .map(|facts| facts.tier.as_str().to_string()),
        selected: attempt.selected,
        incumbent_reason: attempt.incumbent_reason.to_string(),
        combat_final_hp: attempt.candidate_facts.map(|facts| facts.combat_final_hp),
        run_hp: attempt.candidate_facts.map(|facts| facts.run_hp),
        potions_used: attempt.candidate_facts.map(|facts| facts.potions_used),
        turns: attempt.candidate_facts.map(|facts| facts.turns),
    })
}

fn lane_status(session: &RunControlSession, outcome: &RunProgressOutcome) -> BranchStatus {
    if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else {
        boundary_router::classify_auto_outcome(session, outcome)
    }
}

fn potion_policy_label(
    policy: Option<sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy>,
) -> &'static str {
    match policy {
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never) => "never",
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::All) => "all",
        Some(sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted) => {
            "semantic"
        }
        None => "default",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::owner_audit::run_contract::RunObjective;
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use sts_simulator::state::map::node::RoomType;

    fn test_args() -> super::super::Args {
        super::super::Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn candidate_tier_uses_owner_reserve_without_discarding_relaxed_win() {
        assert_eq!(
            candidate_tier(Some(42), Some(60)),
            super::super::combat_search_incumbent::CombatSearchCandidateTier::ReserveCompliantCompleteWin
        );
        assert_eq!(
            candidate_tier(Some(67), Some(60)),
            super::super::combat_search_incumbent::CombatSearchCandidateTier::RelaxedCompleteWin
        );
        assert_eq!(
            candidate_tier(None, Some(60)),
            super::super::combat_search_incumbent::CombatSearchCandidateTier::SurvivalFallback
        );
    }

    #[test]
    fn lane_attempt_does_not_mutate_root_session() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let request = CombatSearchRequest::from_session(&session, test_args());
        let before_engine = format!("{:?}", session.engine_state);
        let before_run_hp = session.run_state.current_hp;
        let before_combat_hp = session
            .active_combat
            .as_ref()
            .expect("active combat")
            .combat_state
            .entities
            .player
            .current_hp;

        let result = run_lane_attempt(&session, &request, CombatSearchLane::primary());

        assert!(result.is_ok());
        assert_eq!(format!("{:?}", session.engine_state), before_engine);
        assert_eq!(session.run_state.current_hp, before_run_hp);
        assert_eq!(
            session
                .active_combat
                .as_ref()
                .expect("active combat")
                .combat_state
                .entities
                .player
                .current_hp,
            before_combat_hp
        );
    }

    #[test]
    fn applicable_trial_commits_only_when_explicitly_requested() {
        let mut root = RunControlSession::new(RunControlConfig::default());
        root.engine_state = EngineState::CombatPlayerTurn;
        root.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let mut attempt = CombatSearchLaneAttempt {
            trial_session: Some(root.clone()),
            outcome: None,
            status: BranchStatus::Terminal(super::super::TerminalOutcome::Victory),
            label: "primary",
            max_nodes: 0,
            wall_ms: 0,
            potion_policy: None,
            max_potions_used: None,
            action_keys: Vec::new(),
            internal_no_win_rescue_enabled: false,
            applicable: true,
            selected: false,
            incumbent_reason: "test_fixture",
            candidate_facts: None,
            engine_fingerprint: "test_fixture".to_string(),
            accepted_high_loss_diagnostic: None,
        };
        let root_engine = format!("{:?}", root.engine_state);
        let mut committed = root.clone();
        committed.run_state.current_hp = -123;

        attempt
            .commit_into(&mut committed)
            .expect("applicable trial should commit");

        assert!(attempt.selected);
        assert_eq!(format!("{:?}", root.engine_state), root_engine);
        assert_eq!(committed.run_state.current_hp, root.run_state.current_hp);
        assert_ne!(committed.run_state.current_hp, -123);
    }
}
