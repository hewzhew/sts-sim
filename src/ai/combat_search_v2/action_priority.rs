use super::action_effects::{summarize_play_card_effects, PlayCardEffectDiagnostics};
use super::*;
use crate::content::cards::{self, CardTarget, CardType};
use std::cmp::Ordering;

// These ranks only decide child-generation order inside the same legal action set.
// They never merge, prune, or claim that two actions are equivalent.
const ROLE_LETHAL_CARD: i32 = 130;
const ROLE_PREVENT_VISIBLE_LETHAL: i32 = 120;
const ROLE_SUSTAINED_MITIGATION: i32 = 95;
const ROLE_TACTICAL_POTION_BASE: i32 = 60;
const ROLE_PREVENT_HP_LOSS: i32 = 85;
const ROLE_DEFERRED_SETUP: i32 = 75;
const ROLE_DAMAGE_PROGRESS: i32 = 60;
const ROLE_REACTIVE_RISK_PREVENT_HP_LOSS: i32 = 55;
const ROLE_BLOCK: i32 = 45;
const ROLE_UTILITY_PLAY: i32 = 35;
const ROLE_END_TURN: i32 = 0;
const ROLE_PENDING_VALUE_SELECTION: i32 = 70;
const ROLE_PENDING_REMOVAL_SELECTION: i32 = 65;
const ROLE_PENDING_NEUTRAL_SELECTION: i32 = 20;
const ROLE_PENDING_CANCEL: i32 = -10;
const ROLE_DISCARD_POTION: i32 = -20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ActionOrderingPriority {
    pub(super) role: ActionOrderingRole,
    role_rank: i32,
    potion_tactical_rank: i32,
    mitigation: i32,
    reactive_risk: i32,
    target_progress: i32,
    block: i32,
    damage: i32,
    cheaper_cost: i32,
    phase_transition_safety: i32,
    pending_choice_primary: i32,
    pending_choice_secondary: i32,
    pending_choice_selected_count: i32,
    pub(super) effects: PlayCardEffectDiagnostics,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ActionOrderingRole {
    LethalCard,
    PreventVisibleLethal,
    SustainedMitigation,
    TacticalPotion,
    PreventHpLoss,
    DeferredSetup,
    DamageProgress,
    ReactiveRiskPreventHpLoss,
    Block,
    UtilityPlay,
    EndTurn,
    PendingChoiceValueSelection,
    PendingChoiceRemovalSelection,
    PendingChoiceNeutralSelection,
    PendingChoiceCancel,
    DiscardPotion,
    Neutral,
}

pub(super) fn priority_for_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> ActionOrderingPriority {
    if let Some(hint) = pending_choice_ordering_hint(engine, combat, input) {
        let (role, role_rank) = pending_choice_role_rank(hint.role);
        return ActionOrderingPriority {
            role,
            role_rank,
            pending_choice_primary: hint.primary,
            pending_choice_secondary: hint.secondary,
            pending_choice_selected_count: hint.selected_count_tiebreak,
            ..ActionOrderingPriority::neutral(role)
        };
    }

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
    let effects = summarize_play_card_effects(combat, card, target);
    let effect_diagnostics = effects.diagnostics();
    let block = evaluated
        .base_block_mut
        .max(0)
        .saturating_add(effects.reactive_player_block);
    let target_progress = target_progress_hint(combat, target_kind, target, damage)
        .saturating_add(effects.reactive_enemy_damage);
    let mitigation = effects.net_mitigation_ordering_score().max(0);
    let reactive_risk = effects.reactive_risk_score();
    let phase_transition = enemy_phase_transition_hint_for_input(
        combat,
        &ClientInput::PlayCard { card_index, target },
    );
    let visible_damage = visible_incoming_damage(combat);
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block =
        (visible_damage - current_block - block - effects.visible_attack_mitigation_hint)
            .max(0)
            .saturating_add(effects.reactive_player_hp_loss);
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
    } else if mitigation > 0 {
        (
            ActionOrderingRole::SustainedMitigation,
            ROLE_SUSTAINED_MITIGATION,
        )
    } else if def.card_type == CardType::Power {
        (ActionOrderingRole::DeferredSetup, ROLE_DEFERRED_SETUP)
    } else if prevents_hp_loss && reactive_risk == 0 {
        (ActionOrderingRole::PreventHpLoss, ROLE_PREVENT_HP_LOSS)
    } else if target_progress > 0 {
        (ActionOrderingRole::DamageProgress, ROLE_DAMAGE_PROGRESS)
    } else if prevents_hp_loss {
        (
            ActionOrderingRole::ReactiveRiskPreventHpLoss,
            ROLE_REACTIVE_RISK_PREVENT_HP_LOSS,
        )
    } else if block > 0 {
        (ActionOrderingRole::Block, ROLE_BLOCK)
    } else {
        (ActionOrderingRole::UtilityPlay, ROLE_UTILITY_PLAY)
    };

    ActionOrderingPriority {
        role,
        role_rank,
        mitigation,
        reactive_risk: -reactive_risk,
        target_progress,
        block,
        damage,
        cheaper_cost: -card.cost_for_turn_java().max(0),
        phase_transition_safety: -phase_transition.ordering_risk_score(),
        effects: effect_diagnostics,
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
            mitigation: 0,
            reactive_risk: 0,
            target_progress: 0,
            block: 0,
            damage: 0,
            cheaper_cost: 0,
            phase_transition_safety: 0,
            pending_choice_primary: 0,
            pending_choice_secondary: 0,
            pending_choice_selected_count: 0,
            effects: PlayCardEffectDiagnostics::default(),
        }
    }
}

