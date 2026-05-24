use super::action_effects::PlayCardEffectDiagnostics;
use super::action_priority::{priority_for_input, ActionOrderingPriority, ActionOrderingRole};
use super::*;
use std::collections::BTreeMap;

const LARGEST_REORDER_SAMPLE_LIMIT: usize = 8;
const ACTION_EFFECT_SAMPLE_LIMIT: usize = 12;

#[derive(Clone, Debug)]
pub(super) struct IndexedActionChoice {
    pub(super) original_action_id: usize,
    pub(super) choice: CombatActionChoice,
}

pub(super) type OrderedActionChoice = IndexedActionChoice;

#[derive(Clone, Debug)]
pub(super) struct ActionOrderingResult {
    pub(super) choices: Vec<OrderedActionChoice>,
    pub(super) summary: ActionOrderingSummary,
}

#[derive(Clone, Debug)]
pub(super) struct ActionOrderingSummary {
    action_count: usize,
    max_position_shift: usize,
    role_counts: BTreeMap<ActionOrderingRole, usize>,
    first_role: Option<ActionOrderingRole>,
    first_original_action_id: Option<usize>,
    first_action_key: Option<String>,
    phase_signal_actions: usize,
    action_effect_samples: Vec<ActionOrderingActionEffectSummary>,
}

#[derive(Default)]
pub(super) struct ActionOrderingDiagnosticsCollector {
    states_observed: u64,
    states_reordered: u64,
    total_actions_observed: u64,
    total_position_shift: u64,
    max_position_shift: usize,
    role_counts: BTreeMap<ActionOrderingRole, MutableOrderingRoleCount>,
    largest_reorders: Vec<ActionOrderingObservation>,
    action_effect_actions: u64,
    phase_action_hint_actions: u64,
    action_effect_samples: Vec<ActionOrderingActionEffectObservation>,
}

#[derive(Clone, Debug)]
struct ActionOrderingEntry {
    original_action_id: usize,
    choice: CombatActionChoice,
    priority: ActionOrderingPriority,
}

#[derive(Clone, Debug, Default)]
struct MutableOrderingRoleCount {
    actions: u64,
    first_actions: u64,
}

#[derive(Clone, Debug)]
struct ActionOrderingObservation {
    observed_at_state_query: u64,
    action_count: usize,
    max_position_shift: usize,
    first_role: ActionOrderingRole,
    first_original_action_id: usize,
    first_action_key: String,
}

#[derive(Clone, Debug)]
struct ActionOrderingActionEffectSummary {
    original_action_id: usize,
    ordered_index: usize,
    role: ActionOrderingRole,
    action_key: String,
    effects: PlayCardEffectDiagnostics,
}

#[derive(Clone, Debug)]
struct ActionOrderingActionEffectObservation {
    observed_at_state_query: u64,
    original_action_id: usize,
    ordered_index: usize,
    role: ActionOrderingRole,
    action_key: String,
    effects: PlayCardEffectDiagnostics,
}

impl ActionOrderingSummary {
    pub(super) fn action_count(&self) -> usize {
        self.action_count
    }

    pub(super) fn first_role(&self) -> Option<ActionOrderingRole> {
        self.first_role
    }

    pub(super) fn role_counts(&self) -> impl Iterator<Item = (ActionOrderingRole, usize)> + '_ {
        self.role_counts.iter().map(|(role, count)| (*role, *count))
    }
}

#[cfg(test)]
fn order_action_choices(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<CombatActionChoice>,
) -> ActionOrderingResult {
    order_indexed_action_choices(
        engine,
        combat,
        choices
            .into_iter()
            .enumerate()
            .map(|(original_action_id, choice)| IndexedActionChoice {
                original_action_id,
                choice,
            })
            .collect(),
    )
}

pub(super) fn order_indexed_action_choices(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<IndexedActionChoice>,
) -> ActionOrderingResult {
    let mut entries = choices
        .into_iter()
        .map(|indexed| ActionOrderingEntry {
            original_action_id: indexed.original_action_id,
            priority: priority_for_input(engine, combat, &indexed.choice.input),
            choice: indexed.choice,
        })
        .collect::<Vec<_>>();

    if action_ordering_enabled(engine) {
        entries.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.original_action_id.cmp(&right.original_action_id))
        });
    }

    let summary = summarize_ordering(&entries);
    let choices = entries
        .into_iter()
        .map(|entry| IndexedActionChoice {
            original_action_id: entry.original_action_id,
            choice: entry.choice,
        })
        .collect();

    ActionOrderingResult { choices, summary }
}

fn action_ordering_enabled(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    )
}

