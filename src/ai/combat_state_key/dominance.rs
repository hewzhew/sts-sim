use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power};
use crate::state::core::{EngineState, PendingChoice};

use super::types::CombatDominanceKey;

/// Exact in-combat bucket used by Combat Search V2 for transposition and
/// resource dominance pruning. This is stricter than `stable_outcome_key`:
/// current HP/block are intentionally left out for dominance comparison, but
/// card instances, queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatDominanceKey {
    CombatDominanceKey(format!(
        "engine:{}|turn:{}|phase:{:?}|energy:{}|player:{}|zones:{}|monsters:{}|powers:{}|potions:{}|queue:{}|rng:{:?}",
        engine_key(engine),
        combat.turn.turn_count,
        combat.turn.current_phase,
        combat.turn.energy,
        player_non_hp_key(combat),
        zones_key(combat),
        monsters_key(combat),
        powers_key(combat),
        potions_key(combat),
        queue_key(combat),
        combat.rng.pool,
    ))
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
        "max_hp:{}|stance:{:?}|orbs:{:?}|relics:{}|energy_master:{}|gold:{}",
        player.max_hp,
        player.stance,
        player.orbs,
        player
            .relics
            .iter()
            .map(|relic| format!(
                "{:?}:{}:{}:{}",
                relic.id, relic.counter, relic.used_up, relic.amount
            ))
            .collect::<Vec<_>>()
            .join(","),
        player.energy_master,
        player.gold,
    )
}

fn zones_key(combat: &CombatState) -> String {
    format!(
        "hand:{}|draw:{}|discard:{}|exhaust:{}|limbo:{}|queued:{}",
        zone_key(&combat.zones.hand),
        zone_key(&combat.zones.draw_pile),
        zone_key(&combat.zones.discard_pile),
        zone_key(&combat.zones.exhaust_pile),
        zone_key(&combat.zones.limbo),
        combat
            .zones
            .queued_cards
            .iter()
            .map(|queued| format!(
                "{}:{}:{:?}",
                card_key(&queued.card),
                target_label(combat, queued.target),
                queued.source
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn zone_key(cards: &[CombatCard]) -> String {
    cards.iter().map(card_key).collect::<Vec<_>>().join(",")
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

fn monsters_key(combat: &CombatState) -> String {
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
        .collect::<Vec<_>>()
        .join("|")
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

fn powers_key(combat: &CombatState) -> String {
    let mut entries = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| {
            let mut power_keys = powers.iter().map(power_key).collect::<Vec<_>>();
            power_keys.sort();
            format!("{entity}:{}", power_keys.join(","))
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries.join("|")
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

fn potions_key(combat: &CombatState) -> String {
    combat
        .entities
        .potions
        .iter()
        .enumerate()
        .map(|(index, potion)| match potion {
            Some(potion) => format!("{index}:{:?}:{}", potion.id, potion.uuid),
            None => format!("{index}:empty"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn queue_key(combat: &CombatState) -> String {
    combat
        .engine
        .action_queue
        .iter()
        .map(|action| format!("{action:?}"))
        .collect::<Vec<_>>()
        .join(",")
}
