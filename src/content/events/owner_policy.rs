use crate::ai::strategy::decision_pipeline::{
    evaluate_decision_candidate, DecisionCandidateKind, DecisionPipelineContext,
};
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::deck_purge_target::{best_purge_uuid, rank_purge_target};
use crate::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit_summary, StrategicBurdenLevel, StrategicDeficitLevel,
};
use crate::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1,
};
use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
use crate::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId,
};
use crate::content::potions::PotionId;
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
        EventId::Beggar => return Ok(choose(beggar_choice(run_state))),
        EventId::BigFish => return Ok(choose(big_fish_choice(run_state))),
        EventId::CursedTome => return Ok(choose(cursed_tome_choice(run_state))),
        EventId::DeadAdventurer => return Ok(choose(dead_adventurer_choice(run_state))),
        EventId::Designer => return Ok(choose(designer_choice(run_state))),
        EventId::Falling => return Ok(choose(super::falling_owner::falling_choice(run_state))),
        EventId::ForgottenAltar => return Ok(choose(forgotten_altar_choice(run_state))),
        EventId::Ghosts => return Ok(choose(ghosts_choice(run_state))),
        EventId::LivingWall => return Ok(choose(living_wall_choice(run_state))),
        EventId::MaskedBandits => return Ok(choose(masked_bandits_choice(run_state))),
        EventId::MatchAndKeep => return Ok(choose(match_and_keep_choice(run_state))),
        EventId::Mausoleum => return Ok(choose(mausoleum_choice(run_state))),
        EventId::MindBloom => return Ok(choose(mind_bloom_choice(run_state))),
        EventId::MoaiHead => return Ok(choose(moai_head_choice(run_state))),
        EventId::Mushrooms => return Ok(choose(mushrooms_choice(run_state))),
        EventId::MysteriousSphere => return Ok(choose(mysterious_sphere_choice(run_state))),
        EventId::Nest => return Ok(choose(nest_choice(run_state))),
        EventId::Nloth => return Ok(choose(nloth_choice(run_state))),
        EventId::Purifier => return Ok(choose(purifier_choice(run_state))),
        EventId::ShiningLight => return Ok(choose(shining_light_choice(run_state))),
        EventId::TheLibrary => return Ok(choose(the_library_choice(run_state))),
        EventId::TombRedMask => return Ok(choose(tomb_red_mask_choice(run_state))),
        EventId::Transmorgrifier => return Ok(choose(transmorgrifier_choice(run_state))),
        EventId::Vampires => return Ok(choose(vampires_choice(run_state))),
        EventId::WomanInBlue => return Ok(choose(woman_in_blue_choice(run_state))),
        EventId::WeMeetAgain => return Ok(choose(we_meet_again_choice(run_state))),
        EventId::WindingHalls => return Ok(choose(winding_halls_choice(run_state))),
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

fn beggar_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if run_state.gold >= 75 && has_safe_purge_target(run_state) => {
            action(EventActionKind::Trade)
        }
        0 => action(EventActionKind::Leave),
        1 => action(EventActionKind::DeckOperation),
        _ => action(EventActionKind::Leave),
    }
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

fn transmorgrifier_choice(_run_state: &RunState) -> EventOwnerOptionSelector {
    action(EventActionKind::Leave)
}

fn purifier_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if has_safe_purge_target(run_state) => action(EventActionKind::DeckOperation),
        0 => action(EventActionKind::Leave),
        _ => action(EventActionKind::Leave),
    }
}

fn the_library_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if the_library_should_sleep(run_state) => option_index(1),
        0 => option_index(0),
        1 => the_library_best_card_index(run_state)
            .map(option_index)
            .unwrap_or_else(|| action(EventActionKind::Leave)),
        _ => action(EventActionKind::Leave),
    }
}