fn summarize_ordering(entries: &[ActionOrderingEntry]) -> ActionOrderingSummary {
    let mut role_counts = BTreeMap::new();
    let mut max_position_shift = 0usize;
    let mut phase_signal_actions = 0usize;
    let mut action_effect_samples = Vec::new();
    for (ordered_index, entry) in entries.iter().enumerate() {
        *role_counts.entry(entry.priority.role).or_insert(0) += 1;
        max_position_shift =
            max_position_shift.max(entry.original_action_id.abs_diff(ordered_index));
        if entry.priority.phase_hint.has_signal() {
            phase_signal_actions += 1;
        }
        if entry.priority.effects.has_reactive_signal() {
            action_effect_samples.push(ActionOrderingActionEffectSummary {
                original_action_id: entry.original_action_id,
                ordered_index,
                role: entry.priority.role,
                action_key: entry.choice.action_key.clone(),
                effects: entry.priority.effects,
            });
        }
    }

    ActionOrderingSummary {
        action_count: entries.len(),
        max_position_shift,
        role_counts,
        first_role: entries.first().map(|entry| entry.priority.role),
        first_original_action_id: entries.first().map(|entry| entry.original_action_id),
        first_action_key: entries.first().map(|entry| entry.choice.action_key.clone()),
        phase_signal_actions,
        action_effect_samples,
    }
}

