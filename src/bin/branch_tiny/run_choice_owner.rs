use sts_simulator::ai::strategy::deck_purge_target::{
    best_purge_uuids, purge_rank_label, rank_purge_target,
};
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId};
use sts_simulator::eval::run_control::{RunControlCommand, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::{
    ClientInput, EngineState, RunPendingChoiceReason, RunPendingChoiceState,
};
use sts_simulator::state::selection::{SelectionResolution, SelectionScope};

use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion, OwnerDecision};

const PREMIUM: i32 = 0;
const GOOD: i32 = 100;
const FALLBACK: i32 = 200;
const AVOID: i32 = 300;

pub(super) fn can_handle(reason: RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Purge
            | RunPendingChoiceReason::PurgeNonBottled
            | RunPendingChoiceReason::BottleFlame
            | RunPendingChoiceReason::BottleLightning
            | RunPendingChoiceReason::BottleTornado
    )
}

pub(super) fn run_choice_owner_decision(session: &RunControlSession) -> OwnerDecision {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return OwnerDecision::Gap("RunChoice owner requires RunPendingChoice state".to_string());
    };
    if !can_handle(choice.reason) {
        return OwnerDecision::Gap(format!(
            "RunChoice owner has no policy for {:?}",
            choice.reason
        ));
    }
    match choice.reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => {
            purge_owner_decision(session, choice)
        }
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => bottle_owner_decision(session, choice),
        _ => OwnerDecision::Gap(format!(
            "RunChoice owner has no policy for {:?}",
            choice.reason
        )),
    }
}

fn purge_owner_decision(
    session: &RunControlSession,
    choice: &RunPendingChoiceState,
) -> OwnerDecision {
    if choice.min_choices != choice.max_choices {
        return OwnerDecision::Gap(format!(
            "Purge owner requires fixed-count choice, got {}-{}",
            choice.min_choices, choice.max_choices
        ));
    }
    let request = choice.selection_request(&session.run_state);
    let uuids = best_purge_uuids(&session.run_state, &request.targets, choice.max_choices);
    if uuids.len() != choice.max_choices {
        return OwnerDecision::Gap(format!(
            "{:?} needs {} safe purge targets, found {}",
            choice.reason,
            choice.max_choices,
            uuids.len()
        ));
    }
    let labels = uuids
        .iter()
        .filter_map(|uuid| {
            let card = session
                .run_state
                .master_deck
                .iter()
                .find(|card| card.uuid == *uuid)?;
            Some(format!(
                "{} {}",
                card_label(card),
                purge_rank_label(rank_purge_target(card))
            ))
        })
        .collect::<Vec<_>>();

    OwnerDecision::Candidates(vec![OwnerChoice {
        key: None,
        action: RunControlCommand::Input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(SelectionScope::Deck, uuids),
        )),
        label: format!("{:?} {}", choice.reason, labels.join(", ")),
        annotation: ChoiceAnnotation::None,
        expansion: OwnerChoiceExpansion::AutoAllowed,
    }])
}

fn bottle_owner_decision(
    session: &RunControlSession,
    choice: &RunPendingChoiceState,
) -> OwnerDecision {
    if choice.min_choices != 1 || choice.max_choices != 1 {
        return OwnerDecision::Gap(format!(
            "Bottle owner requires single-card choice, got {}-{}",
            choice.min_choices, choice.max_choices
        ));
    }

    let ctx = BottleContext::from_deck(&session.run_state.master_deck);
    let mut ranked = choice
        .selection_request(&session.run_state)
        .targets
        .iter()
        .filter_map(|target| {
            let uuid = target.card_uuid();
            session
                .run_state
                .master_deck
                .iter()
                .find(|card| card.uuid == uuid)
                .map(|card| (uuid, card, classify(choice.reason, card, &ctx)))
        })
        .collect::<Vec<_>>();
    if ranked.is_empty() {
        return OwnerDecision::Gap(format!("{:?} has no legal card target", choice.reason));
    }

    ranked.sort_by_key(|(_, card, rank)| (rank.sort_key(), card.uuid));
    let (uuid, card, rank) = ranked[0];
    OwnerDecision::Candidates(vec![OwnerChoice {
        key: None,
        action: RunControlCommand::Input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(SelectionScope::Deck, [uuid]),
        )),
        label: format!(
            "{:?} {} {} {}",
            choice.reason,
            card_label(card),
            class_label(rank.class),
            rank.reason
        ),
        annotation: ChoiceAnnotation::None,
        expansion: OwnerChoiceExpansion::AutoAllowed,
    }])
}

