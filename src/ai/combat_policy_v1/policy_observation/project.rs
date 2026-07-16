use crate::content::cards::java_id;
use crate::runtime::combat::{
    CombatCard, CombatPhase, CombatState, OrbEntity, OrbId, Power, PowerPayload, StanceId,
};

use super::super::{
    combat_public_observation_v1, HiddenInformationReasonV1, ObservationEvidenceKindV1,
};
use super::types::{
    CombatPolicyCardPileV1, CombatPolicyCardV1, CombatPolicyEncounterV1,
    CombatPolicyMonsterRuntimeV1, CombatPolicyObservationV1, CombatPolicyOrbIdV1,
    CombatPolicyOrbV1, CombatPolicyPhaseV1, CombatPolicyPlayerRuntimeV1,
    CombatPolicyPowerPayloadV1, CombatPolicyPowerV1, CombatPolicyRelicV1, CombatPolicyStanceV1,
    CombatPolicyTurnCountersV1, CombatPolicyTurnV1, CombatPolicyZonesV1,
    COMBAT_POLICY_OBSERVATION_SCHEMA_NAME, COMBAT_POLICY_OBSERVATION_SCHEMA_VERSION,
};

pub fn combat_policy_observation_v1(combat: &CombatState) -> CombatPolicyObservationV1 {
    let draw_order_visible = combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::FrozenEye);
    let player_id = combat.entities.player.id;

    CombatPolicyObservationV1 {
        schema_name: COMBAT_POLICY_OBSERVATION_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_POLICY_OBSERVATION_SCHEMA_VERSION,
        compatibility_public: combat_public_observation_v1(combat),
        encounter: CombatPolicyEncounterV1 {
            is_boss_fight: combat.meta.is_boss_fight,
            is_elite_fight: combat.meta.is_elite_fight,
        },
        turn: CombatPolicyTurnV1 {
            turn_count: combat.turn.turn_count,
            phase: policy_phase(combat.turn.current_phase),
            turn_start_draw_modifier: combat.turn.turn_start_draw_modifier,
            counters: CombatPolicyTurnCountersV1 {
                cards_played_this_turn: combat.turn.counters.cards_played_this_turn,
                attacks_played_this_turn: combat.turn.counters.attacks_played_this_turn,
                cards_discarded_this_turn: combat.turn.counters.cards_discarded_this_turn,
                card_ids_played_this_turn: combat
                    .turn
                    .counters
                    .card_ids_played_this_turn
                    .iter()
                    .map(|id| java_id(*id).to_string())
                    .collect(),
                card_ids_played_this_combat: combat
                    .turn
                    .counters
                    .card_ids_played_this_combat
                    .iter()
                    .map(|id| java_id(*id).to_string())
                    .collect(),
                orbs_channeled_this_turn: combat
                    .turn
                    .counters
                    .orbs_channeled_this_turn
                    .iter()
                    .copied()
                    .map(policy_orb_id)
                    .collect(),
                orbs_channeled_this_combat: combat
                    .turn
                    .counters
                    .orbs_channeled_this_combat
                    .iter()
                    .copied()
                    .map(policy_orb_id)
                    .collect(),
                mantra_gained_this_combat: combat.turn.counters.mantra_gained_this_combat,
                times_damaged_this_combat: combat.turn.counters.times_damaged_this_combat,
                discovery_cost_for_turn: combat.turn.counters.discovery_cost_for_turn,
            },
        },
        player_runtime: CombatPolicyPlayerRuntimeV1 {
            gold: combat.entities.player.gold,
            gold_delta_this_combat: combat.entities.player.gold_delta_this_combat,
            facing_left: combat.entities.player.facing_left,
            energy_master: combat.entities.player.energy_master,
            stance: policy_stance(combat.entities.player.stance),
            max_orbs: combat.entities.player.max_orbs,
            orbs: combat.entities.player.orbs.iter().map(policy_orb).collect(),
            relics: combat
                .entities
                .player
                .relics
                .iter()
                .map(|relic| CombatPolicyRelicV1 {
                    relic_id: format!("{:?}", relic.id),
                    counter: relic.counter,
                    used_up: relic.used_up,
                    amount: relic.amount,
                })
                .collect(),
            powers: policy_powers(combat, player_id),
        },
        zones: CombatPolicyZonesV1 {
            hand: combat
                .zones
                .hand
                .iter()
                .map(combat_policy_card_v1)
                .collect(),
            draw: policy_pile(
                &combat.zones.draw_pile,
                if draw_order_visible {
                    ObservationEvidenceKindV1::PublicOrderedCollection
                } else {
                    ObservationEvidenceKindV1::PublicUnorderedCollection
                },
                (!draw_order_visible && !combat.zones.draw_pile.is_empty())
                    .then_some(HiddenInformationReasonV1::DrawPileOrderHidden),
            ),
            discard: policy_pile(
                &combat.zones.discard_pile,
                ObservationEvidenceKindV1::PublicUnorderedCollection,
                None,
            ),
            exhaust: policy_pile(
                &combat.zones.exhaust_pile,
                ObservationEvidenceKindV1::PublicUnorderedCollection,
                None,
            ),
        },
        monster_runtime: combat
            .entities
            .monsters
            .iter()
            .map(|monster| CombatPolicyMonsterRuntimeV1 {
                monster_slot: monster.slot,
                powers: policy_powers(combat, monster.id),
            })
            .collect(),
    }
}

