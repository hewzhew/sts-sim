use crate::content::cards::{CardId, CardType};
use crate::state::core::EngineState;
use crate::state::selection::{
    SelectionConstraint, SelectionReason, SelectionRequest, SelectionScope, SelectionTargetRef,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunPendingChoiceReason {
    Purge,
    PurgeNonBottled,
    Upgrade,
    Transform,
    TransformNonBottled,
    TransformUpgraded,
    Duplicate,
    BottleFlame,
    BottleLightning,
    BottleTornado,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunPendingChoiceState {
    pub min_choices: usize,
    pub max_choices: usize,
    pub reason: RunPendingChoiceReason,
    pub return_state: Box<EngineState>,
}

impl From<RunPendingChoiceReason> for SelectionReason {
    fn from(value: RunPendingChoiceReason) -> Self {
        match value {
            RunPendingChoiceReason::Purge => SelectionReason::Purge,
            RunPendingChoiceReason::PurgeNonBottled => SelectionReason::Purge,
            RunPendingChoiceReason::Upgrade => SelectionReason::Upgrade,
            RunPendingChoiceReason::Transform => SelectionReason::Transform,
            RunPendingChoiceReason::TransformNonBottled => SelectionReason::Transform,
            RunPendingChoiceReason::TransformUpgraded => SelectionReason::TransformUpgraded,
            RunPendingChoiceReason::Duplicate => SelectionReason::Duplicate,
            RunPendingChoiceReason::BottleFlame => SelectionReason::BottleFlame,
            RunPendingChoiceReason::BottleLightning => SelectionReason::BottleLightning,
            RunPendingChoiceReason::BottleTornado => SelectionReason::BottleTornado,
        }
    }
}

impl RunPendingChoiceState {
    pub fn selection_request(&self, run_state: &crate::state::run::RunState) -> SelectionRequest {
        let targets: Vec<_> = run_state
            .master_deck
            .iter()
            .filter(|card| run_pending_choice_allows_card_for_run(&self.reason, card, run_state))
            .map(|card| SelectionTargetRef::CardUuid(card.uuid))
            .collect();

        SelectionRequest {
            scope: SelectionScope::Deck,
            reason: self.reason.into(),
            constraint: SelectionConstraint::from_bounds(
                self.min_choices,
                self.max_choices,
                targets.len(),
            ),
            can_cancel: self.min_choices == 0,
            targets,
        }
    }
}

pub(crate) fn run_pending_choice_allows_card(
    reason: &RunPendingChoiceReason,
    card: &crate::runtime::combat::CombatCard,
) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    match reason {
        RunPendingChoiceReason::Purge
        | RunPendingChoiceReason::PurgeNonBottled
        | RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => master_deck_card_is_purgeable(card),
        RunPendingChoiceReason::Upgrade => master_deck_card_can_upgrade(card),
        RunPendingChoiceReason::BottleFlame => {
            master_deck_card_is_purgeable(card) && def.card_type == CardType::Attack
        }
        RunPendingChoiceReason::BottleLightning => {
            master_deck_card_is_purgeable(card) && def.card_type == CardType::Skill
        }
        RunPendingChoiceReason::BottleTornado => {
            master_deck_card_is_purgeable(card) && def.card_type == CardType::Power
        }
        _ => true,
    }
}

pub(crate) fn run_pending_choice_allows_card_for_run(
    reason: &RunPendingChoiceReason,
    card: &crate::runtime::combat::CombatCard,
    run_state: &crate::state::run::RunState,
) -> bool {
    if !run_pending_choice_allows_card(reason, card) {
        return false;
    }

    match reason {
        RunPendingChoiceReason::PurgeNonBottled | RunPendingChoiceReason::TransformNonBottled => {
            !master_deck_card_is_bottled(card, &run_state.relics)
        }
        _ => true,
    }
}

pub(crate) fn master_deck_card_can_upgrade(card: &crate::runtime::combat::CombatCard) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    card.id == CardId::SearingBlow
        || (card.upgrades == 0
            && def.card_type != CardType::Status
            && def.card_type != CardType::Curse)
}

pub(crate) fn master_deck_card_is_purgeable(card: &crate::runtime::combat::CombatCard) -> bool {
    !matches!(
        card.id,
        CardId::AscendersBane | CardId::CurseOfTheBell | CardId::Necronomicurse
    )
}

pub(crate) fn master_deck_card_is_bottled(
    card: &crate::runtime::combat::CombatCard,
    relics: &[crate::content::relics::RelicState],
) -> bool {
    card.uuid != 0
        && relics.iter().any(|relic| {
            matches!(
                relic.id,
                crate::content::relics::RelicId::BottledFlame
                    | crate::content::relics::RelicId::BottledLightning
                    | crate::content::relics::RelicId::BottledTornado
            ) && relic.amount == card.uuid as i32
        })
}

pub(crate) fn non_bottled_purgeable_master_deck_count(
    run_state: &crate::state::run::RunState,
) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| {
            master_deck_card_is_purgeable(card)
                && !master_deck_card_is_bottled(card, &run_state.relics)
        })
        .count()
}

pub(crate) fn has_non_bottled_purgeable_master_deck_card(
    run_state: &crate::state::run::RunState,
) -> bool {
    non_bottled_purgeable_master_deck_count(run_state) > 0
}
