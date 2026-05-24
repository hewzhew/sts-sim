use super::*;

pub(super) const DEFAULT_ROLLOUT_MAX_EVALUATIONS: usize = 384;
pub(super) const DEFAULT_ROLLOUT_MAX_ACTIONS: usize = 80;

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
    pub(super) survival_margin: i32,
    pub(super) actions_simulated: usize,
    pub(super) truncated: bool,
    pub(super) stop_reason: RolloutStopReason,
    pub(super) last_action_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct RolloutCache {
    policy: CombatSearchV2RolloutPolicy,
    max_evaluations: usize,
    max_actions: usize,
    evaluations: u64,
    cache_hits: u64,
    budget_skips: u64,
    truncated: u64,
    terminal_wins: u64,
    terminal_losses: u64,
    cache: HashMap<CombatExactStateKey, RolloutNodeEstimate>,
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
            survival_margin: 0,
            actions_simulated: 0,
            truncated: false,
            stop_reason: RolloutStopReason::NotEvaluated,
            last_action_reason: None,
        }
    }

    fn from_node(
        node: &SearchNode,
        actions_simulated: usize,
        stop_reason: RolloutStopReason,
        last_action_reason: Option<&'static str>,
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

    fn to_report(self) -> Option<CombatSearchV2RolloutEstimateReport> {
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
                survival_margin: self.survival_margin,
                actions_simulated: self.actions_simulated,
                truncated: self.truncated,
                stop_reason: self.stop_reason.label(),
                last_action_reason: self.last_action_reason,
            })
    }
}

impl RolloutStopReason {
    fn label(self) -> &'static str {
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

impl RolloutCache {
    pub(super) fn new(
        policy: CombatSearchV2RolloutPolicy,
        max_evaluations: usize,
        max_actions: usize,
    ) -> Self {
        Self {
            policy,
            max_evaluations,
            max_actions,
            ..Self::default()
        }
    }

    pub(super) fn estimate(
        &mut self,
        node: &SearchNode,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
        deadline: Option<Instant>,
    ) -> RolloutNodeEstimate {
        if self.policy == CombatSearchV2RolloutPolicy::Disabled {
            return RolloutNodeEstimate::unevaluated();
        }

        let key = combat_exact_state_key(&node.engine, &node.combat);
        if let Some(cached) = self.cache.get(&key).copied() {
            self.cache_hits = self.cache_hits.saturating_add(1);
            return cached;
        }
        if self.evaluations as usize >= self.max_evaluations {
            self.budget_skips = self.budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            self.budget_skips = self.budget_skips.saturating_add(1);
            return RolloutNodeEstimate::unevaluated();
        }

        self.evaluations = self.evaluations.saturating_add(1);
        let estimate = match self.policy {
            CombatSearchV2RolloutPolicy::Disabled => RolloutNodeEstimate::unevaluated(),
            CombatSearchV2RolloutPolicy::ConservativeNoPotion => {
                conservative_no_potion_rollout(node, stepper, config, self.max_actions, deadline)
            }
        };
        if estimate.truncated {
            self.truncated = self.truncated.saturating_add(1);
        }
        match estimate.terminal {
            SearchTerminalLabel::Win => self.terminal_wins = self.terminal_wins.saturating_add(1),
            SearchTerminalLabel::Loss => {
                self.terminal_losses = self.terminal_losses.saturating_add(1)
            }
            SearchTerminalLabel::Unresolved => {}
        }
        self.cache.insert(key, estimate);
        estimate
    }

    pub(super) fn finish(&self, best_frontier: Option<&SearchNode>) -> CombatSearchV2RolloutReport {
        CombatSearchV2RolloutReport {
            policy: self.policy.label(),
            behavioral_effect:
                "estimated_frontier_priority_only_no_terminal_proof_no_baseline_claim",
            max_evaluations: self.max_evaluations,
            max_actions_per_rollout: self.max_actions,
            evaluations: self.evaluations,
            cache_hits: self.cache_hits,
            budget_skips: self.budget_skips,
            truncated_rollouts: self.truncated,
            terminal_wins: self.terminal_wins,
            terminal_losses: self.terminal_losses,
            best_frontier_estimate: best_frontier
                .and_then(|node| node.rollout_estimate.to_report()),
            notes: vec![
                "rollout estimates are not terminal proof",
                "conservative_no_potion uses only legal simulator actions and disables potion actions",
                "rollout cache is keyed by exact combat runtime state",
                "unresolved rollout priority uses phase-adjusted enemy effort from phase_profile",
                "high-fanout pending choices stop rollout estimates instead of selecting an arbitrary branch",
            ],
        }
    }
}

fn conservative_no_potion_rollout(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    max_actions: usize,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    let mut rollout = node.clone_for_rollout();
    let mut last_action_reason = None;
    for actions_simulated in 0..=max_actions {
        if terminal_label(&rollout.engine, &rollout.combat) != SearchTerminalLabel::Unresolved {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::TerminalState,
                last_action_reason,
            );
        }
        if actions_simulated == max_actions {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::MaxActions,
                last_action_reason,
            );
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::Deadline,
                last_action_reason,
            );
        }
        let phase_profile = combat_search_phase_profile(&rollout.engine, &rollout.combat);
        if phase_profile.pending_choice.high_fanout {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::HighFanoutPendingChoice,
                last_action_reason,
            );
        }

        let position = CombatPosition::new(rollout.engine.clone(), rollout.combat.clone());
        let legal = filtered_rollout_legal_actions(
            CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            stepper.legal_action_choices(&position),
            &rollout.combat,
        );
        if legal.is_empty() {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::NoLegalActions,
                last_action_reason,
            );
        }

        let Some(selection) = choose_rollout_action(
            CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            &rollout,
            stepper,
            config,
            deadline,
            &rollout.engine,
            &rollout.combat,
            legal,
        ) else {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated,
                RolloutStopReason::PolicyDeclined,
                last_action_reason,
            );
        };
        last_action_reason = Some(selection.reason);
        let choice = selection.choice;

        let step = stepper.apply_to_stable(
            &position,
            choice.choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline,
            },
        );
        let mut child = rollout.clone_for_child(step.position.engine, step.position.combat);
        child.note_input(&choice.choice.input);
        child.actions.push(CombatSearchV2ActionTrace {
            step_index: rollout.actions.len(),
            action_id: choice.original_action_id,
            action_key: choice.choice.action_key,
            action_debug: choice.choice.action_debug,
            input: choice.choice.input,
        });
        rollout = child;

        if step.truncated {
            return RolloutNodeEstimate::from_node(
                &rollout,
                actions_simulated + 1,
                RolloutStopReason::EngineStepLimit,
                last_action_reason,
            );
        }
    }

    RolloutNodeEstimate::from_node(
        &rollout,
        max_actions,
        RolloutStopReason::MaxActions,
        last_action_reason,
    )
}

