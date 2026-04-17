use crate::runtime::combat::{Intent, MonsterEntity};
use crate::runtime::rng::StsRng;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterId {
    // === Act 1: Exordium ===
    BlueSlaver,
    Cultist,
    JawWorm,
    Looter,
    GremlinGang,
    RedSlaver,
    LargeSlime,
    ExordiumThugs,
    ExordiumWildlife,
    ThreeLouse,
    TwoLouse,
    TwoFungiBeasts,
    LotsOfSlimes,
    SmallSlimes,
    GremlinNob,
    Lagavulin,
    ThreeSentries,
    LagavulinEvent,
    TheMushroomLair,
    TheGuardian,
    Hexaghost,
    SlimeBoss,

    // === Act 2: The City ===
    // Weak
    TwoThieves,      // "2 Thieves" — Looter + Mugger
    ThreeByrds,      // "3 Byrds"
    ChosenAlone,     // "Chosen"
    ShellParasite,   // "Shell Parasite"
    SphericGuardian, // "Spheric Guardian"
    // Strong
    ChosenAndByrds,          // "Chosen and Byrds" — Byrd + Chosen
    SentryAndSphere,         // "Sentry and Sphere" — Sentry + SphericGuardian
    SnakePlant,              // "Snake Plant"
    Snecko,                  // "Snecko"
    CenturionAndHealer,      // "Centurion and Healer"
    CultistAndChosen,        // "Cultist and Chosen"
    ThreeCultists,           // "3 Cultists"
    ShelledParasiteAndFungi, // "Shelled Parasite and Fungi"
    // Elite
    GremlinLeader,  // "Gremlin Leader" — 2 gremlins + leader
    Slavers,        // "Slavers" — Blue + Taskmaster + Red
    BookOfStabbing, // "Book of Stabbing"
    // Boss
    Automaton, // "Automaton" — BronzeAutomaton
    TheChamp,  // "Champ"
    Collector, // "Collector" — TheCollector
    // Event
    MaskedBandits,    // "Masked Bandits"
    ColosseumSlavers, // "Colosseum Slavers"
    ColosseumNobs,    // "Colosseum Nobs"

    // === Act 3: The Beyond ===
    // Weak
    ThreeDarklings, // "3 Darklings"
    OrbWalker,      // "Orb Walker"
    ThreeShapes,    // "3 Shapes" — random draw from Spiker/Repulsor/Exploder
    // Strong
    SpireGrowth,        // "Spire Growth"
    Transient,          // "Transient"
    FourShapes,         // "4 Shapes"
    TheMaw,             // "Maw"
    SphereAndTwoShapes, // "Sphere and 2 Shapes"
    JawWormHorde,       // "Jaw Worm Horde" — 3 JawWorms (harder)
    WrithingMass,       // "Writhing Mass"
    // Elite
    GiantHead,   // "Giant Head"
    TheNemesis,  // "Nemesis"
    Reptomancer, // "Reptomancer" — Reptomancer + SnakeDaggers
    // Boss
    AwakenedOne, // "Awakened One" — 2 Cultists + AwakenedOne
    TimeEater,   // "Time Eater"
    DonuAndDeca, // "Donu and Deca"
    // Event
    MysteriousSphere, // "Mysterious Sphere" — 2 Shapes + OrbWalker
    TwoOrbWalkers,    // "2 Orb Walkers"
    SneckoAndMystics, // "Snecko and Mystics" — Healer + Snecko + Healer

    // === Act 4 ===
    ShieldAndSpear, // "Shield and Spear"
    TheHeart,       // "The Heart" — CorruptHeart
}

use crate::content::monsters::EnemyId;

