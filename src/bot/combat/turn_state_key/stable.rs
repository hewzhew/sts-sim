use crate::runtime::combat::{
    CardZones, CombatCard, CombatState, PlayerEntity, Power, TurnRuntime,
};
use crate::state::EngineState;

use super::monster::stable_monster_signature;
use super::pending_choice::pending_choice_key;
use super::postcombat::{
    stable_boss_relic_key, stable_event_combat_key, stable_meta_key, stable_postcombat_player_key,
    stable_postcombat_runtime_key, stable_reward_key, stable_run_pending_choice_key,
    stable_run_result_signature, stable_shop_key,
};
use super::types::{
    StableCombatPlayerKey, StableEngineKey, StableOutcomeKey, StableOutcomePayload, StableTurnKey,
    StableZonesKey,
};
use super::{stable_frontier_scope, StableFrontierScope};

pub(super) fn build_stable_outcome_key(
    engine: &EngineState,
    combat: &CombatState,
) -> StableOutcomeKey {
    let scope = stable_frontier_scope(engine, combat);
    match scope {
        StableFrontierScope::CombatReady
        | StableFrontierScope::PendingChoice
        | StableFrontierScope::Unstable => StableOutcomeKey::new(
            scope,
            stable_combat_engine_key(engine, combat),
            StableOutcomePayload::Combat {
                turn: stable_turn_key(&combat.turn),
                player: stable_player_key(&combat.entities.player),
                zones: stable_zones_key(&combat.zones),
                monsters: combat
                    .entities
                    .monsters
                    .iter()
                    .map(stable_monster_signature)
                    .collect(),
                powers: stable_powers_signature(combat),
                rng: format!("{:?}", combat.rng.pool),
            },
        ),
        StableFrontierScope::PostCombat => StableOutcomeKey::new(
            scope,
            stable_postcombat_engine_key(engine, combat),
            StableOutcomePayload::PostCombat {
                player: stable_postcombat_player_key(&combat.entities.player),
                meta: stable_meta_key(&combat.meta),
                runtime: stable_postcombat_runtime_key(combat),
                rng: format!("{:?}", combat.rng.pool),
            },
        ),
        StableFrontierScope::GameOver => StableOutcomeKey::new(
            scope,
            stable_postcombat_engine_key(engine, combat),
            StableOutcomePayload::GameOver,
        ),
    }
}

pub(super) fn stable_card_signature(card: &CombatCard) -> String {
    format!(
        "{:?}:u{}:misc{}:d{:?}:cm{}:ct{:?}:bd{}:bb{}:bm{}:md{:?}:ex{:?}:re{:?}:free{}:eou{}",
        card.id,
        card.upgrades,
        card.misc_value,
        card.base_damage_override,
        card.cost_modifier,
        card.cost_for_turn,
        card.base_damage_mut,
        card.base_block_mut,
        card.base_magic_num_mut,
        card.multi_damage,
        card.exhaust_override,
        card.retain_override,
        card.free_to_play_once,
        card.energy_on_use,
    )
}

fn stable_combat_engine_key(engine: &EngineState, combat: &CombatState) -> StableEngineKey {
    match engine {
        EngineState::CombatPlayerTurn => StableEngineKey::CombatReady,
        EngineState::CombatProcessing => StableEngineKey::CombatProcessing,
        EngineState::PendingChoice(choice) => {
            StableEngineKey::PendingChoice(pending_choice_key(choice, combat))
        }
        _ => stable_postcombat_engine_key(engine, combat),
    }
}

fn stable_postcombat_engine_key(engine: &EngineState, combat: &CombatState) -> StableEngineKey {
    match engine {
        EngineState::RewardScreen(reward) => StableEngineKey::Reward(stable_reward_key(reward)),
        EngineState::Campfire => StableEngineKey::Campfire,
        EngineState::Shop(shop) => StableEngineKey::Shop(stable_shop_key(shop)),
        EngineState::MapNavigation => StableEngineKey::MapNavigation,
        EngineState::EventRoom => StableEngineKey::EventRoom,
        EngineState::RunPendingChoice(state) => {
            StableEngineKey::RunPendingChoice(stable_run_pending_choice_key(state))
        }
        EngineState::EventCombat(state) => {
            StableEngineKey::EventCombat(stable_event_combat_key(state))
        }
        EngineState::BossRelicSelect(state) => {
            StableEngineKey::BossRelic(stable_boss_relic_key(state))
        }
        EngineState::GameOver(result) => {
            StableEngineKey::GameOver(stable_run_result_signature(result))
        }
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => stable_combat_engine_key(engine, combat),
    }
}

fn stable_turn_key(turn: &TurnRuntime) -> StableTurnKey {
    StableTurnKey {
        turn_count: turn.turn_count,
        current_phase: format!("{:?}", turn.current_phase),
        energy: turn.energy,
        turn_start_draw_modifier: turn.turn_start_draw_modifier,
        cards_played_this_turn: turn.counters.cards_played_this_turn,
        attacks_played_this_turn: turn.counters.attacks_played_this_turn,
        times_damaged_this_combat: turn.counters.times_damaged_this_combat,
        victory_triggered: turn.counters.victory_triggered,
        discovery_cost_for_turn: turn.counters.discovery_cost_for_turn,
        early_end_turn_pending: turn.counters.early_end_turn_pending,
        player_escaping: turn.counters.player_escaping,
        escape_pending_reward: turn.counters.escape_pending_reward,
    }
}

fn stable_player_key(player: &PlayerEntity) -> StableCombatPlayerKey {
    StableCombatPlayerKey {
        max_hp: player.max_hp,
        orbs: format!("{:?}", player.orbs),
        max_orbs: player.max_orbs,
        stance: format!("{:?}", player.stance),
        relics: format!("{:?}", player.relics),
        relic_buses: format!("{:?}", player.relic_buses),
        energy_master: player.energy_master,
    }
}

fn stable_zones_key(zones: &CardZones) -> StableZonesKey {
    StableZonesKey {
        draw: stable_card_zone_key(&zones.draw_pile),
        hand: stable_card_zone_key(&zones.hand),
        discard: stable_card_zone_key(&zones.discard_pile),
        exhaust: stable_card_zone_key(&zones.exhaust_pile),
        limbo: stable_card_zone_key(&zones.limbo),
    }
}

fn stable_card_zone_key(cards: &[CombatCard]) -> Vec<String> {
    cards.iter().map(stable_card_signature).collect()
}

fn stable_powers_signature(combat: &CombatState) -> Vec<String> {
    let mut powers = combat
        .entities
        .power_db
        .iter()
        .map(|(entity_id, powers)| {
            let mut entries = powers
                .iter()
                .map(stable_power_signature)
                .collect::<Vec<_>>();
            entries.sort();
            format!("{entity_id}:[{}]", entries.join(","))
        })
        .collect::<Vec<_>>();
    powers.sort();
    powers
}

fn stable_power_signature(power: &Power) -> String {
    format!(
        "{:?}:{}:{}:{}",
        power.power_type, power.amount, power.extra_data, power.just_applied,
    )
}