fn pending_choice_role_rank(role: PendingChoiceOrderingRole) -> (ActionOrderingRole, i32) {
    match role {
        PendingChoiceOrderingRole::ValueSelection => (
            ActionOrderingRole::PendingChoiceValueSelection,
            ROLE_PENDING_VALUE_SELECTION,
        ),
        PendingChoiceOrderingRole::RemovalSelection => (
            ActionOrderingRole::PendingChoiceRemovalSelection,
            ROLE_PENDING_REMOVAL_SELECTION,
        ),
        PendingChoiceOrderingRole::NeutralSelection => (
            ActionOrderingRole::PendingChoiceNeutralSelection,
            ROLE_PENDING_NEUTRAL_SELECTION,
        ),
        PendingChoiceOrderingRole::Cancel => {
            (ActionOrderingRole::PendingChoiceCancel, ROLE_PENDING_CANCEL)
        }
    }
}

impl ActionOrderingRole {
    pub(super) fn label(self) -> &'static str {
        match self {
            ActionOrderingRole::LethalCard => "lethal_card",
            ActionOrderingRole::PreventVisibleLethal => "prevent_visible_lethal",
            ActionOrderingRole::SustainedMitigation => "sustained_mitigation",
            ActionOrderingRole::TacticalPotion => "tactical_potion",
            ActionOrderingRole::PreventHpLoss => "prevent_hp_loss",
            ActionOrderingRole::DeferredSetup => "deferred_setup",
            ActionOrderingRole::DamageProgress => "damage_progress",
            ActionOrderingRole::ReactiveRiskPreventHpLoss => "reactive_risk_prevent_hp_loss",
            ActionOrderingRole::Block => "block",
            ActionOrderingRole::UtilityPlay => "utility_play",
            ActionOrderingRole::EndTurn => "end_turn",
            ActionOrderingRole::PendingChoiceValueSelection => "pending_choice_value_selection",
            ActionOrderingRole::PendingChoiceRemovalSelection => "pending_choice_removal_selection",
            ActionOrderingRole::PendingChoiceNeutralSelection => "pending_choice_neutral_selection",
            ActionOrderingRole::PendingChoiceCancel => "pending_choice_cancel",
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
            .then_with(|| self.mitigation.cmp(&other.mitigation))
            .then_with(|| self.reactive_risk.cmp(&other.reactive_risk))
            .then_with(|| {
                self.phase_transition_safety
                    .cmp(&other.phase_transition_safety)
            })
            .then_with(|| self.target_progress.cmp(&other.target_progress))
            .then_with(|| self.block.cmp(&other.block))
            .then_with(|| self.damage.cmp(&other.damage))
            .then_with(|| self.cheaper_cost.cmp(&other.cheaper_cost))
            .then_with(|| {
                self.pending_choice_primary
                    .cmp(&other.pending_choice_primary)
            })
            .then_with(|| {
                self.pending_choice_secondary
                    .cmp(&other.pending_choice_secondary)
            })
            .then_with(|| {
                self.pending_choice_selected_count
                    .cmp(&other.pending_choice_selected_count)
            })
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
    fn non_player_turn_priority_is_neutral() {
        let combat = blank_test_combat();

        let priority = priority_for_input(
            &EngineState::CombatProcessing,
            &combat,
            &ClientInput::EndTurn,
        );

        assert_eq!(priority.role, ActionOrderingRole::Neutral);
    }

    #[test]
    fn lethal_play_card_gets_lethal_role() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.current_hp = 6;
        monster.max_hp = 6;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

        let priority = priority_for_input(
            &EngineState::CombatPlayerTurn,
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        assert_eq!(priority.role, ActionOrderingRole::LethalCard);
    }

    #[test]
    fn pending_choice_priority_uses_structured_selection_role() {
        let mut combat = blank_test_combat();
        combat.zones.discard_pile = vec![CombatCard::new(CardId::Carnage, 20)];
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::core::GridSelectReason::MoveToDrawPile,
        });

        let priority =
            priority_for_input(&engine, &combat, &ClientInput::SubmitGridSelect(vec![20]));

        assert_eq!(
            priority.role,
            ActionOrderingRole::PendingChoiceValueSelection
        );
        assert!(priority.pending_choice_primary > 0);
    }
}
