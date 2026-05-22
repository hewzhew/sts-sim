use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power};
use crate::state::core::{EngineState, PendingChoice};

use super::types::{
    CombatDominanceKey, CombatDominancePlayerKey, CombatEntityPowersKey, CombatExactPlayerKey,
    CombatExactStateKey, CombatRuntimeKey, CombatZonesKey,
};

/// Exact in-combat runtime key used by Combat Search V2 transposition pruning.
/// This is stricter than `stable_outcome_key`: player hp/block, card
/// instances, queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_exact_runtime_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatExactStateKey {
    CombatExactStateKey {
        common: combat_runtime_key(engine, combat),
        player: player_exact_key(combat),
    }
}

/// In-combat bucket used by Combat Search V2 resource dominance pruning. This
/// is not an exact transposition key: current HP/block are intentionally left
/// out because they are compared through `ResourceVector`, but card instances,
/// queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatDominanceKey {
    CombatDominanceKey {
        common: combat_runtime_key(engine, combat),
        player: CombatDominancePlayerKey {
            future_relevant: player_non_hp_key(combat),
        },
    }
}

fn combat_runtime_key(engine: &EngineState, combat: &CombatState) -> CombatRuntimeKey {
    CombatRuntimeKey {
        engine: engine_key(engine),
        turn: turn_key(combat),
        meta: meta_key(combat),
        zones: zones_key(combat),
        monsters: monsters_key(combat),
        powers: powers_key(combat),
        potions: potions_key(combat),
        queue: queue_key(combat),
        runtime: runtime_key(combat),
        rng: format!("{:?}", combat.rng.pool),
    }
}

fn player_exact_key(combat: &CombatState) -> CombatExactPlayerKey {
    let player = &combat.entities.player;
    CombatExactPlayerKey {
        current_hp: player.current_hp,
        block: player.block,
        future_relevant: player_non_hp_key(combat),
    }
}

fn engine_key(engine: &EngineState) -> String {
    match engine {
        EngineState::PendingChoice(choice) => {
            format!("PendingChoice:{}", pending_choice_key(choice))
        }
        _ => format!("{engine:?}"),
    }
}

fn pending_choice_key(choice: &PendingChoice) -> String {
    format!("{choice:?}")
}

fn player_non_hp_key(combat: &CombatState) -> String {
    let player = &combat.entities.player;
    format!(
        "max_hp:{}|stance:{:?}|orbs:{:?}|max_orbs:{}|relics:{:?}|relic_buses:{:?}|energy_master:{}|gold:{}",
        player.max_hp,
        player.stance,
        player.orbs,
        player.max_orbs,
        player.relics,
        player.relic_buses,
        player.energy_master,
        player.gold,
    )
}

fn turn_key(combat: &CombatState) -> String {
    format!("{:?}", combat.turn)
}

fn meta_key(combat: &CombatState) -> String {
    format!("{:?}", combat.meta)
}

fn zones_key(combat: &CombatState) -> CombatZonesKey {
    CombatZonesKey {
        card_uuid_counter: combat.zones.card_uuid_counter,
        hand: zone_key(&combat.zones.hand),
        draw: zone_key(&combat.zones.draw_pile),
        discard: zone_key(&combat.zones.discard_pile),
        exhaust: zone_key(&combat.zones.exhaust_pile),
        limbo: zone_key(&combat.zones.limbo),
        queued: combat
            .zones
            .queued_cards
            .iter()
            .map(|queued| {
                format!(
                    "{}:{}:{:?}",
                    card_key(&queued.card),
                    target_label(combat, queued.target),
                    queued.source
                )
            })
            .collect(),
    }
}

fn zone_key(cards: &[CombatCard]) -> Vec<String> {
    cards.iter().map(card_key).collect()
}

fn card_key(card: &CombatCard) -> String {
    format!(
        "{:?}+{}#{}:misc{}:cost{}:{:?}:free{}",
        card.id,
        card.upgrades,
        card.uuid,
        card.misc_value,
        card.get_cost(),
        card.cost_for_turn,
        card.free_to_play_once
    )
}

fn target_label(combat: &CombatState, target: Option<usize>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn monsters_key(combat: &CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            format!(
                "id{}:type{}:hp{}:max{}:block{}:dying{}:escaped{}:half{}:move{}:hist{:?}:plan{:?}:runtime{:?}",
                monster.id,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_dying,
                monster.is_escaped,
                monster.half_dead,
                monster.planned_move_id(),
                monster.move_history(),
                monster.turn_plan(),
                monster_runtime_key(monster),
            )
        })
        .collect()
}

fn monster_runtime_key(monster: &MonsterEntity) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        monster.hexaghost,
        monster.louse,
        monster.jaw_worm,
        monster.thief,
        monster.byrd,
        monster.chosen,
        monster.snecko,
        monster.shelled_parasite,
        monster.bronze_automaton,
        monster.bronze_orb,
        monster.book_of_stabbing,
        monster.collector,
        monster.champ,
        monster.awakened_one,
        monster.corrupt_heart,
        monster.writhing_mass,
        monster.spiker,
        monster.spire_shield,
        monster.spire_spear,
        monster.slaver_red,
        monster.gremlin_leader,
        monster.gremlin_nob,
        monster.gremlin_wizard,
        monster.cultist,
        monster.sentry,
        monster.slime_boss,
        monster.large_slime,
        monster.spheric_guardian,
        monster.reptomancer,
        monster.darkling,
        monster.nemesis,
        monster.giant_head,
        monster.time_eater,
        monster.donu,
        monster.deca,
        monster.transient,
        monster.exploder,
        monster.maw,
    )
}

fn powers_key(combat: &CombatState) -> Vec<CombatEntityPowersKey> {
    let mut entries = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| {
            let powers = powers.iter().map(power_key).collect::<Vec<_>>();
            CombatEntityPowersKey {
                entity_id: *entity,
                powers,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.entity_id);
    entries
}

fn power_key(power: &Power) -> String {
    format!(
        "{:?}:inst{:?}:amount{}:extra{}:payload{:?}:just{}",
        power.power_type,
        power.instance_id,
        power.amount,
        power.extra_data,
        power.payload,
        power.just_applied
    )
}

fn potions_key(combat: &CombatState) -> Vec<String> {
    combat
        .entities
        .potions
        .iter()
        .enumerate()
        .map(|(index, potion)| match potion {
            Some(potion) => format!("{index}:{:?}:{}", potion.id, potion.uuid),
            None => format!("{index}:empty"),
        })
        .collect()
}

fn queue_key(combat: &CombatState) -> Vec<String> {
    combat
        .engine
        .action_queue
        .iter()
        .map(|action| format!("{action:?}"))
        .collect()
}

fn runtime_key(combat: &CombatState) -> String {
    format!("{:?}", combat.runtime)
}
