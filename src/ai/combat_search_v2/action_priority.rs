use super::action_effects::PlayCardEffectDiagnostics;
use super::phase_action_ordering::PhaseActionOrderingHint;
use super::*;
use std::cmp::Ordering;

mod play_card;

// These ranks only decide child-generation order inside the same legal action set.
// They never merge, prune, or claim that two actions are equivalent.
pub(super) const ROLE_LETHAL_CARD: i32 = 130;
pub(super) const ROLE_PREVENT_VISIBLE_LETHAL: i32 = 120;
pub(super) const ROLE_SUSTAINED_MITIGATION: i32 = 95;
const ROLE_TACTICAL_POTION_BASE: i32 = 60;
pub(super) const ROLE_PREVENT_HP_LOSS: i32 = 85;
pub(super) const ROLE_DEFERRED_SETUP: i32 = 75;
pub(super) const ROLE_DAMAGE_PROGRESS: i32 = 60;
pub(super) const ROLE_REACTIVE_RISK_PREVENT_HP_LOSS: i32 = 55;
pub(super) const ROLE_BLOCK: i32 = 45;
pub(super) const ROLE_UTILITY_PLAY: i32 = 35;
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
    phase_setup: i32,
    phase_survival: i32,
    phase_transition_safety: i32,
    pending_choice_primary: i32,
    pending_choice_secondary: i32,
    pending_choice_selected_count: i32,
    pub(super) phase_hint: PhaseActionOrderingHint,
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
            let phase_profile = combat_search_phase_profile(engine, combat);
            play_card::priority_for_play_card(combat, *card_index, *target, phase_profile)
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
            phase_setup: 0,
            phase_survival: 0,
            phase_transition_safety: 0,
            pending_choice_primary: 0,
            pending_choice_secondary: 0,
            pending_choice_selected_count: 0,
            phase_hint: PhaseActionOrderingHint::default(),
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
            .then_with(|| self.phase_setup.cmp(&other.phase_setup))
            .then_with(|| self.phase_survival.cmp(&other.phase_survival))
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
mod tests;
