use serde_json::Value;
use std::collections::HashMap;

use crate::content::monsters::EnemyId;
use crate::protocol::java::{intent_from_java, monster_id_from_java};
use crate::runtime::combat::{MonsterEntity, MonsterProtocolState};

fn runtime_state<'a>(monster: &'a Value, monster_type: EnemyId) -> &'a Value {
    monster.get("runtime_state").unwrap_or_else(|| {
        panic!("strict state_sync: monster.runtime_state missing for {monster_type:?}")
    })
}

fn runtime_state_bool(monster: &Value, monster_type: EnemyId, key: &str) -> bool {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

fn runtime_state_u8(monster: &Value, monster_type: EnemyId, key: &str) -> u8 {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as u8)
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

fn runtime_state_i32(monster: &Value, monster_type: EnemyId, key: &str) -> i32 {
    runtime_state(monster, monster_type)
        .get(key)
        .and_then(|value| value.as_i64())
        .map(|value| value as i32)
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

pub(crate) fn seed_hexaghost_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Hexaghost;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.hexaghost.activated = runtime_state_bool(monster, monster_type, "activated");
    entity.hexaghost.orb_active_count = runtime_state_u8(monster, monster_type, "orb_active_count");
    entity.hexaghost.burn_upgraded = runtime_state_bool(monster, monster_type, "burn_upgraded");
    entity.hexaghost.divider_damage =
        match runtime_state_i32(monster, monster_type, "divider_damage") {
            value if value >= 0 => Some(value),
            _ => None,
        };
}

pub(crate) fn seed_darkling_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Darkling;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.darkling.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.darkling.nip_dmg = runtime_state_i32(monster, monster_type, "nip_dmg");
    entity.darkling.protocol_seeded = true;
}

pub(crate) fn seed_nemesis_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Nemesis;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.nemesis.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.nemesis.scythe_cooldown = runtime_state_i32(monster, monster_type, "scythe_cooldown");
    entity.nemesis.protocol_seeded = true;
}

pub(crate) fn seed_giant_head_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::GiantHead;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.giant_head.count = runtime_state_i32(monster, monster_type, "count");
    entity.giant_head.protocol_seeded = true;
}

pub(crate) fn seed_time_eater_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::TimeEater;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.time_eater.used_haste = runtime_state_bool(monster, monster_type, "used_haste");
    entity.time_eater.protocol_seeded = true;
}

pub(crate) fn seed_donu_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Donu;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.donu.is_attacking = runtime_state_bool(monster, monster_type, "is_attacking");
    entity.donu.protocol_seeded = true;
}

pub(crate) fn seed_deca_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Deca;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.deca.is_attacking = runtime_state_bool(monster, monster_type, "is_attacking");
    entity.deca.protocol_seeded = true;
}

pub(crate) fn seed_transient_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Transient;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.transient.count = runtime_state_i32(monster, monster_type, "count");
    entity.transient.protocol_seeded = true;
}

pub(crate) fn seed_exploder_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Exploder;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.exploder.turn_count = runtime_state_i32(monster, monster_type, "turn_count");
    entity.exploder.protocol_seeded = true;
}

pub(crate) fn seed_maw_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Maw;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.maw.roared = runtime_state_bool(monster, monster_type, "roared");
    entity.maw.turn_count = runtime_state_i32(monster, monster_type, "turn_count");
    entity.maw.protocol_seeded = true;
}

pub(crate) fn seed_awakened_one_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::AwakenedOne;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.awakened_one.form1 = runtime_state_bool(monster, monster_type, "form1");
    entity.awakened_one.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.awakened_one.protocol_seeded = true;
}

pub(crate) fn seed_corrupt_heart_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::CorruptHeart;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.corrupt_heart.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.corrupt_heart.move_count = runtime_state_u8(monster, monster_type, "move_count");
    entity.corrupt_heart.buff_count = runtime_state_u8(monster, monster_type, "buff_count");
    entity.corrupt_heart.blood_hit_count =
        runtime_state_u8(monster, monster_type, "blood_hit_count");
    entity.corrupt_heart.protocol_seeded = true;
}

pub(crate) fn seed_writhing_mass_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::WrithingMass;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.writhing_mass.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.writhing_mass.used_mega_debuff =
        runtime_state_bool(monster, monster_type, "used_mega_debuff");
    entity.writhing_mass.protocol_seeded = true;
}

pub(crate) fn seed_snake_dagger_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::SnakeDagger;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.snake_dagger.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.snake_dagger.protocol_seeded = true;
}

pub(crate) fn seed_spiker_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Spiker;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.spiker.thorns_count = runtime_state_u8(monster, monster_type, "thorns_count");
    entity.spiker.protocol_seeded = true;
}

pub(crate) fn seed_spire_shield_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::SpireShield;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.spire_shield.move_count = runtime_state_u8(monster, monster_type, "move_count");
    entity.spire_shield.protocol_seeded = true;
}

pub(crate) fn seed_spire_spear_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::SpireSpear;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.spire_spear.move_count = runtime_state_u8(monster, monster_type, "move_count");
    entity.spire_spear.skewer_count = runtime_state_u8(monster, monster_type, "skewer_count");
    entity.spire_spear.protocol_seeded = true;
}

pub(crate) fn seed_slaver_red_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::SlaverRed;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.slaver_red.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.slaver_red.used_entangle = runtime_state_bool(monster, monster_type, "used_entangle");
    entity.slaver_red.protocol_seeded = true;
}

pub(crate) fn seed_gremlin_nob_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::GremlinNob;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.gremlin_nob.used_bellow = runtime_state_bool(monster, monster_type, "used_bellow");
    entity.gremlin_nob.protocol_seeded = true;
}

pub(crate) fn seed_gremlin_wizard_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::GremlinWizard;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.gremlin_wizard.current_charge =
        runtime_state_u8(monster, monster_type, "current_charge");
    entity.gremlin_wizard.protocol_seeded = true;
}

pub(crate) fn seed_cultist_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Cultist;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.cultist.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.cultist.protocol_seeded = true;
}

pub(crate) fn seed_sentry_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Sentry;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.sentry.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.sentry.protocol_seeded = true;
}

pub(crate) fn seed_jaw_worm_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::JawWorm;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.jaw_worm.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.jaw_worm.hard_mode = runtime_state_bool(monster, monster_type, "hard_mode");
    entity.jaw_worm.protocol_seeded = true;
}

pub(crate) fn seed_slime_boss_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::SlimeBoss;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.slime_boss.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.slime_boss.protocol_seeded = true;
}

pub(crate) fn seed_large_slime_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let Some(monster_type @ (EnemyId::AcidSlimeL | EnemyId::SpikeSlimeL)) =
        EnemyId::from_id(entity.monster_type)
    else {
        return;
    };

    entity.large_slime.split_triggered =
        runtime_state_bool(monster, monster_type, "split_triggered");
    entity.large_slime.protocol_seeded = true;
}

