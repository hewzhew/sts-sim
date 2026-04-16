use crate::bot::card_taxonomy::taxonomy;
use crate::bot::combat_posture::{posture_features, CombatPostureFeatures};
use crate::bot::monster_belief::{build_combat_belief_state, total_damage_for_intent};
use crate::bot::strategy_families::{
    assess_branch_opening, assess_turn_action, branch_family_for_card, classify_turn_action,
    default_chance_profile, default_ordering_constraint, default_ordering_hint,
    default_risk_profile, BranchOpeningContext, BranchOpeningEstimate, ChanceProfile,
    OrderingConstraint, RiskProfile, TurnActionRole, TurnOrderingHint, TurnRiskContext,
    TurnSequencingContext,
};
use crate::runtime::combat::CombatState;
use crate::runtime::combat::Intent;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::potions::PotionId;
use crate::state::core::ClientInput;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatePressureFeatures {
    pub visible_incoming: i32,
    pub visible_unblocked: i32,
    pub belief_expected_incoming: i32,
    pub belief_expected_unblocked: i32,
    pub belief_max_incoming: i32,
    pub belief_max_unblocked: i32,
    pub value_incoming: i32,
    pub value_unblocked: i32,
    pub survival_guard_incoming: i32,
    pub survival_guard_unblocked: i32,
    pub incoming: i32,
    pub max_incoming: i32,
    pub unblocked: i32,
    pub max_unblocked: i32,
    pub player_hp: i32,
    pub lethal_pressure: bool,
    pub urgent_pressure: bool,
    pub hidden_intent_active: bool,
    pub attack_probability: f32,
    pub lethal_probability: f32,
    pub urgent_probability: f32,
    pub encounter_risk: bool,
}

impl StatePressureFeatures {
    pub(crate) fn from_combat(combat: &CombatState) -> Self {
        let belief = build_combat_belief_state(combat);
        let visible_incoming = visible_total_incoming_damage(combat);
        let visible_unblocked = (visible_incoming - combat.entities.player.block).max(0);
        let belief_expected_incoming = belief.expected_incoming_damage.round() as i32;
        let belief_expected_unblocked =
            (belief_expected_incoming - combat.entities.player.block).max(0);
        let belief_max_incoming = belief.max_incoming_damage;
        let belief_max_unblocked = (belief_max_incoming - combat.entities.player.block).max(0);
        let hidden_intent_active = belief.hidden_intent_active;
        let value_incoming = if hidden_intent_active {
            belief_expected_incoming
        } else {
            visible_incoming
        };
        let value_unblocked = (value_incoming - combat.entities.player.block).max(0);
        let survival_guard_incoming = if hidden_intent_active {
            belief_max_incoming
        } else {
            visible_incoming
        };
        let survival_guard_unblocked =
            (survival_guard_incoming - combat.entities.player.block).max(0);
        let player_hp = combat.entities.player.current_hp.max(1);
        let lethal_probability = if hidden_intent_active {
            belief.lethal_probability
        } else if visible_unblocked >= player_hp {
            1.0
        } else {
            0.0
        };
        let urgent_probability = if hidden_intent_active {
            belief.urgent_probability
        } else if visible_unblocked >= 8 || visible_unblocked >= player_hp {
            1.0
        } else {
            0.0
        };
        let lethal_pressure = survival_guard_unblocked >= player_hp
            || (hidden_intent_active && belief.lethal_probability >= 0.20);
        let urgent_pressure = lethal_pressure
            || value_unblocked >= 8
            || (hidden_intent_active && belief.urgent_probability >= 0.35);
        Self {
            visible_incoming,
            visible_unblocked,
            belief_expected_incoming,
            belief_expected_unblocked,
            belief_max_incoming,
            belief_max_unblocked,
            value_incoming,
            value_unblocked,
            survival_guard_incoming,
            survival_guard_unblocked,
            incoming: value_incoming,
            max_incoming: survival_guard_incoming,
            unblocked: value_unblocked,
            max_unblocked: survival_guard_unblocked,
            player_hp,
            lethal_pressure,
            urgent_pressure,
            hidden_intent_active,
            attack_probability: if hidden_intent_active {
                belief.attack_probability
            } else {
                visible_attack_probability(combat)
            },
            lethal_probability,
            urgent_probability,
            encounter_risk: combat.meta.is_elite_fight || combat.meta.is_boss_fight,
        }
    }
}