/// Builds a complete encounter with seeded HP and precise composition parity with Java.
pub fn build_encounter(
    encounter: EncounterId,
    misc_rng: &mut StsRng,
    monster_hp_rng: &mut StsRng,
    ascension_level: u8,
) -> Vec<MonsterEntity> {
    let mut monsters = Vec::new();
    let mut slot_counter = 0;

    let spawn_monster = |enemy_id: EnemyId, hp_rng: &mut StsRng, slot: u8| -> MonsterEntity {
        let (min, max) = crate::content::monsters::get_hp_range(enemy_id, ascension_level);
        let current_hp = hp_rng.random_range(min as i32, max as i32) as i32;
        let id = (slot + 1) as usize; // Naive unique engine IDs for this batch
        let intent_preview_damage = match enemy_id {
            EnemyId::LouseNormal | EnemyId::LouseDefensive => {
                if ascension_level >= 2 {
                    hp_rng.random_range(6, 8) as i32
                } else {
                    hp_rng.random_range(5, 7) as i32
                }
            }
            _ => 0,
        };

        let mut monster = MonsterEntity {
            id,
            monster_type: enemy_id as usize,
            current_hp,
            max_hp: current_hp,
            block: 0,
            slot,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_preview_damage,
            logical_position: slot as i32,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        };

        if enemy_id == EnemyId::Chosen {
            monster.chosen.first_turn = true;
        }

        if enemy_id == EnemyId::Darkling {
            crate::content::monsters::beyond::darkling::initialize_runtime_state(
                &mut monster,
                hp_rng,
                ascension_level,
            );
        }

        monster
    };

    match encounter {
        EncounterId::BlueSlaver => {
            monsters.push(spawn_monster(
                EnemyId::SlaverBlue,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Cultist => {
            monsters.push(spawn_monster(
                EnemyId::Cultist,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::JawWorm => {
            monsters.push(spawn_monster(
                EnemyId::JawWorm,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Looter => {
            monsters.push(spawn_monster(EnemyId::Looter, monster_hp_rng, slot_counter));
        }
        EncounterId::GremlinGang => {
            // Java: spawnGremlins() — draw-without-replace from pool of 8, pick 4
            let mut pool = vec![
                EnemyId::GremlinWarrior,
                EnemyId::GremlinWarrior,
                EnemyId::GremlinThief,
                EnemyId::GremlinThief,
                EnemyId::GremlinFat,
                EnemyId::GremlinFat,
                EnemyId::GremlinTsundere,
                EnemyId::GremlinWizard,
            ];
            for _ in 0..4 {
                let index = misc_rng.random_range(0, (pool.len() - 1) as i32) as usize;
                let picked = pool.remove(index);
                monsters.push(spawn_monster(picked, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::RedSlaver => {
            monsters.push(spawn_monster(
                EnemyId::SlaverRed,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::LargeSlime => {
            let is_acid = misc_rng.random_boolean();
            let slime_id = if is_acid {
                EnemyId::AcidSlimeL
            } else {
                EnemyId::SpikeSlimeL
            };
            monsters.push(spawn_monster(slime_id, monster_hp_rng, slot_counter));
        }
        EncounterId::ExordiumThugs => {
            // Java: bottomHumanoid() — weakWildlife + strongHumanoid

            // bottomGetWeakWildlife: getLouse(), SpikeSlimeM, AcidSlimeM
            let get_louse = |rng: &mut StsRng| -> EnemyId {
                if rng.random_boolean() {
                    EnemyId::LouseNormal
                } else {
                    EnemyId::LouseDefensive
                }
            };
            let weak_pool = [
                get_louse(misc_rng),
                EnemyId::SpikeSlimeM,
                EnemyId::AcidSlimeM,
            ];
            let weak_idx = misc_rng.random_range(0, 2) as usize;
            monsters.push(spawn_monster(
                weak_pool[weak_idx],
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;

            // bottomGetStrongHumanoid: Cultist, getSlaver(), Looter
            // Java: getSlaver() = miscRng.randomBoolean() ? SlaverRed : SlaverBlue
            let slaver = if misc_rng.random_boolean() {
                EnemyId::SlaverRed
            } else {
                EnemyId::SlaverBlue
            };
            let strong_pool = [EnemyId::Cultist, slaver, EnemyId::Looter];
            let strong_idx = misc_rng.random_range(0, 2) as usize;
            monsters.push(spawn_monster(
                strong_pool[strong_idx],
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ExordiumWildlife => {
            // Java: bottomWildlife() — numMonster=2: strongWildlife + weakWildlife

            // bottomGetStrongWildlife: FungiBeast, JawWorm
            let strong_pool = [EnemyId::FungiBeast, EnemyId::JawWorm];
            let strong_idx = misc_rng.random_range(0, 1) as usize;
            monsters.push(spawn_monster(
                strong_pool[strong_idx],
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;

            // bottomGetWeakWildlife: getLouse(), SpikeSlimeM, AcidSlimeM
            let louse = if misc_rng.random_boolean() {
                EnemyId::LouseNormal
            } else {
                EnemyId::LouseDefensive
            };
            let weak_pool = [louse, EnemyId::SpikeSlimeM, EnemyId::AcidSlimeM];
            let weak_idx = misc_rng.random_range(0, 2) as usize;
            monsters.push(spawn_monster(
                weak_pool[weak_idx],
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ThreeLouse => {
            // Java: getLouse() × 3 — each independently random Normal/Defensive
            for _ in 0..3 {
                let louse = if misc_rng.random_boolean() {
                    EnemyId::LouseNormal
                } else {
                    EnemyId::LouseDefensive
                };
                monsters.push(spawn_monster(louse, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::TwoLouse => {
            // Java: getLouse() × 2
            for _ in 0..2 {
                let louse = if misc_rng.random_boolean() {
                    EnemyId::LouseNormal
                } else {
                    EnemyId::LouseDefensive
                };
                monsters.push(spawn_monster(louse, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::TwoFungiBeasts => {
            for _ in 0..2 {
                monsters.push(spawn_monster(
                    EnemyId::FungiBeast,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::LotsOfSlimes => {
            // Java: spawnManySmallSlimes() — draw-without-replace from pool of 5
            let mut pool = vec![
                EnemyId::SpikeSlimeS,
                EnemyId::SpikeSlimeS,
                EnemyId::SpikeSlimeS,
                EnemyId::AcidSlimeS,
                EnemyId::AcidSlimeS,
            ];
            for _ in 0..5 {
                let index = misc_rng.random_range(0, (pool.len() - 1) as i32) as usize;
                let picked = pool.remove(index);
                monsters.push(spawn_monster(picked, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::SmallSlimes => {
            let is_spike = misc_rng.random_boolean();
            if is_spike {
                monsters.push(spawn_monster(
                    EnemyId::SpikeSlimeS,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
                monsters.push(spawn_monster(
                    EnemyId::AcidSlimeM,
                    monster_hp_rng,
                    slot_counter,
                ));
            } else {
                monsters.push(spawn_monster(
                    EnemyId::AcidSlimeS,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
                monsters.push(spawn_monster(
                    EnemyId::SpikeSlimeM,
                    monster_hp_rng,
                    slot_counter,
                ));
            }
        }
        EncounterId::GremlinNob => {
            monsters.push(spawn_monster(
                EnemyId::GremlinNob,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Lagavulin | EncounterId::LagavulinEvent => {
            monsters.push(spawn_monster(
                EnemyId::Lagavulin,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ThreeSentries => {
            for _ in 0..3 {
                monsters.push(spawn_monster(EnemyId::Sentry, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::TheMushroomLair => {
            for _ in 0..3 {
                monsters.push(spawn_monster(
                    EnemyId::FungiBeast,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::TheGuardian => {
            monsters.push(spawn_monster(
                EnemyId::TheGuardian,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Hexaghost => {
            monsters.push(spawn_monster(
                EnemyId::Hexaghost,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::SlimeBoss => {
            monsters.push(spawn_monster(
                EnemyId::SlimeBoss,
                monster_hp_rng,
                slot_counter,
            ));
        }

        // ==========================================
        // Act 2: The City
        // ==========================================
        EncounterId::TwoThieves => {
            // Java: Looter + Mugger
            monsters.push(spawn_monster(EnemyId::Looter, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Mugger, monster_hp_rng, slot_counter));
        }
        EncounterId::ThreeByrds => {
            for _ in 0..3 {
                monsters.push(spawn_monster(EnemyId::Byrd, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::ChosenAlone => {
            monsters.push(spawn_monster(EnemyId::Chosen, monster_hp_rng, slot_counter));
        }
        EncounterId::ShellParasite => {
            monsters.push(spawn_monster(
                EnemyId::ShelledParasite,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::SphericGuardian => {
            monsters.push(spawn_monster(
                EnemyId::SphericGuardian,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ChosenAndByrds => {
            // Java: Byrd + Chosen
            monsters.push(spawn_monster(EnemyId::Byrd, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Chosen, monster_hp_rng, slot_counter));
        }
        EncounterId::SentryAndSphere => {
            // Java: Sentry + SphericGuardian
            monsters.push(spawn_monster(EnemyId::Sentry, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::SphericGuardian,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::SnakePlant => {
            monsters.push(spawn_monster(
                EnemyId::SnakePlant,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Snecko => {
            monsters.push(spawn_monster(EnemyId::Snecko, monster_hp_rng, slot_counter));
        }
        EncounterId::CenturionAndHealer => {
            monsters.push(spawn_monster(
                EnemyId::Centurion,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Healer, monster_hp_rng, slot_counter));
        }
        EncounterId::CultistAndChosen => {
            // Java: Cultist(-230, 15, false) + Chosen(100, 25)
            monsters.push(spawn_monster(
                EnemyId::Cultist,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Chosen, monster_hp_rng, slot_counter));
        }
        EncounterId::ThreeCultists => {
            for _ in 0..3 {
                monsters.push(spawn_monster(
                    EnemyId::Cultist,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::ShelledParasiteAndFungi => {
            monsters.push(spawn_monster(
                EnemyId::ShelledParasite,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::FungiBeast,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::GremlinLeader => {
            // Java: spawnGremlin() + spawnGremlin() + GremlinLeader
            // spawnGremlin = weighted random from the 8-entry pool used by MonsterHelper.
            let gremlin_pool = [
                EnemyId::GremlinWarrior,
                EnemyId::GremlinWarrior,
                EnemyId::GremlinThief,
                EnemyId::GremlinThief,
                EnemyId::GremlinFat,
                EnemyId::GremlinFat,
                EnemyId::GremlinTsundere,
                EnemyId::GremlinWizard,
            ];
            let g1 = gremlin_pool[misc_rng.random_range(0, 7) as usize];
            let mut first = spawn_monster(g1, monster_hp_rng, slot_counter);
            first.logical_position =
                crate::content::monsters::city::gremlin_leader::GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[0];
            monsters.push(first);
            slot_counter += 1;
            let g2 = gremlin_pool[misc_rng.random_range(0, 7) as usize];
            let mut second = spawn_monster(g2, monster_hp_rng, slot_counter);
            second.logical_position =
                crate::content::monsters::city::gremlin_leader::GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[1];
            monsters.push(second);
            slot_counter += 1;
            let mut leader = spawn_monster(EnemyId::GremlinLeader, monster_hp_rng, slot_counter);
            leader.logical_position =
                crate::content::monsters::city::gremlin_leader::GremlinLeader::LEADER_LOGICAL_POSITION;
            monsters.push(leader);
        }
        EncounterId::Slavers => {
            // Java: SlaverBlue + Taskmaster + SlaverRed
            monsters.push(spawn_monster(
                EnemyId::SlaverBlue,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::Taskmaster,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::SlaverRed,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::BookOfStabbing => {
            monsters.push(spawn_monster(
                EnemyId::BookOfStabbing,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Automaton => {
            monsters.push(spawn_monster(
                EnemyId::BronzeAutomaton,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::TheChamp => {
            monsters.push(spawn_monster(EnemyId::Champ, monster_hp_rng, slot_counter));
        }
        EncounterId::Collector => {
            monsters.push(spawn_monster(
                EnemyId::TheCollector,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::MaskedBandits => {
            // Java: BanditPointy + BanditLeader + BanditBear
            monsters.push(spawn_monster(
                EnemyId::BanditPointy,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::BanditLeader,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::BanditBear,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ColosseumSlavers => {
            // Java: SlaverBlue + SlaverRed
            monsters.push(spawn_monster(
                EnemyId::SlaverBlue,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::SlaverRed,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ColosseumNobs => {
            // Java: Taskmaster + GremlinNob
            monsters.push(spawn_monster(
                EnemyId::Taskmaster,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::GremlinNob,
                monster_hp_rng,
                slot_counter,
            ));
        }

        // ==========================================
        // Act 3: The Beyond
        // ==========================================
        EncounterId::ThreeDarklings => {
            for _ in 0..3 {
                monsters.push(spawn_monster(
                    EnemyId::Darkling,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::OrbWalker => {
            monsters.push(spawn_monster(
                EnemyId::OrbWalker,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::ThreeShapes => {
            // Java: spawnShapes(true) — 3 random shapes, draw-without-replace
            let mut shape_pool = vec![
                EnemyId::Spiker,
                EnemyId::Spiker,
                EnemyId::Repulsor,
                EnemyId::Repulsor,
                EnemyId::Exploder,
                EnemyId::Exploder,
            ];
            for _ in 0..3 {
                let idx = misc_rng.random_range(0, (shape_pool.len() - 1) as i32) as usize;
                let picked = shape_pool.remove(idx);
                monsters.push(spawn_monster(picked, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::SpireGrowth => {
            monsters.push(spawn_monster(
                EnemyId::SpireGrowth,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Transient => {
            monsters.push(spawn_monster(
                EnemyId::Transient,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::FourShapes => {
            // Java: spawnShapes(false) — 4 random shapes, draw-without-replace
            let mut shape_pool = vec![
                EnemyId::Spiker,
                EnemyId::Spiker,
                EnemyId::Repulsor,
                EnemyId::Repulsor,
                EnemyId::Exploder,
                EnemyId::Exploder,
            ];
            for _ in 0..4 {
                let idx = misc_rng.random_range(0, (shape_pool.len() - 1) as i32) as usize;
                let picked = shape_pool.remove(idx);
                monsters.push(spawn_monster(picked, monster_hp_rng, slot_counter));
                slot_counter += 1;
            }
        }
        EncounterId::TheMaw => {
            monsters.push(spawn_monster(EnemyId::Maw, monster_hp_rng, slot_counter));
        }
        EncounterId::SphereAndTwoShapes => {
            // Java: getAncientShape() × 2 + SphericGuardian
            let s1 = get_ancient_shape(misc_rng);
            monsters.push(spawn_monster(s1, monster_hp_rng, slot_counter));
            slot_counter += 1;
            let s2 = get_ancient_shape(misc_rng);
            monsters.push(spawn_monster(s2, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::SphericGuardian,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::JawWormHorde => {
            // Java: 3 JawWorms with isHorde=true
            for _ in 0..3 {
                monsters.push(spawn_monster(
                    EnemyId::JawWorm,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::WrithingMass => {
            monsters.push(spawn_monster(
                EnemyId::WrithingMass,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::GiantHead => {
            monsters.push(spawn_monster(
                EnemyId::GiantHead,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::TheNemesis => {
            monsters.push(spawn_monster(
                EnemyId::Nemesis,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::Reptomancer => {
            // Java: SnakeDagger + Reptomancer + SnakeDagger
            let mut left_dagger = spawn_monster(EnemyId::SnakeDagger, monster_hp_rng, slot_counter);
            left_dagger.logical_position =
                crate::content::monsters::beyond::reptomancer::Reptomancer::DAGGER_DRAW_X[1];
            left_dagger.protocol_identity.draw_x =
                Some(crate::content::monsters::beyond::reptomancer::Reptomancer::DAGGER_DRAW_X[1]);
            monsters.push(left_dagger);
            slot_counter += 1;
            let mut reptomancer = spawn_monster(EnemyId::Reptomancer, monster_hp_rng, slot_counter);
            reptomancer.logical_position = 0;
            reptomancer.protocol_identity.draw_x = Some(0);
            monsters.push(reptomancer);
            slot_counter += 1;
            let mut right_dagger =
                spawn_monster(EnemyId::SnakeDagger, monster_hp_rng, slot_counter);
            right_dagger.logical_position =
                crate::content::monsters::beyond::reptomancer::Reptomancer::DAGGER_DRAW_X[0];
            right_dagger.protocol_identity.draw_x =
                Some(crate::content::monsters::beyond::reptomancer::Reptomancer::DAGGER_DRAW_X[0]);
            monsters.push(right_dagger);
        }
        EncounterId::AwakenedOne => {
            // Java: 2 Cultists + AwakenedOne
            monsters.push(spawn_monster(
                EnemyId::Cultist,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::Cultist,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::AwakenedOne,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::TimeEater => {
            monsters.push(spawn_monster(
                EnemyId::TimeEater,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::DonuAndDeca => {
            monsters.push(spawn_monster(EnemyId::Deca, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Donu, monster_hp_rng, slot_counter));
        }
        EncounterId::MysteriousSphere => {
            // Java: getAncientShape() × 2 + OrbWalker
            let s1 = get_ancient_shape(misc_rng);
            monsters.push(spawn_monster(s1, monster_hp_rng, slot_counter));
            slot_counter += 1;
            let s2 = get_ancient_shape(misc_rng);
            monsters.push(spawn_monster(s2, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::OrbWalker,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::TwoOrbWalkers => {
            for _ in 0..2 {
                monsters.push(spawn_monster(
                    EnemyId::OrbWalker,
                    monster_hp_rng,
                    slot_counter,
                ));
                slot_counter += 1;
            }
        }
        EncounterId::SneckoAndMystics => {
            // Java: Healer + Snecko + Healer
            monsters.push(spawn_monster(EnemyId::Healer, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Snecko, monster_hp_rng, slot_counter));
            slot_counter += 1;
            monsters.push(spawn_monster(EnemyId::Healer, monster_hp_rng, slot_counter));
        }

        // ==========================================
        // Act 4
        // ==========================================
        EncounterId::ShieldAndSpear => {
            monsters.push(spawn_monster(
                EnemyId::SpireShield,
                monster_hp_rng,
                slot_counter,
            ));
            slot_counter += 1;
            monsters.push(spawn_monster(
                EnemyId::SpireSpear,
                monster_hp_rng,
                slot_counter,
            ));
        }
        EncounterId::TheHeart => {
            monsters.push(spawn_monster(
                EnemyId::CorruptHeart,
                monster_hp_rng,
                slot_counter,
            ));
        }
    }

    monsters
}

/// Java: getAncientShape() — random(0..2) → Spiker, Repulsor, or Exploder
fn get_ancient_shape(misc_rng: &mut StsRng) -> EnemyId {
    match misc_rng.random_range(0, 2) {
        0 => EnemyId::Spiker,
        1 => EnemyId::Repulsor,
        _ => EnemyId::Exploder,
    }
}