fn the_library_should_sleep(run_state: &RunState) -> bool {
    !has_relic(run_state, RelicId::MarkOfTheBloom)
        && run_state.current_hp * 100 <= run_state.max_hp * 35
}

fn the_library_best_card_index(run_state: &RunState) -> Option<usize> {
    let event_state = run_state.event_state.as_ref()?;
    let context = DecisionPipelineContext::reward(DeckPlanSnapshot::from_run_state(run_state));
    (0..event_state.extra_data.len() / 2)
        .filter_map(|idx| {
            let (card, upgrades) =
                super::the_library::library_card_entry_at(run_state, &event_state.extra_data, idx)?;
            let admission =
                assess_reward_admission_from_master_deck(&run_state.master_deck, card, upgrades);
            let evaluation = evaluate_decision_candidate(
                context,
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                Some(&admission),
            );
            Some((
                (
                    evaluation.order_key(false),
                    reward_admission_order_key_v1(&admission),
                    idx,
                ),
                idx,
            ))
        })
        .min_by_key(|(key, _)| *key)
        .map(|(_, idx)| idx)
}

fn tomb_red_mask_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if has_relic(run_state, RelicId::RedMask) => effect(EventEffect::GainGold(222)),
        0 if run_state.gold <= 80 => action(EventActionKind::Trade),
        0 => action(EventActionKind::Leave),
        _ => action(EventActionKind::Leave),
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
        1 if !hp_after_loss_is_safe(run_state, 6) => {
            effect(EventEffect::GainGold(if run_state.ascension_level >= 15 {
                50
            } else {
                99
            }))
        }
        1 if has_relic(run_state, RelicId::Ectoplasm) || run_state.master_deck.len() < 28 => {
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

fn nloth_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    nloth_feed_option(run_state).unwrap_or_else(|| action(EventActionKind::Leave))
}

fn nloth_feed_option(run_state: &RunState) -> Option<EventOwnerOptionSelector> {
    crate::engine::event_handler::get_event_options(run_state)
        .into_iter()
        .enumerate()
        .filter_map(|(option_index, option)| {
            let relic_id = option.semantics.effects.iter().find_map(|effect| {
                if let EventEffect::LoseRelic {
                    specific: Some(id), ..
                } = effect
                {
                    Some(*id)
                } else {
                    None
                }
            })?;
            let relic = run_state.relics.iter().find(|relic| relic.id == relic_id)?;
            let priority =
                crate::ai::strategy::relic_expendability::nloth_free_feed_priority(relic)?;
            Some((priority, option_index))
        })
        .max_by_key(|(priority, _)| *priority)
        .map(|(_, idx)| option_index(idx))
}

fn match_and_keep_choice(_run_state: &RunState) -> EventOwnerOptionSelector {
    option_index(0)
}

fn mind_bloom_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if run_state.floor_num % 50 > 40 && run_state.current_hp * 100 <= run_state.max_hp * 35 {
        return effect(EventEffect::Heal(
            (run_state.max_hp - run_state.current_hp).max(0),
        ));
    }
    if run_state.floor_num % 50 <= 40 && omamori_charges(run_state) >= 2 {
        return effect(EventEffect::GainGold(999));
    }
    action(EventActionKind::Fight)
}

fn moai_head_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if run_state.current_hp * 100 <= run_state.max_hp * 45 => option_index(0),
        0 if has_relic(run_state, RelicId::GoldenIdol) && run_state.floor_num <= 43 => {
            option_index(1)
        }
        0 => action(EventActionKind::Leave),
        _ => action(EventActionKind::Leave),
    }
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

fn forgotten_altar_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_relic(run_state, RelicId::GoldenIdol) {
        return action(EventActionKind::Trade);
    }
    if has_omamori_charge(run_state) {
        return effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Decay),
        });
    }
    if forgotten_altar_pray_projected_hp(run_state) >= forgotten_altar_min_hp_after_pray(run_state)
    {
        effect(EventEffect::GainMaxHp(5))
    } else {
        effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Decay),
        })
    }
}

