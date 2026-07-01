use crate::ai::strategy::deck_purge_target::{best_purge_uuid, rank_purge_target};
use crate::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId,
};
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{
    EventActionKind, EventCardKind, EventEffect, EventId, EventOptionSemantics,
    EventOwnerPolicyKind,
};
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
    UnsupportedDesignerSelection(RunPendingChoiceReason),
    UnsupportedLivingWallSelection(RunPendingChoiceReason),
    EmptySelectionTargets,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOwnerAction {
    ChooseOption(EventOwnerOptionSelector),
    SubmitSelection(SelectionResolution),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOwnerOptionSelector {
    Action(EventActionKind),
    Effect(EventEffect),
    OwnerPolicy(EventOwnerPolicyKind),
    OptionIndex(usize),
}

impl EventOwnerOptionSelector {
    pub fn matches(&self, option_index: usize, semantics: &EventOptionSemantics) -> bool {
        match self {
            EventOwnerOptionSelector::Action(action) => semantics.action == *action,
            EventOwnerOptionSelector::Effect(effect) => semantics.effects.contains(effect),
            EventOwnerOptionSelector::OwnerPolicy(policy) => semantics.owner_policy == *policy,
            EventOwnerOptionSelector::OptionIndex(index) => option_index == *index,
        }
    }
}

pub fn event_owner_policy_action(
    engine_state: &EngineState,
    run_state: &RunState,
) -> Result<EventOwnerAction, EventOwnerPolicyGap> {
    match engine_state {
        EngineState::EventRoom => event_room_policy_action(run_state),
        EngineState::RunPendingChoice(choice) => event_run_choice_policy_action(choice, run_state),
        _ => Err(EventOwnerPolicyGap::MissingEventState),
    }
}

fn event_room_policy_action(run_state: &RunState) -> Result<EventOwnerAction, EventOwnerPolicyGap> {
    let event_id = run_state
        .event_state
        .as_ref()
        .map(|event| event.id)
        .ok_or(EventOwnerPolicyGap::MissingEventState)?;
    match event_id {
        EventId::BackTotheBasics => return Ok(choose(back_to_basics_choice(run_state))),
        EventId::BigFish => return Ok(choose(big_fish_choice(run_state))),
        EventId::CursedTome => return Ok(choose(cursed_tome_choice(run_state))),
        EventId::DeadAdventurer => return Ok(choose(dead_adventurer_choice(run_state))),
        EventId::Designer => return Ok(choose(designer_choice(run_state))),
        EventId::Ghosts => return Ok(choose(ghosts_choice(run_state))),
        EventId::LivingWall => return Ok(choose(living_wall_choice(run_state))),
        EventId::MatchAndKeep => return Ok(choose(match_and_keep_choice(run_state))),
        EventId::Mausoleum => return Ok(choose(mausoleum_choice(run_state))),
        EventId::Mushrooms => return Ok(choose(mushrooms_choice(run_state))),
        EventId::MysteriousSphere => return Ok(choose(mysterious_sphere_choice(run_state))),
        EventId::Nest => return Ok(choose(nest_choice(run_state))),
        EventId::ShiningLight => return Ok(choose(shining_light_choice(run_state))),
        EventId::WomanInBlue => return Ok(choose(woman_in_blue_choice(run_state))),
        EventId::WeMeetAgain => return Ok(choose(we_meet_again_choice(run_state))),
        _ => {}
    }
    let marked_count = crate::engine::event_handler::get_event_options(run_state)
        .iter()
        .filter(|option| {
            !option.ui.disabled
                && option.semantics.owner_policy == EventOwnerPolicyKind::ConservativeAuto
        })
        .count();
    if marked_count != 1 {
        return if marked_count == 0 {
            Err(EventOwnerPolicyGap::MissingMarkedPolicy(event_id))
        } else {
            Err(EventOwnerPolicyGap::AmbiguousMarkedPolicy {
                event_id,
                found: marked_count,
            })
        };
    }
    Ok(choose(EventOwnerOptionSelector::OwnerPolicy(
        EventOwnerPolicyKind::ConservativeAuto,
    )))
}

fn event_run_choice_policy_action(
    choice: &RunPendingChoiceState,
    run_state: &RunState,
) -> Result<EventOwnerAction, EventOwnerPolicyGap> {
    let DomainEventSource::Event(event_id) = choice.source else {
        return Err(EventOwnerPolicyGap::MissingEventState);
    };
    match event_id {
        EventId::Designer => designer_selection(choice, run_state),
        EventId::LivingWall => living_wall_selection(choice, run_state),
        _ => Err(EventOwnerPolicyGap::MissingPendingPolicy(event_id)),
    }
}

fn choose(selector: EventOwnerOptionSelector) -> EventOwnerAction {
    EventOwnerAction::ChooseOption(selector)
}

fn action(action: EventActionKind) -> EventOwnerOptionSelector {
    EventOwnerOptionSelector::Action(action)
}

fn effect(effect: EventEffect) -> EventOwnerOptionSelector {
    EventOwnerOptionSelector::Effect(effect)
}

fn option_index(index: usize) -> EventOwnerOptionSelector {
    EventOwnerOptionSelector::OptionIndex(index)
}

fn big_fish_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if run_state.current_hp * 100 <= run_state.max_hp * 35 {
        return effect(EventEffect::Heal(run_state.max_hp / 3));
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Omamori && relic.counter > 0 && !relic.used_up)
    {
        return effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Regret),
        });
    }
    effect(EventEffect::GainMaxHp(5))
}