fn visible_total_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .filter(|monster| !matches!(monster.current_intent, Intent::Unknown))
        .map(|monster| total_damage_for_intent(&monster.current_intent))
        .sum()
}

fn visible_attack_probability(combat: &CombatState) -> f32 {
    let any_attack = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .any(|monster| {
            matches!(
                monster.current_intent,
                Intent::Attack { .. }
                    | Intent::AttackBuff { .. }
                    | Intent::AttackDebuff { .. }
                    | Intent::AttackDefend { .. }
            )
        });
    if any_attack {
        1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TransitionPressureDelta {
    pub block_gain: i32,
    pub incoming_reduction: i32,
    pub after_incoming: i32,
    pub after_unblocked: i32,
}

impl TransitionPressureDelta {
    pub(super) fn between(before: &CombatState, after: &CombatState) -> Self {
        let before_incoming = total_incoming_damage(before);
        let after_incoming = total_incoming_damage(after);
        Self {
            block_gain: (after.entities.player.block - before.entities.player.block).max(0),
            incoming_reduction: (before_incoming - after_incoming).max(0),
            after_incoming,
            after_unblocked: (after_incoming - after.entities.player.block).max(0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ActionSemanticTags {
    pub attack_like: bool,
    pub persistent_setup: bool,
    pub defensive_potion: bool,
    pub exhaust_engine: bool,
    pub exhaust_trigger: bool,
    pub block_core: bool,
    pub draw_core: bool,
    pub resource_bridge: bool,
    pub role: TurnActionRole,
    pub ordering_hint: TurnOrderingHint,
    pub chance_profile: ChanceProfile,
    pub risk_profile: RiskProfile,
    pub ordering_constraint: Option<OrderingConstraint>,
}

pub(super) fn action_semantic_tags(
    combat: &CombatState,
    input: &ClientInput,
) -> ActionSemanticTags {
    match input {
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                let card_type = get_card_definition(card.id).card_type;
                let tax = taxonomy(card.id);
                let role = classify_turn_action(card.id, card_type);
                ActionSemanticTags {
                    attack_like: card_type == CardType::Attack,
                    persistent_setup: tax.is_setup_power(),
                    defensive_potion: false,
                    exhaust_engine: tax.is_exhaust_engine(),
                    exhaust_trigger: tax.is_exhaust_outlet()
                        || matches!(
                            card.id,
                            CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                        ),
                    block_core: tax.is_block_core(),
                    draw_core: tax.is_draw_core(),
                    resource_bridge: tax.is_resource_conversion(),
                    role,
                    ordering_hint: default_ordering_hint(card.id, role),
                    chance_profile: default_chance_profile(card.id),
                    risk_profile: default_risk_profile(card.id, role),
                    ordering_constraint: default_ordering_constraint(card.id),
                }
            })
            .unwrap_or_default(),
        ClientInput::UsePotion { potion_index, .. } => ActionSemanticTags {
            defensive_potion: combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .is_some_and(|potion| {
                    matches!(
                        potion.id,
                        PotionId::BlockPotion
                            | PotionId::DexterityPotion
                            | PotionId::EssenceOfSteel
                            | PotionId::WeakenPotion
                            | PotionId::GhostInAJar
                    )
                }),
            role: TurnActionRole::DefensiveBridge,
            ordering_hint: TurnOrderingHint::PreferEarly,
            chance_profile: ChanceProfile::Deterministic,
            risk_profile: RiskProfile::WindowSensitive,
            ..ActionSemanticTags::default()
        },
        _ => ActionSemanticTags::default(),
    }
}

pub(super) fn posture_snapshot(combat: &CombatState) -> CombatPostureFeatures {
    posture_features(combat)
}

pub(super) fn same_turn_exhaust_setup_bonus_excluding(
    combat: &CombatState,
    exclude_card_index: usize,
    draw_engine: bool,
) -> i32 {
    let immediate_triggers = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != exclude_card_index)
        .filter(|(_, card)| action_semantic_tags_for_card(card.id).exhaust_trigger)
        .count() as i32;
    immediate_exhaust_setup_bonus_inner(
        immediate_triggers,
        junk_fuel_count(combat, Some(exclude_card_index)),
        draw_engine,
    )
}

pub(super) fn immediate_exhaust_setup_bonus(combat: &CombatState, draw_engine: bool) -> i32 {
    let immediate_triggers = combat
        .zones
        .hand
        .iter()
        .filter(|card| action_semantic_tags_for_card(card.id).exhaust_trigger)
        .count() as i32;
    immediate_exhaust_setup_bonus_inner(
        immediate_triggers,
        junk_fuel_count(combat, None),
        draw_engine,
    )
}

fn immediate_exhaust_setup_bonus_inner(
    immediate_triggers: i32,
    junk_fuel: i32,
    draw_engine: bool,
) -> i32 {
    if immediate_triggers <= 0 {
        return 0;
    }

    let mut bonus = 3_000 + immediate_triggers * if draw_engine { 1_900 } else { 1_300 };
    bonus += junk_fuel * if draw_engine { 2_200 } else { 1_000 };
    if draw_engine && junk_fuel > 0 {
        bonus += 1_600;
    }
    bonus
}

fn junk_fuel_count(combat: &CombatState, exclude_card_index: Option<usize>) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| Some(*idx) != exclude_card_index)
        .filter(|(_, card)| {
            matches!(
                card.id,
                CardId::Burn | CardId::Dazed | CardId::Slimed | CardId::Wound | CardId::Injury
            )
        })
        .count() as i32
}

fn action_semantic_tags_for_card(card_id: CardId) -> ActionSemanticTags {
    let card_type = get_card_definition(card_id).card_type;
    let tax = taxonomy(card_id);
    let role = classify_turn_action(card_id, card_type);
    ActionSemanticTags {
        attack_like: card_type == CardType::Attack,
        persistent_setup: tax.is_setup_power(),
        defensive_potion: false,
        exhaust_engine: tax.is_exhaust_engine(),
        exhaust_trigger: tax.is_exhaust_outlet()
            || matches!(
                card_id,
                CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
            ),
        block_core: tax.is_block_core(),
        draw_core: tax.is_draw_core(),
        resource_bridge: tax.is_resource_conversion(),
        role,
        ordering_hint: default_ordering_hint(card_id, role),
        chance_profile: default_chance_profile(card_id),
        risk_profile: default_risk_profile(card_id, role),
        ordering_constraint: default_ordering_constraint(card_id),
    }
}

pub(super) fn tactical_hint_bonus(combat: &CombatState, input: &ClientInput) -> f32 {
    match input {
        ClientInput::UsePotion { .. } => 0.0,
        ClientInput::EndTurn => 2_000.0,
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| match get_card_definition(card.id).card_type {
                CardType::Attack => 0.0,
                CardType::Power => 3_500.0,
                _ => 2_500.0,
            })
            .unwrap_or(0.0),
        _ => 2_000.0,
    }
}