fn policy_phase(phase: CombatPhase) -> CombatPolicyPhaseV1 {
    match phase {
        CombatPhase::PlayerTurn => CombatPolicyPhaseV1::PlayerTurn,
        CombatPhase::MonsterTurn => CombatPolicyPhaseV1::MonsterTurn,
        CombatPhase::TurnTransition => CombatPolicyPhaseV1::TurnTransition,
    }
}

fn policy_stance(stance: StanceId) -> CombatPolicyStanceV1 {
    match stance {
        StanceId::Neutral => CombatPolicyStanceV1::Neutral,
        StanceId::Wrath => CombatPolicyStanceV1::Wrath,
        StanceId::Calm => CombatPolicyStanceV1::Calm,
        StanceId::Divinity => CombatPolicyStanceV1::Divinity,
    }
}

fn policy_orb_id(orb_id: OrbId) -> CombatPolicyOrbIdV1 {
    match orb_id {
        OrbId::Empty => CombatPolicyOrbIdV1::Empty,
        OrbId::Lightning => CombatPolicyOrbIdV1::Lightning,
        OrbId::Dark => CombatPolicyOrbIdV1::Dark,
        OrbId::Frost => CombatPolicyOrbIdV1::Frost,
        OrbId::Plasma => CombatPolicyOrbIdV1::Plasma,
    }
}

fn policy_orb(orb: &OrbEntity) -> CombatPolicyOrbV1 {
    CombatPolicyOrbV1 {
        orb_id: policy_orb_id(orb.id),
        base_passive_amount: orb.base_passive_amount,
        base_evoke_amount: orb.base_evoke_amount,
        passive_amount: orb.passive_amount,
        evoke_amount: orb.evoke_amount,
    }
}

fn policy_powers(combat: &CombatState, entity_id: usize) -> Vec<CombatPolicyPowerV1> {
    combat
        .entities
        .power_db
        .get(&entity_id)
        .into_iter()
        .flatten()
        .map(policy_power)
        .collect()
}

fn policy_power(power: &Power) -> CombatPolicyPowerV1 {
    CombatPolicyPowerV1 {
        power_id: format!("{:?}", power.power_type),
        amount: power.amount,
        extra_data: power.extra_data,
        fresh_this_round: power.just_applied,
        payload: match &power.payload {
            PowerPayload::None => CombatPolicyPowerPayloadV1::None,
            PowerPayload::Card(card) => CombatPolicyPowerPayloadV1::Card {
                card: combat_policy_card_v1(card),
            },
        },
    }
}

fn policy_pile(
    cards: &[CombatCard],
    evidence: ObservationEvidenceKindV1,
    hidden_reason: Option<HiddenInformationReasonV1>,
) -> CombatPolicyCardPileV1 {
    let mut public_cards = cards.iter().map(combat_policy_card_v1).collect::<Vec<_>>();
    if evidence == ObservationEvidenceKindV1::PublicUnorderedCollection {
        public_cards.sort();
    }
    CombatPolicyCardPileV1 {
        count: cards.len(),
        evidence,
        hidden_reason,
        cards: public_cards,
    }
}

pub(crate) fn combat_policy_card_v1(card: &CombatCard) -> CombatPolicyCardV1 {
    CombatPolicyCardV1 {
        card_id: java_id(card.id).to_string(),
        upgrades: card.upgrades,
        misc_value: card.misc_value,
        base_damage_override: card.base_damage_override,
        base_block_override: card.base_block_override,
        cost_modifier: card.cost_modifier,
        combat_cost: card.combat_cost_without_turn_override_java(),
        cost_for_turn: card.cost_for_turn_java(),
        base_damage_mut: card.base_damage_mut,
        base_block_mut: card.base_block_mut,
        base_magic_num_mut: card.base_magic_num_mut,
        multi_damage: card.multi_damage.iter().copied().collect(),
        exhaust_override: card.exhaust_override,
        retain_override: card.retain_override,
        free_to_play_once: card.free_to_play_once,
        energy_on_use: card.energy_on_use,
    }
}