fn back_to_basics_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_bad_purge_target(run_state) {
        return option_index(0);
    }
    if unupgraded_starter_basic_count(run_state) >= 4 {
        return option_index(1);
    }
    if has_safe_purge_target(run_state) {
        option_index(0)
    } else {
        option_index(1)
    }
}

fn shining_light_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if !run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade)
    {
        return action(EventActionKind::Leave);
    }
    let after = run_state.current_hp - shining_light_damage(run_state);
    if after >= 18 && after * 100 >= run_state.max_hp * 35 {
        effect(EventEffect::UpgradeCard { count: 2 })
    } else {
        action(EventActionKind::Leave)
    }
}

fn living_wall_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if legal_purge_targets(run_state)
        .iter()
        .any(|card| is_non_parasite_curse(card))
    {
        return effect(living_wall_remove_effect());
    }
    if legal_purge_targets(run_state)
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Parasite)
    {
        return effect(EventEffect::TransformCard { count: 1 });
    }
    let best_upgrade = crate::ai::strategy::campfire_upgrade_quality::rank_campfire_upgrades(
        &run_state.master_deck,
    )
    .into_iter()
    .find(|target| {
        legal_purge_targets(run_state)
            .iter()
            .any(|card| card.uuid == run_state.master_deck[target.deck_index].uuid)
    });
    if best_upgrade.as_ref().is_some_and(|target| {
        target.tier >= crate::ai::strategy::campfire_upgrade_quality::CampfireUpgradeTier::Useful
    }) {
        return effect(EventEffect::UpgradeCard { count: 1 });
    }
    if legal_purge_targets(run_state)
        .iter()
        .any(|card| is_starter_strike(card.id))
    {
        return effect(living_wall_remove_effect());
    }
    if best_upgrade.is_some() {
        return effect(EventEffect::UpgradeCard { count: 1 });
    }
    effect(EventEffect::TransformCard { count: 1 })
}

fn living_wall_remove_effect() -> EventEffect {
    EventEffect::RemoveCard {
        count: 1,
        target_uuid: None,
        kind: EventCardKind::Unknown,
    }
}

fn we_meet_again_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    action(if event_screen(run_state) == 0 {
        EventActionKind::Decline
    } else {
        EventActionKind::Leave
    })
}

fn mysterious_sphere_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    action(match event_screen(run_state) {
        1 => EventActionKind::Fight,
        _ => EventActionKind::Leave,
    })
}

fn nest_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 if has_relic(run_state, RelicId::Ectoplasm) || hp_after_loss_is_safe(run_state, 6) => {
            effect(EventEffect::ObtainColorlessCard {
                count: 1,
                kind: EventCardKind::Specific(CardId::RitualDagger),
            })
        }
        1 => effect(EventEffect::GainGold(if run_state.ascension_level >= 15 {
            50
        } else {
            99
        })),
        _ => action(EventActionKind::Leave),
    }
}

fn match_and_keep_choice(_run_state: &RunState) -> EventOwnerOptionSelector {
    option_index(0)
}

fn mushrooms_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    action(match event_screen(run_state) {
        0 if mushrooms_eat_is_emergency(run_state) => EventActionKind::Trade,
        0 | 2 => EventActionKind::Fight,
        _ => EventActionKind::Leave,
    })
}

fn mushrooms_eat_is_emergency(run_state: &RunState) -> bool {
    run_state.current_hp * 100 < run_state.max_hp * 30
}