pub(crate) fn sequencing_assessment_for_input(
    combat: &CombatState,
    input: &ClientInput,
    has_safe_line: bool,
) -> Option<crate::bot::strategy_families::SequencingAssessment> {
    let tags = action_semantic_tags(combat, input);
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    let _card = combat.zones.hand.get(*card_index)?;
    let current_energy = combat.turn.energy as i32;
    let remaining_actions = combat.zones.hand.len().saturating_sub(1) as i32;
    let pressure = StatePressureFeatures::from_combat(combat);
    let risk = TurnRiskContext {
        current_hp: combat.entities.player.current_hp,
        unblocked_damage: pressure.value_unblocked,
        defense_gap: pressure.value_unblocked,
        lethal_pressure: pressure.lethal_pressure,
        urgent_pressure: pressure.urgent_pressure,
        current_energy,
        remaining_actions,
        has_safe_line,
    };
    let branch = branch_opening_estimate(combat, input, &risk);
    let sequencing = TurnSequencingContext {
        role: tags.role,
        ordering_hint: tags.ordering_hint,
        chance_profile: tags.chance_profile,
        risk_profile: tags.risk_profile,
        ordering_constraint: tags.ordering_constraint,
        immediate_payoff: immediate_action_payoff(combat, input),
        followup_payoff: followup_payoff_estimate(combat, *card_index, input),
        growth_window: growth_window_available(combat, input),
    };
    Some(assess_turn_action(&sequencing, &risk, branch.as_ref()))
}

pub(super) fn sequencing_order_bonus(
    combat: &CombatState,
    input: &ClientInput,
    has_safe_line: bool,
) -> f32 {
    sequencing_assessment_for_input(combat, input, has_safe_line)
        .map(|assessment| assessment.total_delta() as f32)
        .unwrap_or(0.0)
}