pub(crate) fn seed_spheric_guardian_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::SphericGuardian;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.spheric_guardian.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.spheric_guardian.second_move = runtime_state_bool(monster, monster_type, "second_move");
    entity.spheric_guardian.protocol_seeded = true;
}

pub(crate) fn seed_byrd_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Byrd;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.byrd.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.byrd.is_flying = runtime_state_bool(monster, monster_type, "is_flying");
    entity.byrd.protocol_seeded = true;
}

pub(crate) fn seed_louse_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    if !matches!(
        entity.monster_type,
        x if x == EnemyId::LouseNormal as usize || x == EnemyId::LouseDefensive as usize
    ) {
        return;
    }

    let monster_type = EnemyId::from_id(entity.monster_type)
        .expect("strict state_sync: louse monster type must be initialized before runtime seeding");
    entity.louse.bite_damage = Some(runtime_state_i32(monster, monster_type, "bite_damage"));
}

pub(crate) fn seed_chosen_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Chosen;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.chosen.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.chosen.used_hex = runtime_state_bool(monster, monster_type, "used_hex");
    entity.chosen.protocol_seeded = true;
}

pub(crate) fn seed_snecko_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Snecko;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.snecko.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.snecko.protocol_seeded = true;
}

pub(crate) fn seed_shelled_parasite_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::ShelledParasite;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.shelled_parasite.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.shelled_parasite.protocol_seeded = true;
}

pub(crate) fn seed_bronze_automaton_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::BronzeAutomaton;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.bronze_automaton.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.bronze_automaton.num_turns = runtime_state_u8(monster, monster_type, "num_turns");
    entity.bronze_automaton.protocol_seeded = true;
}

pub(crate) fn seed_bronze_orb_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::BronzeOrb;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.bronze_orb.used_stasis = runtime_state_bool(monster, monster_type, "used_stasis");
    entity.bronze_orb.protocol_seeded = true;
}

pub(crate) fn seed_book_of_stabbing_runtime_from_snapshot(
    monster: &Value,
    entity: &mut MonsterEntity,
) {
    let monster_type = EnemyId::BookOfStabbing;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.book_of_stabbing.stab_count = runtime_state_u8(monster, monster_type, "stab_count");
    entity.book_of_stabbing.protocol_seeded = true;
}

pub(crate) fn seed_collector_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::TheCollector;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.collector.initial_spawn = runtime_state_bool(monster, monster_type, "initial_spawn");
    entity.collector.ult_used = runtime_state_bool(monster, monster_type, "ult_used");
    entity.collector.turns_taken = runtime_state_u8(monster, monster_type, "turns_taken");
    entity.collector.protocol_seeded = true;
}

pub(crate) fn seed_collector_enemy_slots_from_snapshots(
    truth_monsters: &[Value],
    monster_protocol: &HashMap<usize, MonsterProtocolState>,
    monsters: &mut [MonsterEntity],
) {
    let mut entity_by_instance_id = HashMap::new();
    for monster in monsters.iter() {
        if let Some(instance_id) = monster_protocol
            .get(&monster.id)
            .and_then(|protocol| protocol.identity.instance_id)
        {
            entity_by_instance_id.insert(instance_id, monster.id);
        }
    }

    for (index, snapshot) in truth_monsters.iter().enumerate() {
        if monsters[index].monster_type != EnemyId::TheCollector as usize {
            continue;
        }
        let slots = runtime_state(snapshot, EnemyId::TheCollector)
            .get("enemy_slots")
            .and_then(|value| value.as_array())
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: monster.runtime_state.enemy_slots missing for TheCollector"
                )
            });
        let mut enemy_slots = [None, None];
        for slot in slots {
            let java_slot = slot
                .get("slot")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!("strict state_sync: Collector enemy_slots entry missing slot")
                });
            assert!(
                matches!(java_slot, 1 | 2),
                "strict state_sync: Collector enemySlots key must be 1 or 2"
            );
            let instance_id = slot
                .get("monster_instance_id")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: Collector enemy_slots entry missing monster_instance_id"
                    )
                });
            let entity_id = entity_by_instance_id
                .get(&instance_id)
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: Collector enemySlots referenced unknown monster_instance_id {instance_id}"
                    )
                });
            enemy_slots[java_slot as usize - 1] = Some(entity_id);
        }
        monsters[index].collector.enemy_slots = enemy_slots;
    }
}

pub(crate) fn seed_gremlin_leader_slots_from_snapshots(
    truth_monsters: &[Value],
    monster_protocol: &HashMap<usize, MonsterProtocolState>,
    monsters: &mut [MonsterEntity],
) {
    let mut entity_by_instance_id = HashMap::new();
    for monster in monsters.iter() {
        if let Some(instance_id) = monster_protocol
            .get(&monster.id)
            .and_then(|protocol| protocol.identity.instance_id)
        {
            entity_by_instance_id.insert(instance_id, monster.id);
        }
    }

    for (index, snapshot) in truth_monsters.iter().enumerate() {
        if monsters[index].monster_type != EnemyId::GremlinLeader as usize {
            continue;
        }
        let slots = runtime_state(snapshot, EnemyId::GremlinLeader)
            .get("gremlin_slots")
            .and_then(|value| value.as_array())
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: monster.runtime_state.gremlin_slots missing for GremlinLeader"
                )
            });
        let mut gremlin_slots = [None, None, None];
        for slot in slots {
            let java_slot = slot
                .get("slot")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!("strict state_sync: GremlinLeader gremlin_slots entry missing slot")
                });
            assert!(
                java_slot < 3,
                "strict state_sync: GremlinLeader gremlins slot must be 0, 1, or 2"
            );
            let instance_id = slot
                .get("monster_instance_id")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: GremlinLeader gremlin_slots entry missing monster_instance_id"
                    )
                });
            let entity_id = entity_by_instance_id
                .get(&instance_id)
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: GremlinLeader gremlins referenced unknown monster_instance_id {instance_id}"
                    )
                });
            gremlin_slots[java_slot as usize] = Some(entity_id);
        }
        monsters[index].gremlin_leader.gremlin_slots = gremlin_slots;
        monsters[index].gremlin_leader.protocol_seeded = true;
    }
}

pub(crate) fn seed_reptomancer_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Reptomancer;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.reptomancer.first_move = runtime_state_bool(monster, monster_type, "first_move");
    entity.reptomancer.protocol_seeded = true;
}

