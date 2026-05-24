use super::rollout_pending_choice::RolloutPendingChoiceProgress;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RolloutNodeEstimate {
    pub(super) evaluated: bool,
    pub(super) terminal: SearchTerminalLabel,
    pub(super) final_hp: i32,
    pub(super) hp_loss: i32,
    pub(super) turns: u32,
    pub(super) potions_used: u32,
    pub(super) potions_discarded: u32,
    pub(super) cards_played: u32,
    pub(super) living_enemy_count: usize,
    pub(super) total_enemy_hp: i32,
    pub(super) total_enemy_block: i32,
    pub(super) phase_adjusted_enemy_effort: i32,
    pub(super) special_enemy_phase_count: usize,
    pub(super) guardian_mode_shift_pending_count: usize,
    pub(super) lagavulin_waking_count: usize,
    pub(super) gremlin_nob_anger_amount_total: i32,
    pub(super) sentry_dazed_pressure_count: usize,
    pub(super) hexaghost_opening_pressure_count: usize,
    pub(super) high_fanout_pending_choice: bool,
    pub(super) pending_choice_estimated_action_fanout: usize,
    pub(super) pending_choices_seen: usize,
    pub(super) pending_choice_actions_simulated: usize,
    pub(super) max_pending_choice_candidate_count: usize,
    pub(super) max_pending_choice_estimated_action_fanout: usize,
    pub(super) last_pending_choice_kind: Option<&'static str>,
    pub(super) stopped_on_high_fanout_pending_choice: bool,
    pub(super) survival_margin: i32,
    pub(super) actions_simulated: usize,
    pub(super) truncated: bool,
    pub(super) stop_reason: RolloutStopReason,
    pub(super) last_action_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RolloutStopReason {
    NotEvaluated,
    TerminalState,
    MaxActions,
    Deadline,
    NoLegalActions,
    PolicyDeclined,
    EngineStepLimit,
    HighFanoutPendingChoice,
}

impl RolloutNodeEstimate {
    pub(super) fn unevaluated() -> Self {
        Self {
            evaluated: false,
            terminal: SearchTerminalLabel::Unresolved,
            final_hp: 0,
            hp_loss: 0,
            turns: 0,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            living_enemy_count: 0,
            total_enemy_hp: 0,
            total_enemy_block: 0,
            phase_adjusted_enemy_effort: 0,
            special_enemy_phase_count: 0,
            guardian_mode_shift_pending_count: 0,
            lagavulin_waking_count: 0,
            gremlin_nob_anger_amount_total: 0,
            sentry_dazed_pressure_count: 0,
            hexaghost_opening_pressure_count: 0,
            high_fanout_pending_choice: false,
            pending_choice_estimated_action_fanout: 0,
            pending_choices_seen: 0,
            pending_choice_actions_simulated: 0,
            max_pending_choice_candidate_count: 0,
            max_pending_choice_estimated_action_fanout: 0,
            last_pending_choice_kind: None,
            stopped_on_high_fanout_pending_choice: false,
            survival_margin: 0,
            actions_simulated: 0,
            truncated: false,
            stop_reason: RolloutStopReason::NotEvaluated,
            last_action_reason: None,
        }
    }

    pub(super) fn from_node(
        node: &SearchNode,
        actions_simulated: usize,
        stop_reason: RolloutStopReason,
        last_action_reason: Option<&'static str>,
        pending_choice_progress: RolloutPendingChoiceProgress,
    ) -> Self {
        let phase_profile = combat_search_phase_profile(&node.engine, &node.combat);
        Self {
            evaluated: true,
            terminal: terminal_label(&node.engine, &node.combat),
            final_hp: node.combat.entities.player.current_hp,
            hp_loss: (node.initial_hp - node.combat.entities.player.current_hp).max(0),
            turns: node.combat.turn.turn_count,
            potions_used: node.potions_used,
            potions_discarded: node.potions_discarded,
            cards_played: node.cards_played,
            living_enemy_count: living_enemy_count(&node.combat),
            total_enemy_hp: phase_profile.enemy_phase.raw_living_enemy_hp,
            total_enemy_block: phase_profile.enemy_phase.raw_living_enemy_block,
            phase_adjusted_enemy_effort: phase_profile
                .enemy_phase
                .phase_adjusted_living_enemy_effort,
            special_enemy_phase_count: phase_profile.special_enemy_phase_count(),
            guardian_mode_shift_pending_count: phase_profile
                .enemy_mechanics
                .guardian_mode_shift_pending_count,
            lagavulin_waking_count: phase_profile.enemy_mechanics.lagavulin_waking_count,
            gremlin_nob_anger_amount_total: phase_profile
                .enemy_mechanics
                .gremlin_nob_anger_amount_total,
            sentry_dazed_pressure_count: phase_profile.enemy_mechanics.sentry_dazed_pressure_count,
            hexaghost_opening_pressure_count: phase_profile
                .enemy_mechanics
                .hexaghost_opening_pressure_count,
            high_fanout_pending_choice: phase_profile.pending_choice.high_fanout,
            pending_choice_estimated_action_fanout: phase_profile
                .pending_choice
                .estimated_action_fanout,
            pending_choices_seen: pending_choice_progress.pending_choices_seen,
            pending_choice_actions_simulated: pending_choice_progress
                .pending_choice_actions_simulated,
            max_pending_choice_candidate_count: pending_choice_progress
                .max_pending_choice_candidate_count,
            max_pending_choice_estimated_action_fanout: pending_choice_progress
                .max_pending_choice_estimated_action_fanout,
            last_pending_choice_kind: pending_choice_progress.last_pending_choice_kind_label(),
            stopped_on_high_fanout_pending_choice: pending_choice_progress
                .stopped_on_high_fanout_pending_choice,
            survival_margin: phase_profile.pressure.survival_margin,
            actions_simulated,
            truncated: stop_reason.is_truncated(),
            stop_reason,
            last_action_reason,
        }
    }

    pub(super) fn priority_terminal_rank(self) -> i32 {
        if self.evaluated {
            terminal_rank(self.terminal)
        } else {
            0
        }
    }

    pub(super) fn enemy_progress(self) -> i32 {
        -(self.phase_adjusted_enemy_effort)
    }

    pub(super) fn potion_conservation(self) -> i32 {
        -((self.potions_used + self.potions_discarded) as i32)
    }

    pub(super) fn faster_turns(self) -> i32 {
        -(self.turns as i32)
    }

    pub(super) fn fewer_cards_played(self) -> i32 {
        -(self.cards_played as i32)
    }

    pub(super) fn to_report(self) -> Option<CombatSearchV2RolloutEstimateReport> {
        self.evaluated
            .then_some(CombatSearchV2RolloutEstimateReport {
                terminal: self.terminal,
                estimated: true,
                final_hp: self.final_hp,
                hp_loss: self.hp_loss,
                turns: self.turns,
                potions_used: self.potions_used,
                potions_discarded: self.potions_discarded,
                cards_played: self.cards_played,
                living_enemy_count: self.living_enemy_count,
                total_enemy_hp: self.total_enemy_hp,
                total_enemy_block: self.total_enemy_block,
                phase_adjusted_enemy_effort: self.phase_adjusted_enemy_effort,
                special_enemy_phase_count: self.special_enemy_phase_count,
                guardian_mode_shift_pending_count: self.guardian_mode_shift_pending_count,
                lagavulin_waking_count: self.lagavulin_waking_count,
                gremlin_nob_anger_amount_total: self.gremlin_nob_anger_amount_total,
                sentry_dazed_pressure_count: self.sentry_dazed_pressure_count,
                hexaghost_opening_pressure_count: self.hexaghost_opening_pressure_count,
                high_fanout_pending_choice: self.high_fanout_pending_choice,
                pending_choice_estimated_action_fanout: self.pending_choice_estimated_action_fanout,
                pending_choices_seen: self.pending_choices_seen,
                pending_choice_actions_simulated: self.pending_choice_actions_simulated,
                max_pending_choice_candidate_count: self.max_pending_choice_candidate_count,
                max_pending_choice_estimated_action_fanout: self
                    .max_pending_choice_estimated_action_fanout,
                last_pending_choice_kind: self.last_pending_choice_kind,
                stopped_on_high_fanout_pending_choice: self.stopped_on_high_fanout_pending_choice,
                survival_margin: self.survival_margin,
                actions_simulated: self.actions_simulated,
                truncated: self.truncated,
                stop_reason: self.stop_reason.label(),
                last_action_reason: self.last_action_reason,
            })
    }
}

impl RolloutStopReason {
    pub(super) fn label(self) -> &'static str {
        match self {
            RolloutStopReason::NotEvaluated => "not_evaluated",
            RolloutStopReason::TerminalState => "terminal_state",
            RolloutStopReason::MaxActions => "max_actions",
            RolloutStopReason::Deadline => "deadline",
            RolloutStopReason::NoLegalActions => "no_legal_actions",
            RolloutStopReason::PolicyDeclined => "policy_declined",
            RolloutStopReason::EngineStepLimit => "engine_step_limit",
            RolloutStopReason::HighFanoutPendingChoice => "high_fanout_pending_choice",
        }
    }

    fn is_truncated(self) -> bool {
        !matches!(
            self,
            RolloutStopReason::NotEvaluated | RolloutStopReason::TerminalState
        )
    }
}