pub(super) fn motif_transition_bonus(
    combat: &CombatState,
    input: &ClientInput,
    next_combat: &CombatState,
) -> f32 {
    let pressure = StatePressureFeatures::from_combat(combat);
    if pressure.value_incoming <= 0 {
        return 0.0;
    }

    let delta = TransitionPressureDelta::between(combat, next_combat);
    let tags = action_semantic_tags(combat, input);
    let mut bonus = 0.0;

    if pressure.urgent_pressure {
        bonus += delta.block_gain.min(pressure.value_unblocked).min(18) as f32 * 260.0;
        bonus += delta
            .incoming_reduction
            .min(pressure.value_incoming)
            .min(18) as f32
            * 220.0;
    }

    if tags.defensive_potion {
        bonus += if pressure.lethal_pressure {
            8_500.0
        } else {
            2_400.0
        };
        bonus += delta.block_gain.min(pressure.value_unblocked).min(20) as f32 * 300.0;
        bonus += delta
            .incoming_reduction
            .min(pressure.value_incoming)
            .min(20) as f32
            * 240.0;
        if delta.after_unblocked < pressure.value_unblocked {
            bonus += 1_000.0;
        }
    }

    if tags.persistent_setup && delta.block_gain == 0 && delta.incoming_reduction == 0 {
        bonus -= if pressure.lethal_pressure {
            9_000.0
        } else if pressure.urgent_pressure {
            3_000.0
        } else {
            0.0
        };
    }

    if tags.attack_like
        && delta.block_gain == 0
        && delta.incoming_reduction == 0
        && pressure.urgent_pressure
    {
        bonus -= if pressure.lethal_pressure {
            4_000.0
        } else {
            1_600.0
        };
    }

    bonus
}

fn branch_opening_estimate(
    combat: &CombatState,
    input: &ClientInput,
    risk: &TurnRiskContext,
) -> Option<BranchOpeningEstimate> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    let card = combat.zones.hand.get(*card_index)?;
    let def = get_card_definition(card.id);
    let branch_family = branch_family_for_card(card.id)?;
    let draw_count = match card.id {
        CardId::PommelStrike | CardId::ShrugItOff | CardId::Warcry => 1,
        CardId::BattleTrance => (def.base_magic + card.upgrades as i32 * def.upgrade_magic).max(0),
        CardId::MasterOfStrategy => {
            (def.base_magic + card.upgrades as i32 * def.upgrade_magic).max(0)
        }
        CardId::Offering => 3,
        CardId::BurningPact => 2,
        CardId::DeepBreath => 1,
        CardId::Discovery | CardId::InfernalBlade => 1,
        _ => 0,
    };
    if draw_count <= 0 {
        return None;
    }

    let future_cards: Vec<_> = combat
        .zones
        .draw_pile
        .iter()
        .chain(combat.zones.discard_pile.iter())
        .collect();
    let future_zero_cost_cards = future_cards
        .iter()
        .filter(|card| card.get_cost() <= 0)
        .count() as i32;
    let future_one_cost_cards = future_cards
        .iter()
        .filter(|card| card.get_cost() == 1)
        .count() as i32;
    let future_two_plus_cost_cards = future_cards
        .iter()
        .filter(|card| card.get_cost() >= 2 || card.get_cost() < 0)
        .count() as i32;
    let future_status_cards = future_cards
        .iter()
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32;
    let future_key_delay_weight = future_cards
        .iter()
        .map(|card| key_delay_weight(card.id))
        .sum();
    let future_high_cost_key_delay_weight = future_cards
        .iter()
        .filter(|card| card.get_cost() >= 2 || card.get_cost() < 0)
        .map(|card| key_delay_weight(card.id))
        .sum();
    let other_draw_sources_in_hand = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, hand_card)| {
            *idx != *card_index
                && taxonomy(hand_card.id).is_draw_core()
                && !matches!(hand_card.id, CardId::DarkEmbrace | CardId::Evolve)
        })
        .count() as i32;

    let draw_ctx = crate::bot::strategy_families::DrawTimingContext {
        current_energy: combat.turn.energy as i32,
        player_no_draw: combat.get_power(0, crate::runtime::combat::PowerId::NoDraw) > 0,
        current_hand_size: combat.zones.hand.len() as i32,
        future_zero_cost_cards,
        future_one_cost_cards,
        future_two_plus_cost_cards,
        future_key_delay_weight,
        future_high_cost_key_delay_weight,
        future_status_cards,
        other_draw_sources_in_hand,
    };
    let current_defensive_floor = immediate_defensive_floor(combat, input);
    let energy_after_play = combat.turn.energy as i32 - current_card_energy_cost(card);
    let hand_space_after_play = (10_i32 - (combat.zones.hand.len() as i32 - 1)).max(0);
    let remaining_attack_followups = count_hand_followups(combat, *card_index, true);
    let remaining_defensive_followups = count_hand_followups(combat, *card_index, false);

    Some(assess_branch_opening(&BranchOpeningContext {
        draw: draw_ctx,
        risk: *risk,
        draw_count,
        applies_no_draw: matches!(card.id, CardId::BattleTrance),
        current_safe_line_exists: risk.has_safe_line,
        current_defensive_floor,
        energy_after_play,
        hand_space_after_play,
        immediate_action_value: immediate_action_payoff(combat, input),
        remaining_attack_followups,
        remaining_defensive_followups,
        branch_family,
    }))
}