fn forgotten_altar_pray_projected_hp(run_state: &RunState) -> i32 {
    let (max_hp_gain, raw_loss) = forgotten_altar_pray_effects(run_state)
        .unwrap_or((5, forgotten_altar_fallback_pray_loss(run_state)));
    let healed_hp = (run_state.current_hp + max_hp_gain).min(run_state.max_hp + max_hp_gain);
    healed_hp.saturating_sub(event_hp_loss_estimate(run_state, raw_loss))
}

fn forgotten_altar_pray_effects(run_state: &RunState) -> Option<(i32, i32)> {
    crate::engine::event_handler::get_event_options(run_state)
        .into_iter()
        .filter(|option| !option.ui.disabled)
        .find_map(|option| {
            let max_hp_gain = option
                .semantics
                .effects
                .iter()
                .find_map(|effect| match effect {
                    EventEffect::GainMaxHp(amount) => Some(*amount),
                    _ => None,
                })?;
            let hp_loss = option
                .semantics
                .effects
                .iter()
                .find_map(|effect| match effect {
                    EventEffect::LoseHp(amount) => Some(*amount),
                    _ => None,
                })?;
            Some((max_hp_gain, hp_loss))
        })
}

fn forgotten_altar_fallback_pray_loss(run_state: &RunState) -> i32 {
    let pct = if run_state.ascension_level >= 15 {
        0.35
    } else {
        0.25
    };
    (run_state.max_hp as f32 * pct).round() as i32
}

fn forgotten_altar_min_hp_after_pray(run_state: &RunState) -> i32 {
    let deficit = assess_deck_strategic_deficit_summary(
        &run_state.master_deck,
        RunStrategicFacts::from_run_state(run_state),
    );
    let mut min_hp = if run_state.ascension_level >= 15 {
        32
    } else {
        26
    };
    if has_relic(run_state, RelicId::RunicDome) {
        min_hp += 4;
    }
    if matches!(
        deficit.block_or_mitigation,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    ) {
        min_hp += 4;
    }
    if deficit.deck_burden == StrategicBurdenLevel::Heavy {
        min_hp -= 2;
    }
    min_hp
}

fn masked_bandits_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    action(match event_screen(run_state) {
        0 if masked_bandits_should_pay(run_state) => EventActionKind::Trade,
        0 => EventActionKind::Fight,
        1..=3 => EventActionKind::Continue,
        _ => EventActionKind::Leave,
    })
}

fn masked_bandits_should_pay(run_state: &RunState) -> bool {
    let danger = run_state.current_hp <= 20 || run_state.current_hp * 100 <= run_state.max_hp * 25;
    let borderline =
        run_state.current_hp <= 28 || run_state.current_hp * 100 <= run_state.max_hp * 35;
    danger
        || (borderline && !masked_bandits_ready(run_state))
        || (borderline && run_state.gold <= 80)
}

fn masked_bandits_ready(run_state: &RunState) -> bool {
    let deficit = assess_deck_strategic_deficit_summary(
        &run_state.master_deck,
        RunStrategicFacts::from_run_state(run_state),
    );
    let offense = deficit.frontload_damage != StrategicDeficitLevel::Missing
        || deficit.aoe_or_minion_control != StrategicDeficitLevel::Missing;
    offense
        && (deficit.block_or_mitigation != StrategicDeficitLevel::Missing
            || has_bandit_swing_potion(run_state))
}

fn has_bandit_swing_potion(run_state: &RunState) -> bool {
    run_state.potions.iter().any(|slot| {
        slot.as_ref().is_some_and(|potion| {
            matches!(
                potion.id,
                PotionId::FirePotion
                    | PotionId::ExplosivePotion
                    | PotionId::FearPotion
                    | PotionId::BlockPotion
            )
        })
    })
}

