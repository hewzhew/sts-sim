use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatState, MonsterEntity};

use super::super::types::{
    CombatMonsterKey, CombatMonsterRuntimeFallbackKey, CombatMonsterRuntimeKey,
};

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
    match EnemyId::from_id(monster.monster_type) {
        Some(EnemyId::Hexaghost) => CombatMonsterRuntimeKey::Hexaghost(monster.hexaghost.clone()),
        Some(EnemyId::LouseNormal | EnemyId::LouseDefensive) => {
            CombatMonsterRuntimeKey::Louse(monster.louse.clone())
        }
        Some(EnemyId::JawWorm) => CombatMonsterRuntimeKey::JawWorm(monster.jaw_worm.clone()),
        Some(EnemyId::Looter | EnemyId::Mugger) => {
            CombatMonsterRuntimeKey::Thief(monster.thief.clone())
        }
        Some(EnemyId::Byrd) => CombatMonsterRuntimeKey::Byrd(monster.byrd.clone()),
        Some(EnemyId::Chosen) => CombatMonsterRuntimeKey::Chosen(monster.chosen.clone()),
        Some(EnemyId::Snecko) => CombatMonsterRuntimeKey::Snecko(monster.snecko.clone()),
        Some(EnemyId::ShelledParasite) => {
            CombatMonsterRuntimeKey::ShelledParasite(monster.shelled_parasite.clone())
        }
        Some(EnemyId::BronzeAutomaton) => {
            CombatMonsterRuntimeKey::BronzeAutomaton(monster.bronze_automaton.clone())
        }
        Some(EnemyId::BronzeOrb) => CombatMonsterRuntimeKey::BronzeOrb(monster.bronze_orb.clone()),
        Some(EnemyId::BookOfStabbing) => {
            CombatMonsterRuntimeKey::BookOfStabbing(monster.book_of_stabbing.clone())
        }
        Some(EnemyId::TheCollector) => {
            CombatMonsterRuntimeKey::Collector(monster.collector.clone())
        }
        Some(EnemyId::Champ) => CombatMonsterRuntimeKey::Champ(monster.champ.clone()),
        Some(EnemyId::AwakenedOne) => {
            CombatMonsterRuntimeKey::AwakenedOne(monster.awakened_one.clone())
        }
        Some(EnemyId::CorruptHeart) => {
            CombatMonsterRuntimeKey::CorruptHeart(monster.corrupt_heart.clone())
        }
        Some(EnemyId::WrithingMass) => {
            CombatMonsterRuntimeKey::WrithingMass(monster.writhing_mass.clone())
        }
        Some(EnemyId::Spiker) => CombatMonsterRuntimeKey::Spiker(monster.spiker.clone()),
        Some(EnemyId::SpireShield) => {
            CombatMonsterRuntimeKey::SpireShield(monster.spire_shield.clone())
        }
        Some(EnemyId::SpireSpear) => {
            CombatMonsterRuntimeKey::SpireSpear(monster.spire_spear.clone())
        }
        Some(EnemyId::SlaverRed) => CombatMonsterRuntimeKey::SlaverRed(monster.slaver_red.clone()),
        Some(EnemyId::GremlinLeader) => {
            CombatMonsterRuntimeKey::GremlinLeader(monster.gremlin_leader.clone())
        }
        Some(EnemyId::GremlinNob) => {
            CombatMonsterRuntimeKey::GremlinNob(monster.gremlin_nob.clone())
        }
        Some(EnemyId::GremlinWizard) => {
            CombatMonsterRuntimeKey::GremlinWizard(monster.gremlin_wizard.clone())
        }
        Some(EnemyId::Cultist) => CombatMonsterRuntimeKey::Cultist(monster.cultist.clone()),
        Some(EnemyId::Sentry) => CombatMonsterRuntimeKey::Sentry(monster.sentry.clone()),
        Some(EnemyId::SlimeBoss) => CombatMonsterRuntimeKey::SlimeBoss(monster.slime_boss.clone()),
        Some(EnemyId::AcidSlimeL | EnemyId::SpikeSlimeL) => {
            CombatMonsterRuntimeKey::LargeSlime(monster.large_slime.clone())
        }
        Some(EnemyId::SphericGuardian) => {
            CombatMonsterRuntimeKey::SphericGuardian(monster.spheric_guardian.clone())
        }
        Some(EnemyId::Reptomancer) => {
            CombatMonsterRuntimeKey::Reptomancer(monster.reptomancer.clone())
        }
        Some(EnemyId::Darkling) => CombatMonsterRuntimeKey::Darkling(monster.darkling.clone()),
        Some(EnemyId::Nemesis) => CombatMonsterRuntimeKey::Nemesis(monster.nemesis.clone()),
        Some(EnemyId::GiantHead) => CombatMonsterRuntimeKey::GiantHead(monster.giant_head.clone()),
        Some(EnemyId::TimeEater) => CombatMonsterRuntimeKey::TimeEater(monster.time_eater.clone()),
        Some(EnemyId::Donu) => CombatMonsterRuntimeKey::Donu(monster.donu.clone()),
        Some(EnemyId::Deca) => CombatMonsterRuntimeKey::Deca(monster.deca.clone()),
        Some(EnemyId::Transient) => CombatMonsterRuntimeKey::Transient(monster.transient.clone()),
        Some(EnemyId::Exploder) => CombatMonsterRuntimeKey::Exploder(monster.exploder.clone()),
        Some(EnemyId::Maw) => CombatMonsterRuntimeKey::Maw(monster.maw.clone()),
        Some(EnemyId::SnakeDagger) => {
            CombatMonsterRuntimeKey::SnakeDagger(monster.snake_dagger.clone())
        }
        Some(EnemyId::Lagavulin) => CombatMonsterRuntimeKey::Lagavulin(monster.lagavulin.clone()),
        Some(EnemyId::TheGuardian) => CombatMonsterRuntimeKey::Guardian(monster.guardian.clone()),
        Some(_) => CombatMonsterRuntimeKey::None,
        None => CombatMonsterRuntimeKey::Unknown(Box::new(all_monster_runtime_key(monster))),
    }
}

fn all_monster_runtime_key(monster: &MonsterEntity) -> CombatMonsterRuntimeFallbackKey {
    CombatMonsterRuntimeFallbackKey {
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
