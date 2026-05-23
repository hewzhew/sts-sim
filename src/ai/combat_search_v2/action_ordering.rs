use super::*;
use crate::content::cards::{self, CardTarget, CardType};
use std::cmp::Ordering;
use std::collections::BTreeMap;

const LARGEST_REORDER_SAMPLE_LIMIT: usize = 8;

// These ranks only decide child-generation order inside the same legal action set.
// They never merge, prune, or claim that two actions are equivalent.
const ROLE_LETHAL_CARD: i32 = 130;
const ROLE_PREVENT_VISIBLE_LETHAL: i32 = 120;
const ROLE_TACTICAL_POTION_BASE: i32 = 60;
const ROLE_PREVENT_HP_LOSS: i32 = 85;
const ROLE_DEFERRED_SETUP: i32 = 75;
const ROLE_DAMAGE_PROGRESS: i32 = 60;
const ROLE_BLOCK: i32 = 45;
const ROLE_UTILITY_PLAY: i32 = 35;
const ROLE_END_TURN: i32 = 0;
const ROLE_DISCARD_POTION: i32 = -20;

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
}

#[derive(Clone, Debug)]
struct ActionOrderingEntry {
    original_action_id: usize,
    choice: CombatActionChoice,
    priority: ActionOrderingPriority,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActionOrderingPriority {
    role: ActionOrderingRole,
    role_rank: i32,
    potion_tactical_rank: i32,
    target_progress: i32,
    block: i32,
    damage: i32,
    cheaper_cost: i32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActionOrderingRole {
    LethalCard,
    PreventVisibleLethal,
    TacticalPotion,
    PreventHpLoss,
    DeferredSetup,
    DamageProgress,
    Block,
    UtilityPlay,
    EndTurn,
    DiscardPotion,
    Neutral,
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

    if matches!(engine, EngineState::CombatPlayerTurn) {
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

fn priority_for_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> ActionOrderingPriority {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return ActionOrderingPriority::neutral(ActionOrderingRole::Neutral);
    }

    match input {
        ClientInput::PlayCard { card_index, target } => {
            priority_for_play_card(combat, *card_index, *target)
        }
        ClientInput::UsePotion { .. } => {
            let potion_rank =
                potions::semantic_potion_tactical_priority(combat, input).unwrap_or_default();
            ActionOrderingPriority {
                role: ActionOrderingRole::TacticalPotion,
                role_rank: ROLE_TACTICAL_POTION_BASE + potion_rank,
                potion_tactical_rank: potion_rank,
                ..ActionOrderingPriority::neutral(ActionOrderingRole::TacticalPotion)
            }
        }
        ClientInput::DiscardPotion(_) => ActionOrderingPriority {
            role: ActionOrderingRole::DiscardPotion,
            role_rank: ROLE_DISCARD_POTION,
            ..ActionOrderingPriority::neutral(ActionOrderingRole::DiscardPotion)
        },
        ClientInput::EndTurn => ActionOrderingPriority {
            role: ActionOrderingRole::EndTurn,
            role_rank: ROLE_END_TURN,
            ..ActionOrderingPriority::neutral(ActionOrderingRole::EndTurn)
        },
        _ => ActionOrderingPriority {
            role: ActionOrderingRole::UtilityPlay,
            role_rank: ROLE_UTILITY_PLAY,
            ..ActionOrderingPriority::neutral(ActionOrderingRole::UtilityPlay)
        },
    }
}

fn priority_for_play_card(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
) -> ActionOrderingPriority {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return ActionOrderingPriority::neutral(ActionOrderingRole::Neutral);
    };

    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let def = cards::get_card_definition(card.id);
    let target_kind = cards::effective_target(card);
    let damage = evaluated.base_damage_mut.max(0);
    let block = evaluated.base_block_mut.max(0);
    let target_progress = target_progress_hint(combat, target_kind, target, damage);
    let visible_damage = visible_incoming_damage(combat);
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block = (visible_damage - current_block - block).max(0);
    let prevents_visible_lethal =
        visible_loss_now >= current_hp && visible_loss_after_block < current_hp;
    let prevents_hp_loss = visible_loss_after_block < visible_loss_now;
    let (role, role_rank) = if target_progress_kills(combat, target_kind, target, damage) {
        (ActionOrderingRole::LethalCard, ROLE_LETHAL_CARD)
    } else if prevents_visible_lethal {
        (
            ActionOrderingRole::PreventVisibleLethal,
            ROLE_PREVENT_VISIBLE_LETHAL,
        )
    } else if def.card_type == CardType::Power {
        (ActionOrderingRole::DeferredSetup, ROLE_DEFERRED_SETUP)
    } else if prevents_hp_loss {
        (ActionOrderingRole::PreventHpLoss, ROLE_PREVENT_HP_LOSS)
    } else if target_progress > 0 {
        (ActionOrderingRole::DamageProgress, ROLE_DAMAGE_PROGRESS)
    } else if block > 0 {
        (ActionOrderingRole::Block, ROLE_BLOCK)
    } else {
        (ActionOrderingRole::UtilityPlay, ROLE_UTILITY_PLAY)
    };

    ActionOrderingPriority {
        role,
        role_rank,
        target_progress,
        block,
        damage,
        cheaper_cost: -card.cost_for_turn_java().max(0),
        ..ActionOrderingPriority::neutral(role)
    }
}

fn target_progress_hint(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> i32 {
    if damage <= 0 {
        return 0;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
            .sum(),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .map(|hp| damage.min(hp).max(0))
            .unwrap_or_default(),
        _ => 0,
    }
}

fn target_progress_kills(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> bool {
    if damage <= 0 {
        return false;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .any(|monster| damage >= monster.current_hp + monster.block),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .is_some_and(|hp| damage >= hp),
        _ => false,
    }
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

impl ActionOrderingPriority {
    fn neutral(role: ActionOrderingRole) -> Self {
        Self {
            role,
            role_rank: ROLE_END_TURN,
            potion_tactical_rank: 0,
            target_progress: 0,
            block: 0,
            damage: 0,
            cheaper_cost: 0,
        }
    }
}

fn summarize_ordering(entries: &[ActionOrderingEntry]) -> ActionOrderingSummary {
    let mut role_counts = BTreeMap::new();
    let mut max_position_shift = 0usize;
    for (ordered_index, entry) in entries.iter().enumerate() {
        *role_counts.entry(entry.priority.role).or_insert(0) += 1;
        max_position_shift =
            max_position_shift.max(entry.original_action_id.abs_diff(ordered_index));
    }

    ActionOrderingSummary {
        action_count: entries.len(),
        max_position_shift,
        role_counts,
        first_role: entries.first().map(|entry| entry.priority.role),
        first_original_action_id: entries.first().map(|entry| entry.original_action_id),
        first_action_key: entries.first().map(|entry| entry.choice.action_key.clone()),
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
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsOrdering {
        CombatSearchV2DiagnosticsOrdering {
            ordering_policy: "semantic_role_ordering_for_combat_player_turn_only",
            behavioral_effect: "child_generation_order_only_no_prune_no_merge",
            states_observed: self.states_observed,
            states_reordered: self.states_reordered,
            reordered_state_ratio: rounded_ratio(self.states_reordered, self.states_observed),
            total_actions_observed: self.total_actions_observed,
            max_position_shift: self.max_position_shift,
            avg_position_shift: rounded_ratio(self.total_position_shift, self.states_observed),
            action_role_counts: self.action_role_counts(),
            largest_reorders: self.largest_reorder_samples(),
            notes: vec![
                "ordering diagnostics summarize which semantic roles are explored first",
                "original action ids are preserved in action traces after ordering",
                "a reorder sample is kept only when action order changed",
                "ordering does not remove legal actions or prove action equivalence",
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
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

impl ActionOrderingRole {
    fn label(self) -> &'static str {
        match self {
            ActionOrderingRole::LethalCard => "lethal_card",
            ActionOrderingRole::PreventVisibleLethal => "prevent_visible_lethal",
            ActionOrderingRole::TacticalPotion => "tactical_potion",
            ActionOrderingRole::PreventHpLoss => "prevent_hp_loss",
            ActionOrderingRole::DeferredSetup => "deferred_setup",
            ActionOrderingRole::DamageProgress => "damage_progress",
            ActionOrderingRole::Block => "block",
            ActionOrderingRole::UtilityPlay => "utility_play",
            ActionOrderingRole::EndTurn => "end_turn",
            ActionOrderingRole::DiscardPotion => "discard_potion",
            ActionOrderingRole::Neutral => "neutral",
        }
    }
}

impl Ord for ActionOrderingPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.role_rank
            .cmp(&other.role_rank)
            .then_with(|| self.potion_tactical_rank.cmp(&other.potion_tactical_rank))
            .then_with(|| self.target_progress.cmp(&other.target_progress))
            .then_with(|| self.block.cmp(&other.block))
            .then_with(|| self.damage.cmp(&other.damage))
            .then_with(|| self.cheaper_cost.cmp(&other.cheaper_cost))
    }
}

impl PartialOrd for ActionOrderingPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
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
}
