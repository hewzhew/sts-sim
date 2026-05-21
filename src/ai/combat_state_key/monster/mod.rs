mod fallback;
mod move_state;
mod runtime;

use crate::content::monsters::EnemyId;
use crate::runtime::combat::MonsterEntity;

use fallback::stable_all_monster_runtime_signature;
use move_state::stable_move_state_signature;
use runtime::*;
pub(super) fn stable_monster_signature(monster: &MonsterEntity) -> String {
    format!(
        "{}:hp{}:{}:blk{}:dy{}:esc{}:half{}:move={}:rt={}",
        stable_monster_kind_signature(monster.monster_type),
        monster.current_hp,
        monster.max_hp,
        monster.block,
        monster.is_dying,
        monster.is_escaped,
        monster.half_dead,
        stable_move_state_signature(&monster.move_state),
        stable_monster_runtime_signature(monster),
    )
}

fn stable_monster_kind_signature(monster_type: usize) -> String {
    EnemyId::from_id(monster_type)
        .map(|enemy| format!("{enemy:?}"))
        .unwrap_or_else(|| format!("id{monster_type}"))
}

fn stable_monster_runtime_signature(monster: &MonsterEntity) -> String {
    match EnemyId::from_id(monster.monster_type) {
        Some(EnemyId::Hexaghost) => {
            format!("hex:{}", stable_hexaghost_signature(&monster.hexaghost))
        }
        Some(EnemyId::LouseNormal) | Some(EnemyId::LouseDefensive) => {
            format!("louse:{}", stable_louse_signature(&monster.louse))
        }
        Some(EnemyId::JawWorm) => format!("jaw:{}", stable_jaw_worm_signature(&monster.jaw_worm)),
        Some(EnemyId::Looter) | Some(EnemyId::Mugger) => {
            format!("thief:{}", stable_thief_signature(&monster.thief))
        }
        Some(EnemyId::Byrd) => format!("byrd:{}", stable_byrd_signature(&monster.byrd)),
        Some(EnemyId::Chosen) => format!("chosen:{}", stable_chosen_signature(&monster.chosen)),
        Some(EnemyId::Snecko) => format!("snecko:{}", stable_snecko_signature(&monster.snecko)),
        Some(EnemyId::ShelledParasite) => format!(
            "parasite:{}",
            stable_shelled_parasite_signature(&monster.shelled_parasite)
        ),
        Some(EnemyId::BronzeAutomaton) => format!(
            "bronze_auto:{}",
            stable_bronze_automaton_signature(&monster.bronze_automaton)
        ),
        Some(EnemyId::BronzeOrb) => {
            format!(
                "bronze_orb:{}",
                stable_bronze_orb_signature(&monster.bronze_orb)
            )
        }
        Some(EnemyId::BookOfStabbing) => {
            format!(
                "book:{}",
                stable_book_of_stabbing_signature(&monster.book_of_stabbing)
            )
        }
        Some(EnemyId::TheCollector) => {
            format!(
                "collector:{}",
                stable_collector_signature(&monster.collector)
            )
        }
        Some(EnemyId::Champ) => format!("champ:{}", stable_champ_signature(&monster.champ)),
        Some(EnemyId::AwakenedOne) => {
            format!(
                "awakened:{}",
                stable_awakened_one_signature(&monster.awakened_one)
            )
        }
        Some(EnemyId::CorruptHeart) => {
            format!(
                "heart:{}",
                stable_corrupt_heart_signature(&monster.corrupt_heart)
            )
        }
        Some(EnemyId::Darkling) => {
            format!("darkling:{}", stable_darkling_signature(&monster.darkling))
        }
        Some(EnemyId::Reptomancer) => {
            format!(
                "reptomancer:{}",
                stable_reptomancer_signature(&monster.reptomancer)
            )
        }
        Some(EnemyId::Nemesis) => {
            format!("nemesis:{}", stable_nemesis_signature(&monster.nemesis))
        }
        Some(EnemyId::GiantHead) => {
            format!(
                "giant_head:{}",
                stable_giant_head_signature(&monster.giant_head)
            )
        }
        Some(EnemyId::TimeEater) => {
            format!(
                "time_eater:{}",
                stable_time_eater_signature(&monster.time_eater)
            )
        }
        Some(EnemyId::Donu) => format!("donu:{}", stable_donu_signature(&monster.donu)),
        Some(EnemyId::Deca) => format!("deca:{}", stable_deca_signature(&monster.deca)),
        Some(EnemyId::Transient) => {
            format!(
                "transient:{}",
                stable_transient_signature(&monster.transient)
            )
        }
        Some(EnemyId::Exploder) => {
            format!("exploder:{}", stable_exploder_signature(&monster.exploder))
        }
        Some(EnemyId::Maw) => format!("maw:{}", stable_maw_signature(&monster.maw)),
        Some(EnemyId::SnakeDagger) => {
            format!(
                "snake_dagger:{}",
                stable_snake_dagger_signature(&monster.snake_dagger)
            )
        }
        Some(EnemyId::WrithingMass) => {
            format!(
                "writhing_mass:{}",
                stable_writhing_mass_signature(&monster.writhing_mass)
            )
        }
        Some(EnemyId::Spiker) => {
            format!("spiker:{}", stable_spiker_signature(&monster.spiker))
        }
        Some(EnemyId::SpireShield) => {
            format!(
                "spire_shield:{}",
                stable_spire_shield_signature(&monster.spire_shield)
            )
        }
        Some(EnemyId::SpireSpear) => {
            format!(
                "spire_spear:{}",
                stable_spire_spear_signature(&monster.spire_spear)
            )
        }
        Some(EnemyId::SlaverRed) => {
            format!(
                "slaver_red:{}",
                stable_slaver_red_signature(&monster.slaver_red)
            )
        }
        Some(EnemyId::GremlinLeader) => {
            format!(
                "gremlin_leader:{}",
                stable_gremlin_leader_signature(&monster.gremlin_leader)
            )
        }
        Some(EnemyId::GremlinNob) => {
            format!(
                "gremlin_nob:{}",
                stable_gremlin_nob_signature(&monster.gremlin_nob)
            )
        }
        Some(EnemyId::GremlinWizard) => {
            format!(
                "gremlin_wizard:{}",
                stable_gremlin_wizard_signature(&monster.gremlin_wizard)
            )
        }
        Some(EnemyId::Cultist) => {
            format!("cultist:{}", stable_cultist_signature(&monster.cultist))
        }
        Some(EnemyId::Sentry) => {
            format!("sentry:{}", stable_sentry_signature(&monster.sentry))
        }
        Some(EnemyId::SlimeBoss) => {
            format!(
                "slime_boss:{}",
                stable_slime_boss_signature(&monster.slime_boss)
            )
        }
        Some(EnemyId::AcidSlimeL | EnemyId::SpikeSlimeL) => {
            format!(
                "large_slime:{}",
                stable_large_slime_signature(&monster.large_slime)
            )
        }
        Some(EnemyId::SphericGuardian) => {
            format!(
                "spheric_guardian:{}",
                stable_spheric_guardian_signature(&monster.spheric_guardian)
            )
        }
        Some(EnemyId::Lagavulin) => {
            format!(
                "lagavulin:{}",
                stable_lagavulin_signature(&monster.lagavulin)
            )
        }
        Some(EnemyId::TheGuardian) => {
            format!("guardian:{}", stable_guardian_signature(&monster.guardian))
        }
        Some(_) => "_".to_string(),
        None => stable_all_monster_runtime_signature(monster),
    }
}
