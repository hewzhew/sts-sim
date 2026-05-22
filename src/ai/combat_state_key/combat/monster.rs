use crate::runtime::combat::{CombatState, MonsterEntity};

use super::super::types::{CombatMonsterKey, CombatMonsterRuntimeKey};

pub(super) fn monsters_key(combat: &CombatState) -> Vec<CombatMonsterKey> {
    combat.entities.monsters.iter().map(monster_key).collect()
}

fn monster_key(monster: &MonsterEntity) -> CombatMonsterKey {
    CombatMonsterKey {
        entity_id: monster.id,
        monster_type: monster.monster_type,
        current_hp: monster.current_hp,
        max_hp: monster.max_hp,
        block: monster.block,
        slot: monster.slot,
        logical_position: monster.logical_position,
        is_dying: monster.is_dying,
        is_escaped: monster.is_escaped,
        half_dead: monster.half_dead,
        move_state: monster.move_state.clone(),
        turn_plan: monster.turn_plan(),
        runtime: monster_runtime_key(monster),
    }
}

fn monster_runtime_key(monster: &MonsterEntity) -> CombatMonsterRuntimeKey {
    CombatMonsterRuntimeKey {
        hexaghost: monster.hexaghost.clone(),
        louse: monster.louse.clone(),
        jaw_worm: monster.jaw_worm.clone(),
        thief: monster.thief.clone(),
        byrd: monster.byrd.clone(),
        chosen: monster.chosen.clone(),
        snecko: monster.snecko.clone(),
        shelled_parasite: monster.shelled_parasite.clone(),
        bronze_automaton: monster.bronze_automaton.clone(),
        bronze_orb: monster.bronze_orb.clone(),
        book_of_stabbing: monster.book_of_stabbing.clone(),
        collector: monster.collector.clone(),
        champ: monster.champ.clone(),
        awakened_one: monster.awakened_one.clone(),
        corrupt_heart: monster.corrupt_heart.clone(),
        writhing_mass: monster.writhing_mass.clone(),
        spiker: monster.spiker.clone(),
        spire_shield: monster.spire_shield.clone(),
        spire_spear: monster.spire_spear.clone(),
        slaver_red: monster.slaver_red.clone(),
        gremlin_leader: monster.gremlin_leader.clone(),
        gremlin_nob: monster.gremlin_nob.clone(),
        gremlin_wizard: monster.gremlin_wizard.clone(),
        cultist: monster.cultist.clone(),
        sentry: monster.sentry.clone(),
        slime_boss: monster.slime_boss.clone(),
        large_slime: monster.large_slime.clone(),
        spheric_guardian: monster.spheric_guardian.clone(),
        reptomancer: monster.reptomancer.clone(),
        darkling: monster.darkling.clone(),
        nemesis: monster.nemesis.clone(),
        giant_head: monster.giant_head.clone(),
        time_eater: monster.time_eater.clone(),
        donu: monster.donu.clone(),
        deca: monster.deca.clone(),
        transient: monster.transient.clone(),
        exploder: monster.exploder.clone(),
        maw: monster.maw.clone(),
        snake_dagger: monster.snake_dagger.clone(),
        lagavulin: monster.lagavulin.clone(),
        guardian: monster.guardian.clone(),
    }
}