fn vampires_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if vampires_should_give_vial(run_state) => action(EventActionKind::Trade),
        0 => action(EventActionKind::Decline),
        _ => action(EventActionKind::Leave),
    }
}

fn vampires_should_give_vial(run_state: &RunState) -> bool {
    has_relic(run_state, RelicId::BloodVial)
        && starter_strike_count(run_state) >= 3
        && run_state.master_deck.len() <= 32
}

fn starter_strike_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| is_starter_strike(card.id))
        .count()
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

fn winding_halls_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 if winding_halls_should_press_on(run_state) => effect(EventEffect::Heal(
            (run_state.max_hp as f32
                * if run_state.ascension_level >= 15 {
                    0.20
                } else {
                    0.25
                })
            .round() as i32,
        )),
        1 => effect(EventEffect::LoseMaxHp(
            (run_state.max_hp as f32 * 0.05).round() as i32,
        )),
        _ => action(EventActionKind::Leave),
    }
}

fn winding_halls_should_press_on(run_state: &RunState) -> bool {
    run_state.current_hp * 100 <= run_state.max_hp * 30
        && !has_card(run_state, CardId::Writhe)
        && curse_count(run_state) < 2
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
    omamori_charges(run_state) > 0
}

fn omamori_charges(run_state: &RunState) -> i32 {
    run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::Omamori && !relic.used_up)
        .map(|relic| relic.counter.max(0))
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;
    use crate::state::events::{EventId, EventState};

    fn forgotten_altar_run(current_hp: i32, max_hp: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.floor_num = 29;
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.event_state = Some(EventState::new(EventId::ForgottenAltar));
        run_state.relics = vec![RelicState::new(RelicId::RunicDome)];
        run_state.master_deck = forgotten_altar_heavy_burden_deck();
        run_state
    }

    fn forgotten_altar_heavy_burden_deck() -> Vec<CombatCard> {
        [
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Headbutt,
            CardId::Armaments,
            CardId::BurningPact,
            CardId::Whirlwind,
            CardId::Feed,
            CardId::ShrugItOff,
            CardId::Cleave,
            CardId::DemonForm,
            CardId::Rupture,
        ]
        .into_iter()
        .enumerate()
        .map(|(idx, id)| CombatCard::new(id, idx as u32))
        .collect()
    }

    #[test]
    fn forgotten_altar_prays_when_projected_hp_clears_v0_safety_floor() {
        let run_state = forgotten_altar_run(52, 99);

        assert_eq!(
            forgotten_altar_choice(&run_state),
            EventOwnerOptionSelector::Effect(EventEffect::GainMaxHp(5))
        );
    }

    #[test]
    fn forgotten_altar_takes_decay_when_projected_hp_is_below_v0_safety_floor() {
        let run_state = forgotten_altar_run(40, 99);

        assert_eq!(
            forgotten_altar_choice(&run_state),
            EventOwnerOptionSelector::Effect(EventEffect::ObtainCurse {
                count: 1,
                kind: EventCardKind::Specific(CardId::Decay),
            })
        );
    }

    #[test]
    fn forgotten_altar_offers_golden_idol_before_other_choices() {
        let mut run_state = forgotten_altar_run(20, 99);
        run_state.relics.push(RelicState::new(RelicId::GoldenIdol));

        assert_eq!(
            forgotten_altar_choice(&run_state),
            EventOwnerOptionSelector::Action(EventActionKind::Trade)
        );
    }

    #[test]
    fn forgotten_altar_uses_omamori_to_block_decay_before_losing_hp() {
        let mut run_state = forgotten_altar_run(52, 99);
        run_state.relics.push(RelicState::new(RelicId::Omamori));

        assert_eq!(
            forgotten_altar_choice(&run_state),
            EventOwnerOptionSelector::Effect(EventEffect::ObtainCurse {
                count: 1,
                kind: EventCardKind::Specific(CardId::Decay),
            })
        );
    }
}