fn woman_in_blue_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    let empty_slots = run_state
        .potions
        .iter()
        .filter(|slot| slot.is_none())
        .count();
    match empty_slots {
        0 if run_state.ascension_level >= 15 => effect(EventEffect::ObtainPotion { count: 1 }),
        0 => action(EventActionKind::Leave),
        1 => effect(EventEffect::ObtainPotion { count: 1 }),
        2 => effect(EventEffect::ObtainPotion { count: 2 }),
        _ => effect(EventEffect::ObtainPotion { count: 3 }),
    }
}

fn cursed_tome_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    let screen = event_screen(run_state);
    action(match screen {
        0 if cursed_tome_take_is_safe(run_state, screen) => EventActionKind::Continue,
        0 => EventActionKind::Leave,
        1..=3 => EventActionKind::Continue,
        4 if cursed_tome_take_is_safe(run_state, screen) => EventActionKind::Accept,
        4 => EventActionKind::Decline,
        _ => EventActionKind::Leave,
    })
}

fn cursed_tome_take_is_safe(run_state: &RunState, screen: usize) -> bool {
    hp_after_loss_is_safe(run_state, cursed_tome_take_loss_from(run_state, screen))
}

fn designer_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => designer_service_choice(run_state),
        _ => action(EventActionKind::Leave),
    }
}

fn designer_service_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    let asc = run_state.ascension_level;
    let has_remove = designer_has_clear_remove_target(run_state);
    let has_upgrade = best_upgrade_uuid(run_state, &designer_upgrade_targets(run_state)).is_some();
    if run_state.gold >= designer_full_service_cost(asc) && has_remove {
        return option_index(2);
    }
    if designer_cleanup_removes_cards(run_state)
        && run_state.gold >= designer_cleanup_cost(asc)
        && has_remove
    {
        return option_index(1);
    }
    if run_state.gold >= designer_adjust_cost(asc) && has_upgrade {
        return option_index(0);
    }
    option_index(3)
}

fn designer_selection(
    choice: &RunPendingChoiceState,
    run_state: &RunState,
) -> Result<EventOwnerAction, EventOwnerPolicyGap> {
    let request = choice.selection_request(run_state);
    let uuid = match choice.reason {
        RunPendingChoiceReason::PurgeNonBottled => best_purge_uuid(run_state, &request.targets),
        RunPendingChoiceReason::Upgrade => best_upgrade_uuid(run_state, &request.targets),
        reason => return Err(EventOwnerPolicyGap::UnsupportedDesignerSelection(reason)),
    }
    .ok_or(EventOwnerPolicyGap::EmptySelectionTargets)?;
    Ok(EventOwnerAction::SubmitSelection(
        SelectionResolution::card_uuids(SelectionScope::Deck, [uuid]),
    ))
}

fn designer_has_clear_remove_target(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|card| {
        crate::state::core::run_pending_choice_allows_card_for_run(
            &RunPendingChoiceReason::PurgeNonBottled,
            card,
            run_state,
        ) && rank_purge_target(card) <= 4
    })
}

fn designer_upgrade_targets(run_state: &RunState) -> Vec<SelectionTargetRef> {
    run_state
        .master_deck
        .iter()
        .filter(|card| crate::state::core::master_deck_card_can_upgrade(card))
        .map(|card| SelectionTargetRef::CardUuid(card.uuid))
        .collect()
}

fn designer_adjust_cost(asc: u8) -> i32 {
    if asc >= 15 {
        50
    } else {
        40
    }
}

fn designer_cleanup_cost(asc: u8) -> i32 {
    if asc >= 15 {
        75
    } else {
        60
    }
}

fn designer_full_service_cost(asc: u8) -> i32 {
    if asc >= 15 {
        110
    } else {
        90
    }
}

fn dead_adventurer_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if dead_adventurer_should_search_once(run_state) => option_index(0),
        0 => action(EventActionKind::Leave),
        1 => action(EventActionKind::Fight),
        _ => action(EventActionKind::Leave),
    }
}

fn dead_adventurer_should_search_once(run_state: &RunState) -> bool {
    let Some(event) = run_state.event_state.as_ref() else {
        return false;
    };
    crate::content::events::dead_adventurer::num_rewards(event.internal_state) == 0
        && run_state.current_hp * 100 >= run_state.max_hp * 70
}

fn designer_cleanup_removes_cards(run_state: &RunState) -> bool {
    run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.internal_state & 2 != 0)
}

fn mausoleum_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_omamori_charge(run_state) {
        return option_index(0);
    }
    if run_state.ascension_level < 15 {
        if curse_count(run_state) >= 2
            || has_card(run_state, CardId::Writhe)
            || has_card(run_state, CardId::Clash)
        {
            option_index(1)
        } else {
            option_index(0)
        }
    } else if has_curse_mitigation_or_synergy(run_state) {
        option_index(0)
    } else {
        option_index(1)
    }
}