fn current_card_energy_cost(card: &crate::runtime::combat::CombatCard) -> i32 {
    match card.get_cost() {
        cost if cost < 0 => 0,
        cost => cost.into(),
    }
}

fn count_hand_followups(combat: &CombatState, exclude_idx: usize, attacks: bool) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != exclude_idx)
        .filter(|(_, card)| {
            let def = get_card_definition(card.id);
            if attacks {
                def.card_type == CardType::Attack
            } else {
                def.card_type == CardType::Skill
                    && ((def.base_block + card.upgrades as i32 * def.upgrade_block).max(0) > 0
                        || matches!(
                            card.id,
                            CardId::Disarm
                                | CardId::Shockwave
                                | CardId::Uppercut
                                | CardId::Clothesline
                                | CardId::ThunderClap
                                | CardId::Intimidate
                                | CardId::Blind
                                | CardId::DarkShackles
                                | CardId::Trip
                        ))
            }
        })
        .count() as i32
}

fn immediate_defensive_floor(combat: &CombatState, input: &ClientInput) -> i32 {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return 0;
    };
    let Some(card) = combat.zones.hand.get(*card_index) else {
        return 0;
    };
    let def = get_card_definition(card.id);
    let base_block = (def.base_block + card.upgrades as i32 * def.upgrade_block).max(0);
    base_block
        + match card.id {
            CardId::Disarm => 6,
            CardId::Shockwave | CardId::Uppercut => 7,
            CardId::Clothesline | CardId::Intimidate | CardId::Blind | CardId::Trip => 4,
            CardId::ThunderClap => 2,
            _ => 0,
        }
}

fn immediate_action_payoff(combat: &CombatState, input: &ClientInput) -> i32 {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return 0;
    };
    let Some(card) = combat.zones.hand.get(*card_index) else {
        return 0;
    };
    let def = get_card_definition(card.id);
    let block = (def.base_block + card.upgrades as i32 * def.upgrade_block).max(0);
    let damage = (def.base_damage
        + card.upgrades as i32 * def.upgrade_damage
        + combat.get_power(0, crate::runtime::combat::PowerId::Strength))
    .max(0);
    match def.card_type {
        CardType::Attack => damage * hits_for_card(card.id),
        CardType::Skill => block.max(0),
        CardType::Power => 0,
        _ => 0,
    }
}