#[derive(Clone, Copy)]
struct Rank {
    class: i32,
    reason: &'static str,
    tie: i32,
}

impl Rank {
    fn sort_key(self) -> (i32, i32) {
        (self.class, self.tie)
    }
}

struct BottleContext {
    block_sources: usize,
    exhaust: bool,
    strength: bool,
    self_damage: bool,
}

impl BottleContext {
    fn from_deck(deck: &[CombatCard]) -> Self {
        let mut ctx = Self {
            block_sources: 0,
            exhaust: false,
            strength: false,
            self_damage: false,
        };
        for card in deck {
            let def = get_card_definition(card.id);
            ctx.block_sources += usize::from(def.base_block > 0);
            ctx.exhaust |= is_one_of(
                card.id,
                &[
                    CardId::FeelNoPain,
                    CardId::DarkEmbrace,
                    CardId::Corruption,
                    CardId::SecondWind,
                    CardId::BurningPact,
                    CardId::TrueGrit,
                    CardId::FiendFire,
                ],
            );
            ctx.strength |= is_one_of(
                card.id,
                &[
                    CardId::Inflame,
                    CardId::DemonForm,
                    CardId::Rupture,
                    CardId::LimitBreak,
                    CardId::SpotWeakness,
                ],
            );
            ctx.self_damage |= is_one_of(
                card.id,
                &[
                    CardId::Offering,
                    CardId::Bloodletting,
                    CardId::Combust,
                    CardId::Brutality,
                ],
            );
        }
        ctx
    }
}

fn classify(reason: RunPendingChoiceReason, card: &CombatCard, ctx: &BottleContext) -> Rank {
    match reason {
        RunPendingChoiceReason::BottleLightning => lightning(card),
        RunPendingChoiceReason::BottleFlame => flame(card, ctx),
        RunPendingChoiceReason::BottleTornado => tornado(card, ctx),
        _ => unreachable!("can_handle filters non-bottle reasons"),
    }
}

fn lightning(card: &CombatCard) -> Rank {
    let upgraded = card.upgrades > 0;
    let class = match card.id {
        CardId::Offering | CardId::BattleTrance => PREMIUM,
        CardId::BurningPact if upgraded => PREMIUM,
        CardId::BurningPact
        | CardId::Shockwave
        | CardId::Disarm
        | CardId::SeeingRed
        | CardId::Bloodletting => GOOD,
        CardId::Armaments if upgraded => GOOD,
        CardId::ShrugItOff
        | CardId::FlameBarrier
        | CardId::TrueGrit
        | CardId::Warcry
        | CardId::Armaments => FALLBACK,
        _ if is_starter_basic(card.id) => AVOID,
        _ => FALLBACK,
    };
    let reason = match card.id {
        CardId::Offering | CardId::BattleTrance | CardId::BurningPact => "turn_one_draw",
        CardId::SeeingRed | CardId::Bloodletting => "turn_one_energy",
        CardId::Shockwave | CardId::Disarm => "turn_one_engine",
        CardId::Armaments if upgraded => "upgrades_opening_hand",
        CardId::ShrugItOff | CardId::FlameBarrier | CardId::TrueGrit | CardId::Warcry => {
            "stable_block_draw"
        }
        _ => "starter_or_low_impact",
    };
    rank(class, reason, card)
}