impl SearchNode {
    fn clone_for_rollout(&self) -> Self {
        let mut clone = self.clone();
        clone.rollout_estimate = RolloutNodeEstimate::unevaluated();
        clone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::test_support::{blank_test_combat, test_monster};

    #[derive(Clone, Copy)]
    struct FirstActionWinsStepper;

    impl CombatStepper for FirstActionWinsStepper {
        fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
            vec![ClientInput::EndTurn]
        }

        fn apply_to_stable(
            &self,
            position: &CombatPosition,
            input: ClientInput,
            _limits: CombatStepLimits,
        ) -> crate::sim::combat::CombatStepResult {
            let engine = if matches!(input, ClientInput::EndTurn) {
                EngineState::GameOver(crate::state::core::RunResult::Victory)
            } else {
                position.engine.clone()
            };
            let position = CombatPosition::new(engine, position.combat.clone());
            crate::sim::combat::CombatStepResult {
                terminal: combat_terminal(&position.engine, &position.combat),
                alive: true,
                truncated: false,
                timed_out: false,
                engine_steps: 1,
                position,
            }
        }

        fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
            combat_terminal(&position.engine, &position.combat)
        }
    }

    #[test]
    fn conservative_rollout_records_estimated_terminal_win() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        let node = SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat,
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        };
        let config = CombatSearchV2Config::default();

        let estimate =
            conservative_no_potion_rollout(&node, &FirstActionWinsStepper, &config, 4, None);

        assert!(estimate.evaluated);
        assert_eq!(estimate.terminal, SearchTerminalLabel::Win);
        assert!(!estimate.truncated);
    }

    #[test]
    fn rollout_cache_reuses_exact_state_estimate() {
        let mut cache = RolloutCache::new(CombatSearchV2RolloutPolicy::ConservativeNoPotion, 4, 4);
        let node = SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat: blank_test_combat(),
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        };
        let config = CombatSearchV2Config::default();

        let first = cache.estimate(&node, &FirstActionWinsStepper, &config, None);
        let second = cache.estimate(&node, &FirstActionWinsStepper, &config, None);

        assert_eq!(first, second);
        assert_eq!(cache.evaluations, 1);
        assert_eq!(cache.cache_hits, 1);
    }

    #[test]
    fn rollout_estimate_records_phase_adjusted_enemy_effort() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.id = 1;
        guardian.current_hp = 180;
        guardian.max_hp = 240;
        guardian.block = 20;
        guardian.guardian.is_open = false;
        combat.entities.monsters = vec![guardian];
        let node = SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat,
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        };

        let estimate =
            RolloutNodeEstimate::from_node(&node, 0, RolloutStopReason::MaxActions, None);

        assert_eq!(estimate.total_enemy_hp, 180);
        assert_eq!(estimate.total_enemy_block, 20);
        assert_eq!(estimate.phase_adjusted_enemy_effort, 200);
    }

    #[test]
    fn conservative_rollout_stops_before_large_pending_choice_branch() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        let node = SearchNode {
            engine: EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
                cards: vec![crate::content::cards::CardId::Strike; 7],
                card_uuids: (0..7).collect(),
            }),
            combat,
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        };
        let config = CombatSearchV2Config::default();

        let estimate =
            conservative_no_potion_rollout(&node, &FirstActionWinsStepper, &config, 4, None);

        assert!(estimate.evaluated);
        assert!(estimate.truncated);
        assert_eq!(
            estimate.stop_reason,
            RolloutStopReason::HighFanoutPendingChoice
        );
        assert!(estimate.high_fanout_pending_choice);
        assert_eq!(estimate.pending_choice_estimated_action_fanout, 128);
        assert_eq!(estimate.actions_simulated, 0);
    }
}