pub(crate) fn seed_reptomancer_dagger_slots_from_snapshots(
    truth_monsters: &[Value],
    monster_protocol: &HashMap<usize, MonsterProtocolState>,
    monsters: &mut [MonsterEntity],
) {
    let mut entity_by_instance_id = HashMap::new();
    for monster in monsters.iter() {
        if let Some(instance_id) = monster_protocol
            .get(&monster.id)
            .and_then(|protocol| protocol.identity.instance_id)
        {
            entity_by_instance_id.insert(instance_id, monster.id);
        }
    }

    for (index, snapshot) in truth_monsters.iter().enumerate() {
        if monsters[index].monster_type != EnemyId::Reptomancer as usize {
            continue;
        }
        let slots = runtime_state(snapshot, EnemyId::Reptomancer)
            .get("dagger_slots")
            .and_then(|value| value.as_array())
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: monster.runtime_state.dagger_slots missing for Reptomancer"
                )
            });
        let mut dagger_slots = [None, None, None, None];
        for slot in slots {
            let java_slot = slot
                .get("slot")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!("strict state_sync: Reptomancer dagger_slots entry missing slot")
                });
            assert!(
                java_slot < 4,
                "strict state_sync: Reptomancer daggers slot must be 0, 1, 2, or 3"
            );
            let instance_id = slot
                .get("monster_instance_id")
                .and_then(|value| value.as_u64())
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: Reptomancer dagger_slots entry missing monster_instance_id"
                    )
                });
            let entity_id = entity_by_instance_id
                .get(&instance_id)
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: Reptomancer daggers referenced unknown monster_instance_id {instance_id}"
                    )
                });
            dagger_slots[java_slot as usize] = Some(entity_id);
        }
        monsters[index].reptomancer.dagger_slots = dagger_slots;
        monsters[index].reptomancer.protocol_seeded = true;
    }
}

pub(crate) fn seed_champ_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Champ;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.champ.first_turn = runtime_state_bool(monster, monster_type, "first_turn");
    entity.champ.num_turns = runtime_state_u8(monster, monster_type, "num_turns");
    entity.champ.forge_times = runtime_state_u8(monster, monster_type, "forge_times");
    entity.champ.threshold_reached = runtime_state_bool(monster, monster_type, "threshold_reached");
    entity.champ.protocol_seeded = true;
}

pub(crate) fn seed_thief_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = match entity.monster_type {
        x if x == EnemyId::Looter as usize => EnemyId::Looter,
        x if x == EnemyId::Mugger as usize => EnemyId::Mugger,
        _ => return,
    };

    entity.thief.slash_count = runtime_state_u8(monster, monster_type, "slash_count");
    entity.thief.stolen_gold = runtime_state_i32(monster, monster_type, "stolen_gold");
    entity.thief.protocol_seeded = true;
}

pub(crate) fn seed_lagavulin_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::Lagavulin;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.lagavulin.idle_count = runtime_state_u8(monster, monster_type, "idle_count");
    entity.lagavulin.debuff_turn_count =
        runtime_state_u8(monster, monster_type, "debuff_turn_count");
    entity.lagavulin.is_out = runtime_state_bool(monster, monster_type, "is_out");
    entity.lagavulin.is_out_triggered =
        runtime_state_bool(monster, monster_type, "is_out_triggered");
}

pub(crate) fn seed_guardian_runtime_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let monster_type = EnemyId::TheGuardian;
    if entity.monster_type != monster_type as usize {
        return;
    }

    entity.guardian.damage_threshold =
        runtime_state_i32(monster, monster_type, "guardian_threshold");
    entity.guardian.damage_taken = runtime_state_i32(monster, monster_type, "damage_taken");
    entity.guardian.is_open = runtime_state_bool(monster, monster_type, "is_open");
    entity.guardian.close_up_triggered =
        runtime_state_bool(monster, monster_type, "close_up_triggered");
}

pub(crate) fn seed_move_history_from_snapshot(monster: &Value, entity: &mut MonsterEntity) {
    let current_move_id = monster.get("move_id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let second_last_move_id = monster
        .get("second_last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let last_move_id = monster
        .get("last_move_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    entity.move_history_mut().clear();
    if second_last_move_id != 0 {
        entity.move_history_mut().push_back(second_last_move_id);
    }
    if last_move_id != 0 {
        entity.move_history_mut().push_back(last_move_id);
    }
    if current_move_id != 0 {
        entity.move_history_mut().push_back(current_move_id);
    }
}

fn snapshot_monster_is_half_dead(monster: &Value) -> bool {
    if monster
        .get("half_dead")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return true;
    }

    if !monster
        .get("is_gone")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return false;
    }

    let monster_id = monster["id"].as_str().unwrap_or("");
    let move_id = monster.get("move_id").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    (monster_id == "AwakenedOne" && move_id == 3)
        || (monster_id == "Darkling" && matches!(move_id, 4 | 5))
}

fn snapshot_i32(monster: &Value, key: &str) -> Option<i32> {
    monster
        .get(key)
        .and_then(|v| v.as_i64().map(|value| value as i32))
        .or_else(|| {
            monster
                .get(key)
                .and_then(|v| v.as_f64().map(|value| value.round() as i32))
        })
}

fn snapshot_bool_required(monster: &Value, key: &str) -> bool {
    monster
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| panic!("strict state_sync: monster.{key} missing"))
}

pub(crate) fn monster_protocol_state_from_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    let instance_id = monster.get("monster_instance_id").and_then(|v| v.as_u64());
    let spawn_order = monster
        .get("spawn_order")
        .and_then(|v| v.as_u64())
        .or(instance_id);
    let draw_x = snapshot_i32(monster, "draw_x");
    let group_index = monster
        .get("monster_index")
        .and_then(|v| v.as_u64())
        .map(|value| value as usize)
        .or(Some(index));

    if let Some(draw_x) = draw_x {
        entity.logical_position = draw_x;
    } else if entity.logical_position == 0 {
        entity.logical_position = index as i32;
    }

    let intent_base_damage = monster
        .get("move_base_damage")
        .and_then(|v| v.as_i64())
        .or_else(|| monster.get("move_adjusted_damage").and_then(|v| v.as_i64()))
        .unwrap_or(-1) as i32;
    let intent_hits = monster["move_hits"].as_i64().unwrap_or(1) as i32;
    let intent_str = monster["intent"].as_str().unwrap_or("UNKNOWN");

    crate::runtime::combat::MonsterProtocolState {
        observation: crate::runtime::combat::MonsterProtocolObservationState {
            visible_intent: intent_from_java(intent_str, intent_base_damage, intent_hits),
            preview_damage_per_hit: monster["move_adjusted_damage"].as_i64().unwrap_or(0) as i32,
        },
        identity: crate::runtime::combat::MonsterProtocolIdentity {
            instance_id,
            spawn_order,
            draw_x,
            group_index,
        },
    }
}

