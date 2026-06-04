use crate::content::cards::{CardId, CardType};
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::facts::card_facts;
use super::types::{
    CardRewardPickDependencyV1, CardRewardRouteEvidenceV1, CardRewardRunContextV1,
    CardRewardSelectedRouteV1, DeckProfileV1,
};

pub(crate) fn run_context(run_state: &RunState) -> CardRewardRunContextV1 {
    CardRewardRunContextV1 {
        act: run_state.act_num,
        floor: run_state.floor_num,
        ascension: run_state.ascension_level,
        class: run_state.player_class.to_string(),
        boss: run_state.boss_key.map(|boss| format!("{boss:?}")),
        hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        gold: run_state.gold,
    }
}

pub(crate) fn deck_profile(run_state: &RunState) -> DeckProfileV1 {
    let mut profile = DeckProfileV1 {
        deck_size: run_state.master_deck.len(),
        attacks: 0,
        skills: 0,
        powers: 0,
        curses: 0,
        starter_strikes: 0,
        starter_defends: 0,
        total_attack_damage: 0,
        total_block: 0,
        draw_cards: 0,
        energy_sources: 0,
        strength_sources: 0,
        strength_payoffs: 0,
        vulnerable_sources: 0,
        weak_sources: 0,
        exhaust_generators: 0,
        exhaust_payoffs: 0,
        status_generators: 0,
        status_payoffs: 0,
        route_upgrade_payoffs: 0,
        important_cards_unupgraded: 0,
    };

    for card in &run_state.master_deck {
        let reward_card = RewardCard::new(card.id, card.upgrades);
        let facts = card_facts(&reward_card);
        match facts.card_type {
            CardType::Attack => profile.attacks = profile.attacks.saturating_add(1),
            CardType::Skill => profile.skills = profile.skills.saturating_add(1),
            CardType::Power => profile.powers = profile.powers.saturating_add(1),
            CardType::Curse => profile.curses = profile.curses.saturating_add(1),
            CardType::Status => {}
        }
        match facts.card {
            CardId::Strike | CardId::StrikeG | CardId::StrikeB | CardId::StrikeP => {
                profile.starter_strikes = profile.starter_strikes.saturating_add(1)
            }
            CardId::Defend | CardId::DefendG | CardId::DefendB | CardId::DefendP => {
                profile.starter_defends = profile.starter_defends.saturating_add(1)
            }
            _ => {}
        }
        profile.total_attack_damage = profile
            .total_attack_damage
            .saturating_add(facts.damage.total_damage);
        profile.total_block = profile.total_block.saturating_add(facts.block);
        if facts.draw_cards > 0 {
            profile.draw_cards = profile.draw_cards.saturating_add(1);
        }
        if facts.energy_gain > 0 {
            profile.energy_sources = profile.energy_sources.saturating_add(1);
        }
        if facts.strength_gain > 0 {
            profile.strength_sources = profile.strength_sources.saturating_add(1);
        }
        if facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::StrengthScaling)
        {
            profile.strength_payoffs = profile.strength_payoffs.saturating_add(1);
        }
        if facts.vulnerable > 0 {
            profile.vulnerable_sources = profile.vulnerable_sources.saturating_add(1);
        }
        if facts.weak > 0 {
            profile.weak_sources = profile.weak_sources.saturating_add(1);
        }
        if facts.exhausts_other_cards {
            profile.exhaust_generators = profile.exhaust_generators.saturating_add(1);
        }
        if facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::ExhaustPackage)
        {
            profile.exhaust_payoffs = profile.exhaust_payoffs.saturating_add(1);
        }
        if facts.adds_status_cards > 0 {
            profile.status_generators = profile.status_generators.saturating_add(1);
        }
        if facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::StatusPackage)
        {
            profile.status_payoffs = profile.status_payoffs.saturating_add(1);
        }
        if facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::RouteUpgradeDensity)
        {
            profile.route_upgrade_payoffs = profile.route_upgrade_payoffs.saturating_add(1);
        }
        if card.upgrades == 0 && matches!(card.id, CardId::Bash) {
            profile.important_cards_unupgraded =
                profile.important_cards_unupgraded.saturating_add(1);
        }
    }

    profile
}

pub(crate) fn route_evidence(
    trace: Option<&crate::ai::route_planner_v1::RouteDecisionTraceV1>,
) -> Option<CardRewardRouteEvidenceV1> {
    let trace = trace?;
    let selected = trace
        .selected_index
        .and_then(|idx| trace.candidates.get(idx))
        .map(|candidate| CardRewardSelectedRouteV1 {
            next_x: candidate.target.x,
            next_y: candidate.target.y,
            min_fires: candidate.path_summary.min_fires,
            max_fires: candidate.path_summary.max_fires,
            first_fire_floor: candidate.path_summary.first_fire_floor,
            min_elites: candidate.path_summary.min_elites,
            max_elites: candidate.path_summary.max_elites,
            min_early_pressure: candidate.path_summary.min_early_pressure,
            max_early_pressure: candidate.path_summary.max_early_pressure,
        });
    let selected_candidate = trace
        .selected_index
        .and_then(|idx| trace.candidates.get(idx));

    Some(CardRewardRouteEvidenceV1 {
        route_policy: "route_planner_v1".to_string(),
        selected_route: selected,
        candidate_count: trace.candidates.len(),
        need_card_rewards: selected_candidate
            .map(|candidate| candidate.needs.need_card_rewards)
            .unwrap_or(0.0),
        need_upgrade: selected_candidate
            .map(|candidate| candidate.needs.need_upgrade)
            .unwrap_or(0.0),
        need_heal: selected_candidate
            .map(|candidate| candidate.needs.need_heal)
            .unwrap_or(0.0),
        can_take_elite: selected_candidate
            .map(|candidate| candidate.needs.can_take_elite)
            .unwrap_or(0.0),
        avoid_damage: selected_candidate
            .map(|candidate| candidate.needs.avoid_damage)
            .unwrap_or(0.0),
        warnings: trace.warnings.clone(),
    })
}

pub(crate) fn strategy_route_future(
    route: Option<&CardRewardRouteEvidenceV1>,
) -> Option<crate::ai::noncombat_strategy_v1::StrategyRouteFutureV1> {
    let route = route?;
    let selected = route.selected_route.as_ref()?;
    Some(crate::ai::noncombat_strategy_v1::StrategyRouteFutureV1 {
        min_fires: selected.min_fires,
        max_fires: selected.max_fires,
        first_fire_floor: selected.first_fire_floor,
        max_early_pressure: selected.max_early_pressure,
        need_heal: route.need_heal,
        avoid_damage: route.avoid_damage,
    })
}

pub(crate) fn strategy_candidate_facts(
    facts: &super::types::CardRewardFactsV1,
) -> crate::ai::noncombat_strategy_v1::StrategyCandidateFactsV1 {
    crate::ai::noncombat_strategy_v1::StrategyCandidateFactsV1 {
        card: facts.card,
        damage_total: facts.damage.total_damage,
        weak: facts.weak,
        strength_gain: facts.strength_gain,
    }
}