fn flame(card: &CombatCard, ctx: &BottleContext) -> Rank {
    let upgraded = card.upgrades > 0;
    let class = match card.id {
        CardId::Immolate => PREMIUM,
        CardId::Whirlwind if upgraded || ctx.strength => PREMIUM,
        CardId::Whirlwind => GOOD,
        CardId::PommelStrike if upgraded => PREMIUM,
        CardId::PommelStrike | CardId::Feed => GOOD,
        CardId::Uppercut | CardId::Clothesline if upgraded => GOOD,
        CardId::Uppercut | CardId::Clothesline => FALLBACK,
        CardId::FiendFire if ctx.exhaust => GOOD,
        CardId::FiendFire => FALLBACK,
        CardId::BodySlam if upgraded && ctx.block_sources >= 3 => GOOD,
        CardId::BodySlam => AVOID,
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel if ctx.strength => GOOD,
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel => AVOID,
        CardId::PerfectedStrike | CardId::Rampage => FALLBACK,
        _ if is_starter_basic(card.id) => AVOID,
        _ => FALLBACK,
    };
    let reason = match card.id {
        CardId::Immolate | CardId::Whirlwind => "turn_one_aoe",
        CardId::PommelStrike => "turn_one_draw",
        CardId::Feed => "turn_one_scaling",
        CardId::Uppercut | CardId::Clothesline => "turn_one_engine",
        CardId::FiendFire
        | CardId::BodySlam
        | CardId::HeavyBlade
        | CardId::SwordBoomerang
        | CardId::Pummel
        | CardId::PerfectedStrike
        | CardId::Rampage => "conditional_payoff",
        _ => "starter_or_low_impact",
    };
    rank(class, reason, card)
}

fn tornado(card: &CombatCard, ctx: &BottleContext) -> Rank {
    let class = match card.id {
        CardId::Corruption => PREMIUM,
        CardId::DarkEmbrace if card.upgrades > 0 || ctx.exhaust => PREMIUM,
        CardId::DarkEmbrace | CardId::DemonForm | CardId::Inflame => GOOD,
        CardId::FeelNoPain if card.upgrades > 0 && ctx.exhaust => PREMIUM,
        CardId::FeelNoPain if ctx.exhaust => GOOD,
        CardId::FeelNoPain => FALLBACK,
        CardId::Barricade if ctx.block_sources >= 3 => GOOD,
        CardId::Barricade => AVOID,
        CardId::Rupture if ctx.self_damage => GOOD,
        CardId::Rupture => AVOID,
        CardId::Juggernaut if ctx.block_sources >= 3 => FALLBACK,
        CardId::Juggernaut => AVOID,
        _ => FALLBACK,
    };
    let reason = match card.id {
        CardId::Corruption | CardId::DarkEmbrace | CardId::FeelNoPain => "turn_one_engine",
        CardId::DemonForm | CardId::Inflame => "turn_one_scaling",
        CardId::Barricade | CardId::Rupture | CardId::Juggernaut => "conditional_payoff",
        _ => "starter_or_low_impact",
    };
    rank(class, reason, card)
}

fn rank(class: i32, reason: &'static str, card: &CombatCard) -> Rank {
    let def = get_card_definition(card.id);
    let cost_tie = if def.cost <= 1 {
        0
    } else {
        def.cost as i32 * 4
    };
    let upgrade_tie = if card.upgrades > 0 { -8 } else { 0 };
    let starter_tie = if is_starter_basic(card.id) { 30 } else { 0 };
    Rank {
        class,
        reason,
        tie: cost_tie + upgrade_tie + starter_tie,
    }
}

fn class_label(class: i32) -> &'static str {
    match class {
        PREMIUM => "premium",
        GOOD => "good",
        FALLBACK => "fallback",
        _ => "avoid",
    }
}

fn is_one_of(card: CardId, cards: &[CardId]) -> bool {
    cards.contains(&card)
}

fn card_label(card: &CombatCard) -> String {
    let name = get_card_definition(card.id).name;
    if card.upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{}", card.upgrades)
    }
}