pub(crate) fn apply_monster_truth_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) {
    let half_dead = snapshot_monster_is_half_dead(monster);
    let is_gone = monster
        .get("is_gone")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_dying = snapshot_bool_required(monster, "is_dying");
    let is_escaping = snapshot_bool_required(monster, "is_escaping");
    let is_escaped = snapshot_bool_required(monster, "is_escaped");

    entity.current_hp = monster["current_hp"]
        .as_i64()
        .unwrap_or(monster["hp"].as_i64().unwrap_or(0)) as i32;
    entity.max_hp = monster["max_hp"].as_i64().unwrap_or(0) as i32;
    entity.block = monster["block"].as_i64().unwrap_or(0) as i32;
    entity.slot = index as u8;
    entity.is_dying = is_dying;
    entity.is_escaped = is_escaping || is_escaped;
    entity.half_dead = half_dead;
    assert_eq!(
        is_gone,
        is_dying || half_dead || is_escaping,
        "strict state_sync: monster.is_gone must match Java isDeadOrEscaped()"
    );
    entity.monster_type = monster_id_from_java(monster["id"].as_str().unwrap_or(""))
        .map(|e| e as usize)
        .unwrap_or(0);
    entity.set_planned_move_id(monster["move_id"].as_u64().unwrap_or(0) as u8);
    seed_move_history_from_snapshot(monster, entity);
    seed_hexaghost_runtime_from_snapshot(monster, entity);
    seed_byrd_runtime_from_snapshot(monster, entity);
    seed_louse_runtime_from_snapshot(monster, entity);
    seed_chosen_runtime_from_snapshot(monster, entity);
    seed_snecko_runtime_from_snapshot(monster, entity);
    seed_shelled_parasite_runtime_from_snapshot(monster, entity);
    seed_bronze_automaton_runtime_from_snapshot(monster, entity);
    seed_bronze_orb_runtime_from_snapshot(monster, entity);
    seed_book_of_stabbing_runtime_from_snapshot(monster, entity);
    seed_collector_runtime_from_snapshot(monster, entity);
    seed_champ_runtime_from_snapshot(monster, entity);
    seed_thief_runtime_from_snapshot(monster, entity);
    seed_writhing_mass_runtime_from_snapshot(monster, entity);
    seed_snake_dagger_runtime_from_snapshot(monster, entity);
    seed_spiker_runtime_from_snapshot(monster, entity);
    seed_spire_shield_runtime_from_snapshot(monster, entity);
    seed_spire_spear_runtime_from_snapshot(monster, entity);
    seed_slaver_red_runtime_from_snapshot(monster, entity);
    seed_gremlin_nob_runtime_from_snapshot(monster, entity);
    seed_gremlin_wizard_runtime_from_snapshot(monster, entity);
    seed_cultist_runtime_from_snapshot(monster, entity);
    seed_sentry_runtime_from_snapshot(monster, entity);
    seed_jaw_worm_runtime_from_snapshot(monster, entity);
    seed_slime_boss_runtime_from_snapshot(monster, entity);
    seed_large_slime_runtime_from_snapshot(monster, entity);
    seed_spheric_guardian_runtime_from_snapshot(monster, entity);
    seed_reptomancer_runtime_from_snapshot(monster, entity);
    seed_darkling_runtime_from_snapshot(monster, entity);
    seed_nemesis_runtime_from_snapshot(monster, entity);
    seed_giant_head_runtime_from_snapshot(monster, entity);
    seed_time_eater_runtime_from_snapshot(monster, entity);
    seed_donu_runtime_from_snapshot(monster, entity);
    seed_deca_runtime_from_snapshot(monster, entity);
    seed_transient_runtime_from_snapshot(monster, entity);
    seed_exploder_runtime_from_snapshot(monster, entity);
    seed_maw_runtime_from_snapshot(monster, entity);
    seed_awakened_one_runtime_from_snapshot(monster, entity);
    seed_corrupt_heart_runtime_from_snapshot(monster, entity);
    seed_lagavulin_runtime_from_snapshot(monster, entity);
    seed_guardian_runtime_from_snapshot(monster, entity);
}

pub(crate) fn apply_monster_observation_snapshot(
    monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    monster_protocol_state_from_snapshot(monster, index, entity)
}