fn followup_payoff_estimate(combat: &CombatState, current_idx: usize, input: &ClientInput) -> i32 {
    let current_card = match input {
        ClientInput::PlayCard { card_index, .. } => combat.zones.hand.get(*card_index),
        _ => None,
    };
    let current_card = match current_card {
        Some(card) => card,
        None => return 0,
    };
    if matches!(
        current_card.id,
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption
    ) {
        let exhaust_sources = combat
            .zones
            .hand
            .iter()
            .enumerate()
            .filter(|(idx, card)| {
                *idx != current_idx
                    && (taxonomy(card.id).is_exhaust_outlet()
                        || matches!(
                            card.id,
                            CardId::Offering | CardId::SeeingRed | CardId::InfernalBlade
                        ))
            })
            .count() as i32;
        let junk_fuel = combat
            .zones
            .hand
            .iter()
            .enumerate()
            .filter(|(idx, card)| {
                *idx != current_idx
                    && matches!(
                        card.id,
                        CardId::Burn
                            | CardId::Dazed
                            | CardId::Slimed
                            | CardId::Wound
                            | CardId::Injury
                    )
            })
            .count() as i32;
        return exhaust_sources * 14 + junk_fuel * 8;
    }
    let energy_after =
        (combat.turn.energy as i32 - i32::from(current_card.get_cost().max(0))).max(0);
    let mut best_attack_followup = 0;
    let mut best_high_cost_followup = 0;
    let mut best_multi_hit_followup = 0;
    let mut playable_attack_followups = Vec::new();
    for (idx, card) in combat.zones.hand.iter().enumerate() {
        if idx == current_idx {
            continue;
        }
        let def = get_card_definition(card.id);
        if def.card_type != CardType::Attack {
            continue;
        }
        let card_def = get_card_definition(card.id);
        let damage = (card_def.base_damage
            + card.upgrades as i32 * card_def.upgrade_damage
            + combat.get_power(0, crate::runtime::combat::PowerId::Strength))
        .max(0)
            * hits_for_card(card.id);
        let card_cost = i32::from(card.get_cost());
        if card_cost <= energy_after || (card_cost < 0 && energy_after > 0) {
            best_attack_followup = best_attack_followup.max(damage);
            playable_attack_followups.push(damage);
            if taxonomy(card.id).is_multi_hit() || taxonomy(card.id).is_attack_followup_priority() {
                best_multi_hit_followup = best_multi_hit_followup.max(damage);
            }
        }
        if card_cost >= 2 || card_cost < 0 {
            best_high_cost_followup = best_high_cost_followup.max(damage);
        }
    }
    playable_attack_followups.sort_unstable_by(|lhs, rhs| rhs.cmp(lhs));
    let cumulative_attack_followup: i32 = playable_attack_followups.iter().take(3).sum();
    match current_card.id {
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => best_high_cost_followup,
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip => best_multi_hit_followup.max(best_attack_followup),
        CardId::Rage | CardId::Flex => cumulative_attack_followup.max(best_attack_followup),
        _ if taxonomy(current_card.id).is_setup_power()
            =>
        {
            cumulative_attack_followup.max(best_attack_followup)
        }
        _ => best_attack_followup / 2,
    }
}

fn growth_window_available(combat: &CombatState, input: &ClientInput) -> bool {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return false;
    };
    let Some(card) = combat.zones.hand.get(*card_index) else {
        return false;
    };
    if !matches!(card.id, CardId::Feed | CardId::Reaper) {
        return false;
    }
    let def = get_card_definition(card.id);
    let damage = (def.base_damage
        + card.upgrades as i32 * def.upgrade_damage
        + combat.get_power(0, crate::runtime::combat::PowerId::Strength))
    .max(0);
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .any(|monster| (damage - monster.block.max(0)).max(0) >= monster.current_hp.max(0))
}

fn hits_for_card(card_id: CardId) -> i32 {
    match card_id {
        CardId::TwinStrike => 2,
        CardId::SwordBoomerang => 3,
        CardId::Pummel => 4,
        _ => 1,
    }
}

fn key_delay_weight(card_id: CardId) -> i32 {
    match card_id {
        CardId::Apparition
        | CardId::LimitBreak
        | CardId::Corruption
        | CardId::Barricade
        | CardId::DemonForm
        | CardId::Impervious
        | CardId::Reaper
        | CardId::Feed => 4,
        CardId::Offering => 1,
        CardId::DarkEmbrace
        | CardId::FeelNoPain
        | CardId::BurningPact
        | CardId::BodySlam
        | CardId::PowerThrough
        | CardId::FlameBarrier
        | CardId::GhostlyArmor
        | CardId::HeavyBlade
        | CardId::Exhume
        | CardId::BattleTrance => 3,
        CardId::ShrugItOff
        | CardId::PommelStrike
        | CardId::Disarm
        | CardId::Shockwave
        | CardId::Armaments
        | CardId::Warcry
        | CardId::SeeingRed => 2,
        _ => 0,
    }
}

pub(super) fn total_incoming_damage(combat: &CombatState) -> i32 {
    StatePressureFeatures::from_combat(combat).value_incoming
}