impl ActionOrderingDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &ActionOrderingSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.total_actions_observed = self
            .total_actions_observed
            .saturating_add(summary.action_count as u64);
        self.total_position_shift = self
            .total_position_shift
            .saturating_add(summary.max_position_shift as u64);
        self.max_position_shift = self.max_position_shift.max(summary.max_position_shift);
        if summary.max_position_shift > 0 {
            self.states_reordered = self.states_reordered.saturating_add(1);
        }
        self.phase_action_hint_actions = self
            .phase_action_hint_actions
            .saturating_add(summary.phase_signal_actions as u64);

        for (role, count) in &summary.role_counts {
            let mutable = self.role_counts.entry(*role).or_default();
            mutable.actions = mutable.actions.saturating_add(*count as u64);
        }
        if let Some(first_role) = summary.first_role {
            self.role_counts
                .entry(first_role)
                .or_default()
                .first_actions += 1;
        }

        if let (Some(first_role), Some(first_original_action_id), Some(first_action_key)) = (
            summary.first_role,
            summary.first_original_action_id,
            summary.first_action_key.as_ref(),
        ) {
            self.remember_largest_reorder(ActionOrderingObservation {
                observed_at_state_query: self.states_observed,
                action_count: summary.action_count,
                max_position_shift: summary.max_position_shift,
                first_role,
                first_original_action_id,
                first_action_key: first_action_key.clone(),
            });
        }
        for sample in &summary.action_effect_samples {
            self.action_effect_actions = self.action_effect_actions.saturating_add(1);
            self.remember_action_effect(ActionOrderingActionEffectObservation {
                observed_at_state_query: self.states_observed,
                original_action_id: sample.original_action_id,
                ordered_index: sample.ordered_index,
                role: sample.role,
                action_key: sample.action_key.clone(),
                effects: sample.effects,
            });
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsOrdering {
        CombatSearchV2DiagnosticsOrdering {
            ordering_policy:
                "semantic_role_ordering_for_player_turn_and_pending_choice_boundaries",
            behavioral_effect: "child_generation_order_only_no_prune_no_merge",
            states_observed: self.states_observed,
            states_reordered: self.states_reordered,
            reordered_state_ratio: rounded_ratio(self.states_reordered, self.states_observed),
            total_actions_observed: self.total_actions_observed,
            action_effect_actions: self.action_effect_actions,
            phase_action_hint_actions: self.phase_action_hint_actions,
            max_position_shift: self.max_position_shift,
            avg_position_shift: rounded_ratio(self.total_position_shift, self.states_observed),
            action_role_counts: self.action_role_counts(),
            largest_reorders: self.largest_reorder_samples(),
            action_effect_samples: self.action_effect_samples(),
            notes: vec![
                "ordering diagnostics summarize which semantic roles are explored first",
                "original action ids are preserved in action traces after ordering",
                "a reorder sample is kept only when action order changed",
                "ordering does not remove legal actions or prove action equivalence",
                "reactive power risk is derived from simulator power hooks, not monster-name policy",
                "enemy phase transition hints only reorder children and never suppress phase-triggering actions",
                "phase action hints reuse phase_profile and only add ordering tiebreaks",
                "pending choice ordering uses typed selection facts and never drops alternatives",
            ],
        }
    }

    fn remember_largest_reorder(&mut self, observation: ActionOrderingObservation) {
        if observation.max_position_shift == 0 {
            return;
        }
        self.largest_reorders.push(observation);
        self.largest_reorders.sort_by(|left, right| {
            right
                .max_position_shift
                .cmp(&left.max_position_shift)
                .then_with(|| right.action_count.cmp(&left.action_count))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_reorders.truncate(LARGEST_REORDER_SAMPLE_LIMIT);
    }

    fn remember_action_effect(&mut self, observation: ActionOrderingActionEffectObservation) {
        self.action_effect_samples.push(observation);
        self.action_effect_samples.sort_by(|left, right| {
            right
                .effects
                .reactive_risk_score
                .cmp(&left.effects.reactive_risk_score)
                .then_with(|| {
                    right
                        .effects
                        .mitigation_score
                        .cmp(&left.effects.mitigation_score)
                })
                .then_with(|| {
                    right
                        .effects
                        .reactive_enemy_damage
                        .cmp(&left.effects.reactive_enemy_damage)
                })
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
                .then_with(|| left.ordered_index.cmp(&right.ordered_index))
        });
        self.action_effect_samples
            .truncate(ACTION_EFFECT_SAMPLE_LIMIT);
    }

    fn action_role_counts(&self) -> Vec<CombatSearchV2DiagnosticsActionRoleCount> {
        self.role_counts
            .iter()
            .map(|(role, count)| CombatSearchV2DiagnosticsActionRoleCount {
                role: role.label().to_string(),
                actions: count.actions,
                first_actions: count.first_actions,
            })
            .collect()
    }

    fn largest_reorder_samples(&self) -> Vec<CombatSearchV2DiagnosticsOrderingSample> {
        self.largest_reorders
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsOrderingSample {
                observed_at_state_query: sample.observed_at_state_query,
                action_count: sample.action_count,
                max_position_shift: sample.max_position_shift,
                first_role: sample.first_role.label().to_string(),
                first_original_action_id: sample.first_original_action_id,
                first_action_key: sample.first_action_key.clone(),
            })
            .collect()
    }

    fn action_effect_samples(&self) -> Vec<CombatSearchV2DiagnosticsActionEffectSample> {
        self.action_effect_samples
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsActionEffectSample {
                observed_at_state_query: sample.observed_at_state_query,
                original_action_id: sample.original_action_id,
                ordered_index: sample.ordered_index,
                role: sample.role.label().to_string(),
                action_key: sample.action_key.clone(),
                mitigation_score: sample.effects.mitigation_score,
                reactive_risk_score: sample.effects.reactive_risk_score,
                enemy_strength_gain: sample.effects.enemy_strength_gain,
                visible_attack_pressure_hint: sample.effects.visible_attack_pressure_hint,
                reactive_player_hp_loss: sample.effects.reactive_player_hp_loss,
                reactive_player_block: sample.effects.reactive_player_block,
                reactive_enemy_damage: sample.effects.reactive_enemy_damage,
                reactive_bad_draw_cards: sample.effects.reactive_bad_draw_cards,
                reactive_forced_turn_end: sample.effects.reactive_forced_turn_end,
            })
            .collect()
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn combat_ordering_keeps_original_action_ids_after_reordering() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.current_hp = 6;
        monster.max_hp = 6;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 1);
        assert!(matches!(
            ordered.choices[0].choice.input,
            ClientInput::PlayCard { .. }
        ));
        assert_eq!(ordered.choices[1].original_action_id, 0);
        assert_eq!(ordered.summary.max_position_shift, 1);
        assert_eq!(
            ordered.summary.first_role,
            Some(ActionOrderingRole::LethalCard)
        );
    }

    #[test]
    fn lethal_card_is_ordered_before_nonlethal_damage() {
        let mut combat = blank_test_combat();
        let mut low_hp = test_monster(EnemyId::LouseNormal);
        low_hp.current_hp = 6;
        low_hp.max_hp = 6;
        low_hp.id = 1;
        let mut high_hp = test_monster(EnemyId::JawWorm);
        high_hp.current_hp = 30;
        high_hp.max_hp = 30;
        high_hp.id = 2;
        combat.entities.monsters = vec![low_hp, high_hp];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(2),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 1);
    }

    #[test]
    fn block_that_prevents_visible_lethal_is_ordered_before_damage() {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = 5;
        combat.entities.player.block = 0;
        let mut monster = test_monster(EnemyId::Cultist);
        monster.set_planned_move_id(1);
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: None,
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 1);
    }

    #[test]
    fn persistent_enemy_strength_down_is_ordered_before_plain_damage() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.id = 1;
        monster.current_hp = 50;
        monster.max_hp = 50;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Disarm, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 1);
        assert_eq!(
            ordered.summary.first_role,
            Some(ActionOrderingRole::SustainedMitigation)
        );
    }

    #[test]
    fn reactive_enemy_scaling_risk_orders_damage_before_nonlethal_skill_block() {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = 80;
        let mut nob = test_monster(EnemyId::GremlinNob);
        nob.id = 1;
        nob.current_hp = 83;
        nob.max_hp = 83;
        nob.set_planned_move_id(1);
        combat.entities.monsters = vec![nob];
        combat.entities.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::Anger,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::Defend, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert!(matches!(
            ordered.choices[0].choice.input,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1)
            }
        ));
        assert_eq!(
            ordered.summary.first_role,
            Some(ActionOrderingRole::DamageProgress)
        );
    }

    #[test]
    fn split_trigger_risk_orders_safe_damage_first_without_dropping_actions() {
        let mut combat = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 1;
        slime.current_hp = 40;
        slime.max_hp = 65;
        combat.entities.monsters = vec![slime];
        combat.entities.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::Split,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::Carnage, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let choices = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ];

        let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

        assert_eq!(ordered.choices.len(), 2);
        assert_eq!(ordered.choices[0].original_action_id, 1);
        assert_eq!(ordered.choices[1].original_action_id, 0);
    }

    #[test]
    fn non_player_turn_choices_keep_existing_order() {
        let combat = blank_test_combat();
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::Proceed),
            CombatActionChoice::from_input(&combat, ClientInput::Cancel),
        ];

        let ordered = order_action_choices(&EngineState::CombatProcessing, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 0);
        assert_eq!(ordered.choices[1].original_action_id, 1);
        assert_eq!(ordered.summary.max_position_shift, 0);
    }

    #[test]
    fn pending_choice_actions_are_ordered_without_losing_original_ids() {
        let mut combat = blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Carnage, 20),
        ];
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::core::GridSelectReason::MoveToDrawPile,
        });
        let choices = vec![
            CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![10])),
            CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![20])),
        ];

        let ordered = order_action_choices(&engine, &combat, choices);

        assert_eq!(ordered.choices[0].original_action_id, 1);
        assert_eq!(
            ordered.summary.first_role,
            Some(ActionOrderingRole::PendingChoiceValueSelection)
        );
        assert_eq!(ordered.summary.max_position_shift, 1);
    }

    #[test]
    fn ordering_collector_reports_role_counts_without_action_tree() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.current_hp = 6;
        monster.max_hp = 6;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let ordered = order_action_choices(
            &EngineState::CombatPlayerTurn,
            &combat,
            vec![
                CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
                CombatActionChoice::from_input(
                    &combat,
                    ClientInput::PlayCard {
                        card_index: 0,
                        target: Some(1),
                    },
                ),
            ],
        );
        let mut collector = ActionOrderingDiagnosticsCollector::default();

        collector.observe(&ordered.summary);
        let report = collector.finish();

        assert_eq!(
            report.behavioral_effect,
            "child_generation_order_only_no_prune_no_merge"
        );
        assert_eq!(report.states_reordered, 1);
        assert_eq!(report.max_position_shift, 1);
        assert_eq!(report.largest_reorders.len(), 1);
        assert_eq!(report.largest_reorders[0].first_role, "lethal_card");
    }

    #[test]
    fn ordering_collector_caps_action_effect_samples() {
        let mut combat = blank_test_combat();
        let mut nob = test_monster(EnemyId::GremlinNob);
        nob.id = 1;
        combat.entities.monsters = vec![nob];
        combat.entities.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::Anger,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::Defend, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let ordered = order_action_choices(
            &EngineState::CombatPlayerTurn,
            &combat,
            vec![
                CombatActionChoice::from_input(
                    &combat,
                    ClientInput::PlayCard {
                        card_index: 0,
                        target: None,
                    },
                ),
                CombatActionChoice::from_input(
                    &combat,
                    ClientInput::PlayCard {
                        card_index: 1,
                        target: Some(1),
                    },
                ),
            ],
        );
        let mut collector = ActionOrderingDiagnosticsCollector::default();

        for _ in 0..(ACTION_EFFECT_SAMPLE_LIMIT + 3) {
            collector.observe(&ordered.summary);
        }
        let report = collector.finish();

        assert_eq!(
            report.action_effect_actions,
            (ACTION_EFFECT_SAMPLE_LIMIT + 3) as u64
        );
        assert_eq!(
            report.action_effect_samples.len(),
            ACTION_EFFECT_SAMPLE_LIMIT
        );
        assert!(report.action_effect_samples[0].enemy_strength_gain > 0);
        assert!(report.action_effect_samples[0].reactive_risk_score > 0);
    }

    #[test]
    fn ordering_collector_counts_phase_hint_actions() {
        let mut combat = blank_test_combat();
        let mut lagavulin = test_monster(EnemyId::Lagavulin);
        lagavulin.id = 1;
        lagavulin.lagavulin.is_out = false;
        combat.entities.monsters = vec![lagavulin];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let ordered = order_action_choices(
            &EngineState::CombatPlayerTurn,
            &combat,
            vec![CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            )],
        );
        let mut collector = ActionOrderingDiagnosticsCollector::default();

        collector.observe(&ordered.summary);
        let report = collector.finish();

        assert_eq!(report.phase_action_hint_actions, 1);
    }
}