pub(crate) fn apply_monster_split_snapshot(
    truth_monster: &Value,
    observation_monster: &Value,
    index: usize,
    entity: &mut MonsterEntity,
) -> crate::runtime::combat::MonsterProtocolState {
    apply_monster_truth_snapshot(truth_monster, index, entity);
    apply_monster_observation_snapshot(observation_monster, index, entity)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        apply_monster_observation_snapshot, apply_monster_split_snapshot,
        apply_monster_truth_snapshot, seed_collector_enemy_slots_from_snapshots,
        seed_gremlin_leader_slots_from_snapshots, seed_reptomancer_dagger_slots_from_snapshots,
    };
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{
        ByrdRuntimeState, ChosenRuntimeState, DarklingRuntimeState, DecaRuntimeState,
        DonuRuntimeState, GiantHeadRuntimeState, GuardianRuntimeState, HexaghostRuntimeState,
        LagavulinRuntimeState, MonsterEntity, MonsterMoveState, NemesisRuntimeState,
        ShelledParasiteRuntimeState, SneckoRuntimeState, TimeEaterRuntimeState,
    };

    fn blank_monster_entity() -> MonsterEntity {
        MonsterEntity {
            id: 1,
            monster_type: 0,
            current_hp: 0,
            max_hp: 0,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            move_state: MonsterMoveState::default(),
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: Default::default(),
            jaw_worm: Default::default(),
            thief: Default::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: Default::default(),
            bronze_orb: Default::default(),
            book_of_stabbing: Default::default(),
            collector: Default::default(),
            champ: Default::default(),
            awakened_one: Default::default(),
            corrupt_heart: Default::default(),
            writhing_mass: Default::default(),
            spiker: Default::default(),
            spire_shield: Default::default(),
            spire_spear: Default::default(),
            slaver_red: Default::default(),
            gremlin_leader: Default::default(),
            gremlin_nob: Default::default(),
            gremlin_wizard: Default::default(),
            cultist: Default::default(),
            sentry: Default::default(),
            slime_boss: Default::default(),
            large_slime: Default::default(),
            spheric_guardian: Default::default(),
            reptomancer: Default::default(),
            darkling: DarklingRuntimeState::default(),
            nemesis: NemesisRuntimeState::default(),
            giant_head: GiantHeadRuntimeState::default(),
            time_eater: TimeEaterRuntimeState::default(),
            donu: DonuRuntimeState::default(),
            deca: DecaRuntimeState::default(),
            transient: Default::default(),
            exploder: Default::default(),
            maw: Default::default(),
            snake_dagger: Default::default(),
            lagavulin: LagavulinRuntimeState::default(),
            guardian: GuardianRuntimeState::default(),
        }
    }

    fn chosen_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Chosen",
            "current_hp": 95,
            "max_hp": 95,
            "block": 11,
            "move_id": 3,
            "move_base_damage": 5,
            "move_hits": 2,
            "second_last_move_id": 1,
            "last_move_id": 2,
            "powers": [],
            "runtime_state": {
                "first_turn": false,
                "used_hex": true
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn snecko_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Snecko",
            "current_hp": 118,
            "max_hp": 118,
            "block": 0,
            "move_id": 1,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_turn": true
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn shelled_parasite_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Shelled Parasite",
            "current_hp": 70,
            "max_hp": 70,
            "block": 14,
            "intent": "ATTACK_BUFF",
            "move_id": 3,
            "move_base_damage": 12,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn darkling_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Darkling",
            "current_hp": 48,
            "max_hp": 56,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 11,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false,
                "nip_dmg": 11
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn reptomancer_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Reptomancer",
            "current_hp": 190,
            "max_hp": 190,
            "block": 0,
            "intent": "UNKNOWN",
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false,
                "dagger_slots": []
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn nemesis_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Nemesis",
            "current_hp": 185,
            "max_hp": 185,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 45,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false,
                "scythe_cooldown": 2
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn giant_head_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "GiantHead",
            "current_hp": 500,
            "max_hp": 500,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 2,
            "move_base_damage": 40,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "count": 0
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn time_eater_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "TimeEater",
            "current_hp": 200,
            "max_hp": 456,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 2,
            "move_base_damage": 8,
            "move_hits": 3,
            "powers": [],
            "runtime_state": {
                "used_haste": true
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn donu_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Donu",
            "current_hp": 250,
            "max_hp": 250,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 0,
            "move_base_damage": 10,
            "move_hits": 2,
            "powers": [],
            "runtime_state": {
                "is_attacking": true
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn deca_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Deca",
            "current_hp": 250,
            "max_hp": 250,
            "block": 0,
            "intent": "DEFEND",
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "is_attacking": false
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn transient_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Transient",
            "current_hp": 999,
            "max_hp": 999,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 1,
            "move_base_damage": 70,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "count": 4
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn exploder_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Exploder",
            "current_hp": 31,
            "max_hp": 35,
            "block": 0,
            "intent": "UNKNOWN",
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "turn_count": 2
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn maw_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Maw",
            "current_hp": 260,
            "max_hp": 300,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 5,
            "move_base_damage": 5,
            "move_hits": 3,
            "powers": [],
            "runtime_state": {
                "roared": true,
                "turn_count": 6
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn awakened_one_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "AwakenedOne",
            "current_hp": 0,
            "max_hp": 320,
            "block": 0,
            "intent": "UNKNOWN",
            "move_id": 3,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "form1": false,
                "first_turn": true
            },
            "is_gone": true,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": true
        })
    }

    fn corrupt_heart_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "CorruptHeart",
            "current_hp": 650,
            "max_hp": 800,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 1,
            "move_base_damage": 2,
            "move_hits": 15,
            "powers": [],
            "runtime_state": {
                "first_move": false,
                "move_count": 4,
                "buff_count": 2,
                "blood_hit_count": 15
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn lagavulin_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Lagavulin",
            "current_hp": 90,
            "max_hp": 115,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 20,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "idle_count": 2,
                "debuff_turn_count": 1,
                "is_out": true,
                "is_out_triggered": true
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn mugger_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Mugger",
            "current_hp": 50,
            "max_hp": 50,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 1,
            "move_base_damage": 11,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "slash_count": 1,
                "stolen_gold": 20
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn guardian_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "TheGuardian",
            "current_hp": 240,
            "max_hp": 240,
            "block": 0,
            "intent": "DEFEND",
            "move_id": 6,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [
                {
                    "id": "Mode Shift",
                    "name": "Mode Shift",
                    "amount": 30
                }
            ],
            "runtime_state": {
                "guardian_threshold": 30,
                "damage_taken": 0,
                "is_open": true,
                "close_up_triggered": false
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn collector_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "TheCollector",
            "current_hp": 282,
            "max_hp": 282,
            "block": 0,
            "move_id": 2,
            "move_base_damage": 18,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "initial_spawn": false,
                "ult_used": true,
                "turns_taken": 4,
                "enemy_slots": []
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn book_of_stabbing_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BookOfStabbing",
            "current_hp": 170,
            "max_hp": 170,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 7,
            "move_hits": 3,
            "last_move_id": 2,
            "second_last_move_id": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "stab_count": 3
            }
        })
    }

    fn bronze_automaton_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BronzeAutomaton",
            "current_hp": 300,
            "max_hp": 300,
            "block": 0,
            "move_id": 5,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_turn": false,
                "num_turns": 3
            }
        })
    }

    fn bronze_orb_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "BronzeOrb",
            "current_hp": 55,
            "max_hp": 55,
            "block": 0,
            "move_id": 3,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "used_stasis": true
            }
        })
    }

    fn champ_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Champ",
            "current_hp": 420,
            "max_hp": 420,
            "block": 0,
            "move_id": 4,
            "move_base_damage": 14,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_turn": false,
                "num_turns": 3,
                "forge_times": 1,
                "threshold_reached": true
            }
        })
    }

    fn writhing_mass_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "WrithingMass",
            "current_hp": 160,
            "max_hp": 160,
            "block": 0,
            "move_id": 4,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false,
                "used_mega_debuff": true
            }
        })
    }

    fn snake_dagger_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Dagger",
            "current_hp": 22,
            "max_hp": 25,
            "block": 0,
            "move_id": 2,
            "move_base_damage": 25,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    fn spiker_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Spiker",
            "current_hp": 52,
            "max_hp": 52,
            "block": 0,
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "thorns_count": 5
            }
        })
    }

    fn spire_shield_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "SpireShield",
            "current_hp": 125,
            "max_hp": 125,
            "block": 0,
            "move_id": 3,
            "move_base_damage": 38,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "move_count": 5
            }
        })
    }

    fn spire_spear_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "SpireSpear",
            "current_hp": 180,
            "max_hp": 180,
            "block": 0,
            "move_id": 3,
            "move_base_damage": 10,
            "move_hits": 4,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "move_count": 4,
                "skewer_count": 4
            }
        })
    }

    fn slaver_red_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "SlaverRed",
            "current_hp": 50,
            "max_hp": 50,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 13,
            "move_hits": 1,
            "last_move_id": 2,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_turn": false,
                "used_entangle": true
            }
        })
    }

    fn gremlin_nob_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "GremlinNob",
            "current_hp": 86,
            "max_hp": 86,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 14,
            "move_hits": 1,
            "last_move_id": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "used_bellow": true
            }
        })
    }

    fn gremlin_wizard_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "GremlinWizard",
            "current_hp": 24,
            "max_hp": 24,
            "block": 0,
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "last_move_id": 2,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "current_charge": 2
            }
        })
    }

    fn cultist_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Cultist",
            "current_hp": 48,
            "max_hp": 48,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 6,
            "move_hits": 1,
            "last_move_id": 3,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false
            }
        })
    }

    fn sentry_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "Sentry",
            "current_hp": 40,
            "max_hp": 40,
            "block": 0,
            "move_id": 4,
            "move_base_damage": 9,
            "move_hits": 1,
            "last_move_id": 3,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false
            }
        })
    }

    fn spheric_guardian_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "SphericGuardian",
            "current_hp": 20,
            "max_hp": 20,
            "block": 40,
            "move_id": 4,
            "move_base_damage": 10,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false,
                "second_move": true
            }
        })
    }

    fn jaw_worm_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "JawWorm",
            "current_hp": 44,
            "max_hp": 44,
            "block": 0,
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "last_move_id": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false,
                "hard_mode": true
            }
        })
    }

    fn slime_boss_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "SlimeBoss",
            "current_hp": 150,
            "max_hp": 150,
            "block": 0,
            "move_id": 4,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_turn": false
            }
        })
    }

    fn acid_slime_l_truth_snapshot() -> serde_json::Value {
        json!({
            "id": "AcidSlime_L",
            "current_hp": 34,
            "max_hp": 70,
            "block": 0,
            "move_id": 3,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "split_triggered": true
            }
        })
    }

    fn chosen_observation_snapshot() -> serde_json::Value {
        json!({
            "id": "Chosen",
            "current_hp": 95,
            "max_hp": 95,
            "block": 11,
            "intent": "ATTACK_DEBUFF",
            "move_adjusted_damage": 8,
            "move_hits": 2,
            "monster_instance_id": 42,
            "spawn_order": 77,
            "monster_index": 4,
            "draw_x": 900,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        })
    }

    #[test]
    fn truth_import_does_not_populate_observation_fields() {
        let snapshot = chosen_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 1, &mut entity);

        assert_eq!(entity.current_hp, 95);
        assert_eq!(entity.max_hp, 95);
        assert_eq!(entity.block, 11);
        assert_eq!(entity.slot, 1);
        assert_eq!(entity.planned_move_id(), 3);
        assert_eq!(
            entity.move_history().iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(!entity.chosen.first_turn);
        assert!(entity.chosen.used_hex);
        assert!(entity.chosen.protocol_seeded);
        assert_eq!(entity.logical_position, 0);
    }

    #[test]
    fn observation_import_populates_visible_intent_fields() {
        let snapshot = chosen_observation_snapshot();
        let mut entity = blank_monster_entity();

        let protocol = apply_monster_observation_snapshot(&snapshot, 1, &mut entity);

        assert_eq!(
            protocol.observation.visible_intent,
            crate::runtime::combat::Intent::AttackDebuff { damage: 8, hits: 2 }
        );
        assert_eq!(protocol.observation.preview_damage_per_hit, 8);
        assert_eq!(protocol.identity.instance_id, Some(42));
        assert_eq!(protocol.identity.spawn_order, Some(77));
        assert_eq!(protocol.identity.draw_x, Some(900));
        assert_eq!(protocol.identity.group_index, Some(4));
        assert_eq!(entity.logical_position, 900);
    }

    #[test]
    fn split_import_combines_truth_and_observation() {
        let truth = chosen_truth_snapshot();
        let observation = chosen_observation_snapshot();
        let mut entity = blank_monster_entity();

        let protocol = apply_monster_split_snapshot(&truth, &observation, 1, &mut entity);

        assert_eq!(entity.current_hp, 95);
        assert_eq!(
            protocol.observation.visible_intent,
            crate::runtime::combat::Intent::AttackDebuff { damage: 8, hits: 2 }
        );
        assert_eq!(protocol.observation.preview_damage_per_hit, 8);
        assert_eq!(
            entity.move_history().iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(protocol.identity.instance_id, Some(42));
        assert!(entity.chosen.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_snecko_runtime_first_turn() {
        let snapshot = snecko_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert!(entity.snecko.first_turn);
        assert!(entity.snecko.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_shelled_parasite_runtime_first_move() {
        let snapshot = shelled_parasite_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert!(!entity.shelled_parasite.first_move);
        assert!(entity.shelled_parasite.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_darkling_runtime_state() {
        let snapshot = darkling_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert!(!entity.darkling.first_move);
        assert_eq!(entity.darkling.nip_dmg, 11);
        assert!(entity.darkling.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_reptomancer_runtime_state() {
        let snapshot = reptomancer_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert!(!entity.reptomancer.first_move);
        assert!(entity.reptomancer.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_nemesis_runtime_state() {
        let snapshot = nemesis_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert!(!entity.nemesis.first_move);
        assert_eq!(entity.nemesis.scythe_cooldown, 2);
        assert!(entity.nemesis.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_giant_head_runtime_state() {
        let snapshot = giant_head_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert_eq!(entity.giant_head.count, 0);
        assert!(entity.giant_head.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_time_eater_runtime_state() {
        let snapshot = time_eater_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert!(entity.time_eater.used_haste);
        assert!(entity.time_eater.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_donu_runtime_state() {
        let snapshot = donu_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 0);
        assert!(entity.donu.is_attacking);
        assert!(entity.donu.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_deca_runtime_state() {
        let snapshot = deca_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert!(!entity.deca.is_attacking);
        assert!(entity.deca.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_transient_runtime_state() {
        let snapshot = transient_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert_eq!(entity.transient.count, 4);
        assert!(entity.transient.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_exploder_runtime_state() {
        let snapshot = exploder_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert_eq!(entity.exploder.turn_count, 2);
        assert!(entity.exploder.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_maw_runtime_state() {
        let snapshot = maw_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 5);
        assert!(entity.maw.roared);
        assert_eq!(entity.maw.turn_count, 6);
        assert!(entity.maw.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_awakened_one_runtime_state() {
        let snapshot = awakened_one_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert!(!entity.awakened_one.form1);
        assert!(entity.awakened_one.first_turn);
        assert!(entity.awakened_one.protocol_seeded);
        assert!(entity.half_dead);
    }

    #[test]
    fn truth_import_seeds_corrupt_heart_runtime_state() {
        let snapshot = corrupt_heart_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert!(!entity.corrupt_heart.first_move);
        assert_eq!(entity.corrupt_heart.move_count, 4);
        assert_eq!(entity.corrupt_heart.buff_count, 2);
        assert_eq!(entity.corrupt_heart.blood_hit_count, 15);
        assert!(entity.corrupt_heart.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_lagavulin_full_runtime_state() {
        let snapshot = lagavulin_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Lagavulin as usize);
        assert_eq!(entity.lagavulin.idle_count, 2);
        assert_eq!(entity.lagavulin.debuff_turn_count, 1);
        assert!(entity.lagavulin.is_out);
        assert!(entity.lagavulin.is_out_triggered);
    }

    #[test]
    fn truth_import_maps_java_escaping_to_rust_escaped_not_dying() {
        let snapshot = json!({
            "id": "GremlinWarrior",
            "current_hp": 20,
            "max_hp": 24,
            "block": 0,
            "move_id": 99,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": true,
            "is_dying": false,
            "is_escaping": true,
            "is_escaped": false,
            "half_dead": false
        });
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::GremlinWarrior as usize);
        assert!(!entity.is_dying);
        assert!(!entity.half_dead);
        assert!(entity.is_escaped);
    }

    #[test]
    fn truth_import_seeds_reptomancer_dagger_slots_from_instance_ids() {
        let reptomancer_snapshot = json!({
            "id": "Reptomancer",
            "current_hp": 190,
            "max_hp": 190,
            "block": 0,
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false,
                "dagger_slots": [
                    {
                        "slot": 0,
                        "monster_instance_id": 101,
                        "monster_id": "Dagger",
                        "is_dying": false
                    },
                    {
                        "slot": 1,
                        "monster_instance_id": 202,
                        "monster_id": "Dagger",
                        "is_dying": true
                    }
                ]
            }
        });
        let dagger_one_snapshot = json!({
            "id": "Dagger",
            "monster_instance_id": 101,
            "current_hp": 20,
            "max_hp": 20,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 9,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "first_move": false
            }
        });
        let dagger_two_snapshot = json!({
            "id": "Dagger",
            "monster_instance_id": 202,
            "current_hp": 0,
            "max_hp": 20,
            "block": 0,
            "move_id": 2,
            "move_base_damage": 25,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "first_move": false
            },
            "is_gone": true,
            "is_dying": true,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });

        let mut monsters = vec![
            blank_monster_entity(),
            blank_monster_entity(),
            blank_monster_entity(),
        ];
        for (index, snapshot) in [
            &reptomancer_snapshot,
            &dagger_one_snapshot,
            &dagger_two_snapshot,
        ]
        .into_iter()
        .enumerate()
        {
            apply_monster_truth_snapshot(snapshot, index, &mut monsters[index]);
            monsters[index].id = index + 1;
        }
        let mut protocol = std::collections::HashMap::new();
        for (index, instance_id) in [10, 101, 202].into_iter().enumerate() {
            protocol.insert(
                index + 1,
                crate::runtime::combat::MonsterProtocolState {
                    observation: Default::default(),
                    identity: crate::runtime::combat::MonsterProtocolIdentity {
                        instance_id: Some(instance_id),
                        spawn_order: Some(instance_id),
                        draw_x: None,
                        group_index: Some(index),
                    },
                },
            );
        }

        seed_reptomancer_dagger_slots_from_snapshots(
            &[
                reptomancer_snapshot,
                dagger_one_snapshot,
                dagger_two_snapshot,
            ],
            &protocol,
            &mut monsters,
        );

        assert_eq!(
            monsters[0].reptomancer.dagger_slots,
            [Some(2), Some(3), None, None]
        );
        assert!(monsters[0].reptomancer.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_mugger_thief_runtime() {
        let snapshot = mugger_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 1);
        assert_eq!(entity.thief.slash_count, 1);
        assert_eq!(entity.thief.stolen_gold, 20);
        assert!(entity.thief.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_guardian_runtime_state() {
        let snapshot = guardian_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 6);
        assert_eq!(entity.guardian.damage_threshold, 30);
        assert_eq!(entity.guardian.damage_taken, 0);
        assert!(entity.guardian.is_open);
        assert!(!entity.guardian.close_up_triggered);
    }

    #[test]
    fn truth_import_seeds_collector_runtime_state() {
        let snapshot = collector_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 2);
        assert!(!entity.collector.initial_spawn);
        assert!(entity.collector.ult_used);
        assert_eq!(entity.collector.turns_taken, 4);
        assert!(entity.collector.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_collector_enemy_slots_from_instance_ids() {
        let collector_snapshot = json!({
            "id": "TheCollector",
            "current_hp": 282,
            "max_hp": 282,
            "block": 0,
            "move_id": 5,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "initial_spawn": false,
                "ult_used": true,
                "turns_taken": 5,
                "enemy_slots": [
                    {
                        "slot": 1,
                        "monster_instance_id": 101,
                        "monster_id": "TorchHead",
                        "is_dying": true
                    },
                    {
                        "slot": 2,
                        "monster_instance_id": 202,
                        "monster_id": "TorchHead",
                        "is_dying": false
                    }
                ]
            }
        });
        let torch_one_snapshot = json!({
            "id": "TorchHead",
            "monster_instance_id": 101,
            "current_hp": 0,
            "max_hp": 40,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 7,
            "move_hits": 1,
            "powers": [],
            "is_gone": true,
            "is_dying": true,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });
        let torch_two_snapshot = json!({
            "id": "TorchHead",
            "monster_instance_id": 202,
            "current_hp": 40,
            "max_hp": 40,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 7,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });

        let mut monsters = vec![
            blank_monster_entity(),
            blank_monster_entity(),
            blank_monster_entity(),
        ];
        for (index, snapshot) in [
            &collector_snapshot,
            &torch_one_snapshot,
            &torch_two_snapshot,
        ]
        .into_iter()
        .enumerate()
        {
            apply_monster_truth_snapshot(snapshot, index, &mut monsters[index]);
            monsters[index].id = index + 1;
        }
        let mut protocol = std::collections::HashMap::new();
        for (index, instance_id) in [10, 101, 202].into_iter().enumerate() {
            protocol.insert(
                index + 1,
                crate::runtime::combat::MonsterProtocolState {
                    observation: Default::default(),
                    identity: crate::runtime::combat::MonsterProtocolIdentity {
                        instance_id: Some(instance_id),
                        spawn_order: Some(instance_id),
                        draw_x: None,
                        group_index: Some(index),
                    },
                },
            );
        }

        seed_collector_enemy_slots_from_snapshots(
            &[collector_snapshot, torch_one_snapshot, torch_two_snapshot],
            &protocol,
            &mut monsters,
        );

        assert_eq!(monsters[0].collector.enemy_slots, [Some(2), Some(3)]);
    }

    #[test]
    fn truth_import_seeds_gremlin_leader_slots_from_instance_ids() {
        let leader_snapshot = json!({
            "id": "GremlinLeader",
            "current_hp": 148,
            "max_hp": 148,
            "block": 0,
            "move_id": 2,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false,
            "runtime_state": {
                "gremlin_slots": [
                    {
                        "slot": 0,
                        "monster_instance_id": 101,
                        "monster_id": "GremlinWarrior",
                        "is_dying": false
                    },
                    {
                        "slot": 2,
                        "monster_instance_id": 303,
                        "monster_id": "GremlinWizard",
                        "is_dying": true
                    }
                ]
            }
        });
        let warrior_snapshot = json!({
            "id": "GremlinWarrior",
            "monster_instance_id": 101,
            "current_hp": 20,
            "max_hp": 20,
            "block": 0,
            "move_id": 1,
            "move_base_damage": 4,
            "move_hits": 1,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });
        let wizard_snapshot = json!({
            "id": "GremlinWizard",
            "monster_instance_id": 303,
            "current_hp": 0,
            "max_hp": 25,
            "block": 0,
            "move_id": 1,
            "move_base_damage": -1,
            "move_hits": 1,
            "powers": [],
            "runtime_state": {
                "current_charge": 2
            },
            "is_gone": true,
            "is_dying": true,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });

        let mut monsters = vec![
            blank_monster_entity(),
            blank_monster_entity(),
            blank_monster_entity(),
        ];
        for (index, snapshot) in [&leader_snapshot, &warrior_snapshot, &wizard_snapshot]
            .into_iter()
            .enumerate()
        {
            apply_monster_truth_snapshot(snapshot, index, &mut monsters[index]);
            monsters[index].id = index + 1;
        }
        let mut protocol = std::collections::HashMap::new();
        for (index, instance_id) in [10, 101, 303].into_iter().enumerate() {
            protocol.insert(
                index + 1,
                crate::runtime::combat::MonsterProtocolState {
                    observation: Default::default(),
                    identity: crate::runtime::combat::MonsterProtocolIdentity {
                        instance_id: Some(instance_id),
                        spawn_order: Some(instance_id),
                        draw_x: None,
                        group_index: Some(index),
                    },
                },
            );
        }

        seed_gremlin_leader_slots_from_snapshots(
            &[leader_snapshot, warrior_snapshot, wizard_snapshot],
            &protocol,
            &mut monsters,
        );

        assert_eq!(
            monsters[0].gremlin_leader.gremlin_slots,
            [Some(2), None, Some(3)]
        );
        assert!(monsters[0].gremlin_leader.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_book_of_stabbing_runtime_state() {
        let snapshot = book_of_stabbing_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BookOfStabbing as usize);
        assert_eq!(entity.book_of_stabbing.stab_count, 3);
        assert!(entity.book_of_stabbing.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_bronze_automaton_runtime_state() {
        let snapshot = bronze_automaton_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BronzeAutomaton as usize);
        assert!(!entity.bronze_automaton.first_turn);
        assert_eq!(entity.bronze_automaton.num_turns, 3);
        assert!(entity.bronze_automaton.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_bronze_orb_runtime_state() {
        let snapshot = bronze_orb_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::BronzeOrb as usize);
        assert!(entity.bronze_orb.used_stasis);
        assert!(entity.bronze_orb.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_champ_runtime_state() {
        let snapshot = champ_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Champ as usize);
        assert!(!entity.champ.first_turn);
        assert_eq!(entity.champ.num_turns, 3);
        assert_eq!(entity.champ.forge_times, 1);
        assert!(entity.champ.threshold_reached);
        assert!(entity.champ.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_writhing_mass_runtime_state() {
        let snapshot = writhing_mass_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::WrithingMass as usize);
        assert!(!entity.writhing_mass.first_move);
        assert!(entity.writhing_mass.used_mega_debuff);
        assert!(entity.writhing_mass.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_snake_dagger_runtime_state() {
        let snapshot = snake_dagger_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SnakeDagger as usize);
        assert_eq!(entity.planned_move_id(), 2);
        assert!(!entity.snake_dagger.first_move);
        assert!(entity.snake_dagger.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_spiker_runtime_state() {
        let snapshot = spiker_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Spiker as usize);
        assert_eq!(entity.spiker.thorns_count, 5);
        assert!(entity.spiker.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_spire_shield_runtime_state() {
        let snapshot = spire_shield_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SpireShield as usize);
        assert_eq!(entity.spire_shield.move_count, 5);
        assert!(entity.spire_shield.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_spire_spear_runtime_state() {
        let snapshot = spire_spear_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SpireSpear as usize);
        assert_eq!(entity.spire_spear.move_count, 4);
        assert_eq!(entity.spire_spear.skewer_count, 4);
        assert!(entity.spire_spear.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_slaver_red_runtime_state() {
        let snapshot = slaver_red_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SlaverRed as usize);
        assert!(!entity.slaver_red.first_turn);
        assert!(entity.slaver_red.used_entangle);
        assert!(entity.slaver_red.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_gremlin_nob_runtime_state() {
        let snapshot = gremlin_nob_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::GremlinNob as usize);
        assert!(entity.gremlin_nob.used_bellow);
        assert!(entity.gremlin_nob.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_gremlin_wizard_runtime_state() {
        let snapshot = gremlin_wizard_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::GremlinWizard as usize);
        assert_eq!(entity.gremlin_wizard.current_charge, 2);
        assert!(entity.gremlin_wizard.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_cultist_runtime_state() {
        let snapshot = cultist_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Cultist as usize);
        assert!(!entity.cultist.first_move);
        assert!(entity.cultist.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_sentry_runtime_state() {
        let snapshot = sentry_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::Sentry as usize);
        assert!(!entity.sentry.first_move);
        assert!(entity.sentry.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_spheric_guardian_runtime_state() {
        let snapshot = spheric_guardian_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SphericGuardian as usize);
        assert!(!entity.spheric_guardian.first_move);
        assert!(entity.spheric_guardian.second_move);
        assert!(entity.spheric_guardian.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_jaw_worm_runtime_state() {
        let snapshot = jaw_worm_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::JawWorm as usize);
        assert!(!entity.jaw_worm.first_move);
        assert!(entity.jaw_worm.hard_mode);
        assert!(entity.jaw_worm.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_slime_boss_runtime_state() {
        let snapshot = slime_boss_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::SlimeBoss as usize);
        assert!(!entity.slime_boss.first_turn);
        assert!(entity.slime_boss.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_large_slime_split_triggered_runtime_state() {
        let snapshot = acid_slime_l_truth_snapshot();
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.monster_type, EnemyId::AcidSlimeL as usize);
        assert!(entity.large_slime.split_triggered);
        assert!(entity.large_slime.protocol_seeded);
    }

    #[test]
    fn truth_import_seeds_louse_runtime_damage_when_exported() {
        let snapshot = json!({
            "id": "Louse",
            "current_hp": 12,
            "max_hp": 12,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 5,
            "move_adjusted_damage": 9,
            "move_hits": 1,
            "second_last_move_id": 4,
            "last_move_id": 3,
            "powers": [],
            "runtime_state": {
                "bite_damage": 7
            },
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });
        let mut entity = blank_monster_entity();

        apply_monster_truth_snapshot(&snapshot, 0, &mut entity);

        assert_eq!(entity.planned_move_id(), 3);
        assert_eq!(entity.louse_bite_damage(), Some(7));
    }

    #[test]
    fn truth_import_requires_runtime_bite_damage_for_louse() {
        let snapshot = json!({
            "id": "Louse",
            "current_hp": 12,
            "max_hp": 12,
            "block": 0,
            "intent": "ATTACK",
            "move_id": 3,
            "move_base_damage": 7,
            "move_adjusted_damage": 8,
            "move_hits": 1,
            "second_last_move_id": 4,
            "last_move_id": 3,
            "powers": [],
            "is_gone": false,
            "is_dying": false,
            "is_escaping": false,
            "is_escaped": false,
            "half_dead": false
        });
        let mut entity = blank_monster_entity();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_monster_truth_snapshot(&snapshot, 0, &mut entity);
        }));

        assert!(result.is_err());
    }
}
