use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power};
use crate::state::EngineState;

pub(super) fn render_state_key(
    engine: &EngineState,
    combat: &CombatState,
    include_potions: bool,
    include_player_resources: bool,
    include_turn_state: bool,
    include_runtime_details: bool,
) -> String {
    let mut powers = combat
        .entities
        .power_db
        .iter()
        .map(|(entity_id, powers)| {
            let mut entries = powers.iter().map(power_signature).collect::<Vec<_>>();
            entries.sort();
            format!("{entity_id}:[{}]", entries.join(","))
        })
        .collect::<Vec<_>>();
    powers.sort();

    let potions = if include_potions {
        combat
            .entities
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref()
                    .map(|potion| format!("{:?}:{}", potion.id, potion.uuid))
                    .unwrap_or_else(|| "_".to_string())
            })
            .collect::<Vec<_>>()
            .join("|")
    } else {
        "_ignored_".to_string()
    };

    format!(
        concat!(
            "engine={:?};",
            "turn={};",
            "player=hp:{}:{}:blk:{}:stance:{:?}:relics:{:?}:buses:{:?};",
            "zones=draw:[{}];hand:[{}];disc:[{}];exhaust:[{}];limbo:[{}];queue:{:?};uuid:{};",
            "monsters=[{}];",
            "powers=[{}];",
            "potions=[{}];",
            "actions={:?};",
            "rng={:?};",
            "runtime={}"
        ),
        engine,
        if include_turn_state {
            format!("{:?}", combat.turn)
        } else {
            "_ignored_".to_string()
        },
        if include_player_resources {
            combat.entities.player.current_hp
        } else {
            -1
        },
        combat.entities.player.max_hp,
        if include_player_resources {
            combat.entities.player.block
        } else {
            -1
        },
        combat.entities.player.stance,
        combat.entities.player.relics,
        combat.entities.player.relic_buses,
        card_zone_signature(&combat.zones.draw_pile),
        card_zone_signature(&combat.zones.hand),
        card_zone_signature(&combat.zones.discard_pile),
        card_zone_signature(&combat.zones.exhaust_pile),
        card_zone_signature(&combat.zones.limbo),
        combat.zones.queued_cards,
        combat.zones.card_uuid_counter,
        combat
            .entities
            .monsters
            .iter()
            .map(monster_signature)
            .collect::<Vec<_>>()
            .join("|"),
        powers.join("|"),
        potions,
        if include_runtime_details {
            format!("{:?}", combat.engine.action_queue)
        } else {
            "_ignored_".to_string()
        },
        combat.rng.pool,
        if include_runtime_details {
            format!("{:?}", combat.runtime)
        } else {
            "_ignored_".to_string()
        },
    )
}

fn card_zone_signature(cards: &[CombatCard]) -> String {
    cards
        .iter()
        .map(card_signature)
        .collect::<Vec<_>>()
        .join("|")
}

fn card_signature(card: &CombatCard) -> String {
    format!(
        "{:?}:{}:u{}:misc{}:d{:?}:cm{}:ct{:?}:bd{}:bb{}:bm{}:md{:?}:ex{:?}:re{:?}:free{}:eou{}",
        card.id,
        card.uuid,
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

fn power_signature(power: &Power) -> String {
    format!(
        "{:?}:{:?}:{}:{}:{}",
        power.power_type, power.instance_id, power.amount, power.extra_data, power.just_applied,
    )
}

fn monster_signature(monster: &MonsterEntity) -> String {
    format!(
        concat!(
            "{}:{:?}:hp{}:{}:blk{}:slot{}:dy{}:esc{}:half{}:pos{}:",
            "move={:?}:hex={:?}:louse={:?}:jaw={:?}:thief={:?}:byrd={:?}:chosen={:?}:",
            "snecko={:?}:parasite={:?}:bronze_auto={:?}:bronze_orb={:?}:book={:?}:",
            "collector={:?}:champ={:?}:darkling={:?}:lagavulin={:?}:guardian={:?}"
        ),
        monster.id,
        monster.monster_type,
        monster.current_hp,
        monster.max_hp,
        monster.block,
        monster.slot,
        monster.is_dying,
        monster.is_escaped,
        monster.half_dead,
        monster.logical_position,
        monster.move_state,
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
        monster.darkling,
        monster.lagavulin,
        monster.guardian,
    )
}