fn ghosts_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    let max_hp_after = run_state
        .max_hp
        .saturating_sub(((run_state.max_hp as f32) * 0.5).ceil() as i32);
    let supported = has_ghosts_support(run_state);
    if max_hp_after < 30 && !supported {
        return option_index(1);
    }
    if run_state.ascension_level >= 15 {
        if supported {
            option_index(0)
        } else {
            option_index(1)
        }
    } else if max_hp_after >= 35 || supported {
        option_index(0)
    } else {
        option_index(1)
    }
}

fn cursed_tome_take_loss_from(run_state: &RunState, screen: usize) -> i32 {
    let final_damage = if run_state.ascension_level >= 15 {
        15
    } else {
        10
    };
    [(1, 1), (2, 2), (3, 3), (4, final_damage)]
        .into_iter()
        .filter(|(loss_screen, _)| screen <= *loss_screen)
        .map(|(_, amount)| event_hp_loss_estimate(run_state, amount))
        .sum()
}

fn event_hp_loss_estimate(run_state: &RunState, amount: i32) -> i32 {
    let has_tungsten_rod = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::TungstenRod);
    if has_tungsten_rod && amount > 0 {
        amount - 1
    } else {
        amount
    }
}

fn hp_after_loss_is_safe(run_state: &RunState, loss: i32) -> bool {
    let after = run_state.current_hp.saturating_sub(loss);
    after >= 18 && after * 100 >= run_state.max_hp * 35
}

fn has_omamori_charge(run_state: &RunState) -> bool {
    run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Omamori && relic.counter > 0 && !relic.used_up)
}

fn has_relic(run_state: &RunState, relic_id: RelicId) -> bool {
    run_state
        .relics
        .iter()
        .any(|relic| relic.id == relic_id && !relic.used_up)
}

fn has_curse_mitigation_or_synergy(run_state: &RunState) -> bool {
    has_omamori_charge(run_state)
        || has_relic(run_state, RelicId::BlueCandle)
        || has_relic(run_state, RelicId::DuVuDoll)
        || has_relic(run_state, RelicId::DarkstonePeriapt)
}

fn has_ghosts_support(run_state: &RunState) -> bool {
    has_relic(run_state, RelicId::ToxicEgg)
        || has_card(run_state, CardId::Corruption)
        || has_card(run_state, CardId::FeelNoPain)
        || has_card(run_state, CardId::DarkEmbrace)
        || has_card(run_state, CardId::Feed)
}

fn has_card(run_state: &RunState, card_id: CardId) -> bool {
    run_state.master_deck.iter().any(|card| card.id == card_id)
}

fn curse_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| {
            get_card_definition(card.id).card_type == crate::content::cards::CardType::Curse
        })
        .count()
}

fn living_wall_selection(
    choice: &RunPendingChoiceState,
    run_state: &RunState,
) -> Result<EventOwnerAction, EventOwnerPolicyGap> {
    let request = choice.selection_request(run_state);
    let uuid = match choice.reason {
        RunPendingChoiceReason::PurgeNonBottled => best_purge_uuid(run_state, &request.targets),
        RunPendingChoiceReason::TransformNonBottled => {
            pick_target_uuid(run_state, &request.targets, transform_rank)
        }
        RunPendingChoiceReason::Upgrade => best_upgrade_uuid(run_state, &request.targets),
        reason => return Err(EventOwnerPolicyGap::UnsupportedLivingWallSelection(reason)),
    }
    .ok_or(EventOwnerPolicyGap::EmptySelectionTargets)?;
    Ok(EventOwnerAction::SubmitSelection(
        SelectionResolution::card_uuids(SelectionScope::Deck, [uuid]),
    ))
}

fn best_upgrade_uuid(run_state: &RunState, targets: &[SelectionTargetRef]) -> Option<u32> {
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

fn has_bad_purge_target(run_state: &RunState) -> bool {
    legal_purge_targets(run_state)
        .into_iter()
        .any(|card| is_non_parasite_curse(card) || card.id == CardId::Parasite)
}

fn has_safe_purge_target(run_state: &RunState) -> bool {
    legal_purge_targets(run_state)
        .into_iter()
        .any(|card| rank_purge_target(card) <= 4)
}

fn legal_purge_targets(run_state: &RunState) -> Vec<&CombatCard> {
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

fn unupgraded_starter_basic_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| {
            is_starter_basic(card.id) && crate::state::core::master_deck_card_can_upgrade(card)
        })
        .count()
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
