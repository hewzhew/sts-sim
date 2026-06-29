use crate::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike,
};
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventActionKind, EventId, EventOwnerPolicyKind};
use crate::state::run::RunState;
use crate::state::selection::{
    DomainEventSource, SelectionResolution, SelectionScope, SelectionTargetRef,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventOwnerPolicyGap {
    MissingEventState,
    MissingMarkedPolicy(EventId),
    AmbiguousMarkedPolicy { event_id: EventId, found: usize },
    MissingPendingPolicy(EventId),
    UnsupportedLivingWallSelection(RunPendingChoiceReason),
    EmptySelectionTargets,
}

pub fn event_owner_policy_input(
    engine_state: &EngineState,
    run_state: &RunState,
) -> Result<ClientInput, EventOwnerPolicyGap> {
    match engine_state {
        EngineState::EventRoom => event_room_policy_input(run_state),
        EngineState::RunPendingChoice(choice) => event_run_choice_policy_input(choice, run_state),
        _ => Err(EventOwnerPolicyGap::MissingEventState),
    }
}

pub fn conservative_owner_policy_input(
    run_state: &RunState,
) -> Result<ClientInput, EventOwnerPolicyGap> {
    event_room_policy_input(run_state)
}

fn event_room_policy_input(run_state: &RunState) -> Result<ClientInput, EventOwnerPolicyGap> {
    let event_id = run_state
        .event_state
        .as_ref()
        .map(|event| event.id)
        .ok_or(EventOwnerPolicyGap::MissingEventState)?;
    match event_id {
        EventId::BigFish => return Ok(ClientInput::EventChoice(big_fish_choice(run_state))),
        EventId::LivingWall => return Ok(ClientInput::EventChoice(living_wall_choice(run_state))),
        EventId::ShiningLight => {
            return Ok(ClientInput::EventChoice(shining_light_choice(run_state)))
        }
        EventId::WeMeetAgain => {
            return Ok(ClientInput::EventChoice(we_meet_again_choice(run_state)))
        }
        _ => {}
    }
    let marked_indices = crate::engine::event_handler::get_event_options(run_state)
        .iter()
        .enumerate()
        .filter(|(_, option)| {
            !option.ui.disabled
                && option.semantics.owner_policy == EventOwnerPolicyKind::ConservativeAuto
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [index] = marked_indices.as_slice() else {
        return if marked_indices.is_empty() {
            Err(EventOwnerPolicyGap::MissingMarkedPolicy(event_id))
        } else {
            Err(EventOwnerPolicyGap::AmbiguousMarkedPolicy {
                event_id,
                found: marked_indices.len(),
            })
        };
    };
    Ok(ClientInput::EventChoice(*index))
}

fn event_run_choice_policy_input(
    choice: &RunPendingChoiceState,
    run_state: &RunState,
) -> Result<ClientInput, EventOwnerPolicyGap> {
    let DomainEventSource::Event(event_id) = choice.source else {
        return Err(EventOwnerPolicyGap::MissingEventState);
    };
    match event_id {
        EventId::LivingWall => living_wall_selection(choice, run_state),
        _ => Err(EventOwnerPolicyGap::MissingPendingPolicy(event_id)),
    }
}

fn big_fish_choice(run_state: &RunState) -> usize {
    if event_screen(run_state) != 0 {
        return 0;
    }
    if run_state.current_hp * 100 <= run_state.max_hp * 35 {
        return 0;
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Omamori && relic.counter > 0 && !relic.used_up)
    {
        return 2;
    }
    1
}

fn shining_light_choice(run_state: &RunState) -> usize {
    if event_screen(run_state) != 0 {
        return 0;
    }
    if !run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade)
    {
        return 1;
    }
    let after = run_state.current_hp - shining_light_damage(run_state);
    if after >= 18 && after * 100 >= run_state.max_hp * 35 {
        0
    } else {
        1
    }
}

fn living_wall_choice(run_state: &RunState) -> usize {
    if event_screen(run_state) != 0 {
        return 0;
    }
    if legal_living_wall_cards(run_state)
        .iter()
        .any(|card| is_non_parasite_curse(card))
    {
        return 0;
    }
    if legal_living_wall_cards(run_state)
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Parasite)
    {
        return 1;
    }
    let best_upgrade = crate::ai::strategy::campfire_upgrade_quality::rank_campfire_upgrades(
        &run_state.master_deck,
    )
    .into_iter()
    .find(|target| {
        legal_living_wall_cards(run_state)
            .iter()
            .any(|card| card.uuid == run_state.master_deck[target.deck_index].uuid)
    });
    if best_upgrade.as_ref().is_some_and(|target| {
        target.tier >= crate::ai::strategy::campfire_upgrade_quality::CampfireUpgradeTier::Useful
    }) {
        return 2;
    }
    if legal_living_wall_cards(run_state)
        .iter()
        .any(|card| is_starter_strike(card.id))
    {
        return 0;
    }
    if best_upgrade.is_some() {
        return 2;
    }
    1
}

fn we_meet_again_choice(run_state: &RunState) -> usize {
    let action = if event_screen(run_state) == 0 {
        EventActionKind::Decline
    } else {
        EventActionKind::Leave
    };
    event_action_choice(run_state, action).unwrap_or_default()
}

fn event_action_choice(run_state: &RunState, action: EventActionKind) -> Option<usize> {
    crate::engine::event_handler::get_event_options(run_state)
        .iter()
        .enumerate()
        .find(|(_, option)| !option.ui.disabled && option.semantics.action == action)
        .map(|(index, _)| index)
}

fn living_wall_selection(
    choice: &RunPendingChoiceState,
    run_state: &RunState,
) -> Result<ClientInput, EventOwnerPolicyGap> {
    let request = choice.selection_request(run_state);
    let uuid = match choice.reason {
        RunPendingChoiceReason::PurgeNonBottled => {
            pick_target_uuid(run_state, &request.targets, purge_rank)
        }
        RunPendingChoiceReason::TransformNonBottled => {
            pick_target_uuid(run_state, &request.targets, transform_rank)
        }
        RunPendingChoiceReason::Upgrade => living_wall_upgrade_uuid(run_state, &request.targets),
        reason => return Err(EventOwnerPolicyGap::UnsupportedLivingWallSelection(reason)),
    }
    .ok_or(EventOwnerPolicyGap::EmptySelectionTargets)?;
    Ok(ClientInput::SubmitSelection(
        SelectionResolution::card_uuids(SelectionScope::Deck, [uuid]),
    ))
}

fn living_wall_upgrade_uuid(run_state: &RunState, targets: &[SelectionTargetRef]) -> Option<u32> {
    crate::ai::strategy::campfire_upgrade_quality::rank_campfire_upgrades(&run_state.master_deck)
        .into_iter()
        .find_map(|target| {
            let uuid = run_state.master_deck.get(target.deck_index)?.uuid;
            targets
                .iter()
                .any(|target| target.card_uuid() == uuid)
                .then_some(uuid)
        })
        .or_else(|| pick_target_uuid(run_state, targets, |_| 0))
}

fn pick_target_uuid(
    run_state: &RunState,
    targets: &[SelectionTargetRef],
    rank: fn(&CombatCard) -> u8,
) -> Option<u32> {
    targets
        .iter()
        .filter_map(|target| {
            let uuid = target.card_uuid();
            let card = run_state
                .master_deck
                .iter()
                .find(|card| card.uuid == uuid)?;
            Some((rank(card), uuid))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, uuid)| uuid)
}

fn purge_rank(card: &CombatCard) -> u8 {
    if is_non_parasite_curse(card) {
        0
    } else if is_starter_strike(card.id) {
        1
    } else if card.id == crate::content::cards::CardId::Parasite {
        2
    } else if is_starter_defend(card.id) {
        3
    } else if is_starter_basic(card.id) {
        4
    } else {
        5
    }
}

fn transform_rank(card: &CombatCard) -> u8 {
    if card.id == crate::content::cards::CardId::Parasite {
        0
    } else if is_starter_strike(card.id) {
        1
    } else if is_starter_defend(card.id) {
        2
    } else if is_starter_basic(card.id) {
        3
    } else if is_non_parasite_curse(card) {
        4
    } else {
        5
    }
}

fn legal_living_wall_cards(run_state: &RunState) -> Vec<&CombatCard> {
    run_state
        .master_deck
        .iter()
        .filter(|card| {
            crate::state::core::run_pending_choice_allows_card_for_run(
                &RunPendingChoiceReason::PurgeNonBottled,
                card,
                run_state,
            )
        })
        .collect()
}

fn is_non_parasite_curse(card: &CombatCard) -> bool {
    get_card_definition(card.id).card_type == crate::content::cards::CardType::Curse
        && card.id != crate::content::cards::CardId::Parasite
}

fn shining_light_damage(run_state: &RunState) -> i32 {
    let fraction = if run_state.ascension_level >= 15 {
        0.3
    } else {
        0.2
    };
    (run_state.max_hp as f32 * fraction).round() as i32
}

fn event_screen(run_state: &RunState) -> usize {
    run_state
        .event_state
        .as_ref()
        .map(|event| event.current_screen)
        .unwrap_or_default()
}
