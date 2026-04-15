//! Encounter scheduling system.
//!
//! Mirrors Java's encounter list generation:
//! - `MonsterInfo` with weighted selection (`roll()`)
//! - `populate_monster_list()` with no-repeat constraints
//! - `populate_first_strong_enemy()` with exclusion table
//! - Per-act encounter definitions (Exordium, TheCity, TheBeyond)

use super::factory::EncounterId;
use crate::runtime::rng::StsRng;

/// A weighted encounter entry, mirroring Java's `MonsterInfo`.
struct MonsterInfo {
    encounter: EncounterId,
    weight: f32,
}

/// Normalize weights so they sum to 1.0, sorted ascending by weight.
/// Java: `MonsterInfo.normalizeWeights()` — sorts by weight then divides by total.
fn normalize_weights(list: &mut Vec<MonsterInfo>) {
    list.sort_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap());
    let total: f32 = list.iter().map(|m| m.weight).sum();
    for m in list.iter_mut() {
        m.weight /= total;
    }
}

/// Weighted roll selection.
/// Java: `MonsterInfo.roll(list, roll)` — cumulative weight comparison.
fn roll(list: &[MonsterInfo], roll_value: f32) -> EncounterId {
    let mut current_weight = 0.0f32;
    for m in list {
        current_weight += m.weight;
        if roll_value < current_weight {
            return m.encounter;
        }
    }
    // Fallback (should not happen with properly normalized weights)
    list.last().unwrap().encounter
}

/// Populate monster list with no-repeat constraints.
/// Java: `AbstractDungeon.populateMonsterList()`
///
/// For normal fights: no repeat of last 1 OR last 2 entries.
/// For elite fights: no repeat of last 1 entry.
fn populate_monster_list(
    target: &mut Vec<EncounterId>,
    monsters: &[MonsterInfo],
    count: usize,
    elites: bool,
    monster_rng: &mut StsRng,
) {
    let mut i = 0;
    while i < count {
        let to_add = roll(monsters, monster_rng.random_f32());

        if target.is_empty() {
            target.push(to_add);
            i += 1;
            continue;
        }

        if elites {
            // Elite: must differ from last 1
            if to_add == *target.last().unwrap() {
                continue; // re-roll
            }
            target.push(to_add);
            i += 1;
        } else {
            // Normal: must differ from last 1 AND last 2
            if to_add == *target.last().unwrap() {
                continue; // re-roll
            }
            if target.len() > 1 && to_add == target[target.len() - 2] {
                continue; // re-roll
            }
            target.push(to_add);
            i += 1;
        }
    }
}

/// Populate the first strong enemy, re-rolling if result is in the exclusion set.
/// Java: `AbstractDungeon.populateFirstStrongEnemy()`
fn populate_first_strong_enemy(
    target: &mut Vec<EncounterId>,
    monsters: &[MonsterInfo],
    exclusions: &[EncounterId],
    monster_rng: &mut StsRng,
) {
    loop {
        let m = roll(monsters, monster_rng.random_f32());
        if !exclusions.contains(&m) {
            target.push(m);
            return;
        }
    }
}

/// Generate the full encounter schedule for a given act.
/// Returns (monster_list, elite_monster_list).
///
/// Called at dungeon initialization. The lists are consumed in order
/// as the player progresses through combat rooms.
pub fn generate_encounter_lists(
    act: u8,
    monster_rng: &mut StsRng,
) -> (Vec<EncounterId>, Vec<EncounterId>) {
    let mut monster_list: Vec<EncounterId> = Vec::new();
    let mut elite_list: Vec<EncounterId> = Vec::new();

    match act {
        1 => generate_exordium(&mut monster_list, &mut elite_list, monster_rng),
        2 => generate_the_city(&mut monster_list, &mut elite_list, monster_rng),
        3 => generate_the_beyond(&mut monster_list, &mut elite_list, monster_rng),
        _ => generate_exordium(&mut monster_list, &mut elite_list, monster_rng),
    }

    (monster_list, elite_list)
}

// ============================================================================
// Act 1: Exordium
// ============================================================================

fn generate_exordium(
    monster_list: &mut Vec<EncounterId>,
    elite_list: &mut Vec<EncounterId>,
    monster_rng: &mut StsRng,
) {
    // Weak enemies (3)
    // Java: Cultist(2.0), JawWorm(2.0), 2Louse(2.0), SmallSlimes(2.0)
    let mut weak = vec![
        MonsterInfo {
            encounter: EncounterId::Cultist,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::JawWorm,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::TwoLouse,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::SmallSlimes,
            weight: 2.0,
        },
    ];
    normalize_weights(&mut weak);
    populate_monster_list(monster_list, &weak, 3, false, monster_rng);

    // Strong enemies (12)
    // Java: BlueSlaver(2.0), GremlinGang(1.0), Looter(2.0), LargeSlime(2.0),
    //       LotsOfSlimes(1.0), ExordiumThugs(1.5), ExordiumWildlife(1.5),
    //       RedSlaver(1.0), 3Louse(2.0), 2FungiBeasts(2.0)
    let mut strong = vec![
        MonsterInfo {
            encounter: EncounterId::BlueSlaver,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::GremlinGang,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::Looter,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::LargeSlime,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::LotsOfSlimes,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::ExordiumThugs,
            weight: 1.5,
        },
        MonsterInfo {
            encounter: EncounterId::ExordiumWildlife,
            weight: 1.5,
        },
        MonsterInfo {
            encounter: EncounterId::RedSlaver,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeLouse,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::TwoFungiBeasts,
            weight: 2.0,
        },
    ];
    normalize_weights(&mut strong);

    // First strong enemy with exclusions based on last weak enemy
    let exclusions = exordium_exclusions(monster_list);
    populate_first_strong_enemy(monster_list, &strong, &exclusions, monster_rng);
    populate_monster_list(monster_list, &strong, 12, false, monster_rng);

    // Elites (10)
    // Java: GremlinNob(1.0), Lagavulin(1.0), 3Sentries(1.0)
    let mut elites = vec![
        MonsterInfo {
            encounter: EncounterId::GremlinNob,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::Lagavulin,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeSentries,
            weight: 1.0,
        },
    ];
    normalize_weights(&mut elites);
    populate_monster_list(elite_list, &elites, 10, true, monster_rng);
}

/// Java: Exordium.generateExclusions()
fn exordium_exclusions(monster_list: &[EncounterId]) -> Vec<EncounterId> {
    let last = monster_list.last().copied();
    match last {
        Some(EncounterId::Looter) => vec![EncounterId::ExordiumThugs],
        Some(EncounterId::BlueSlaver) => vec![EncounterId::RedSlaver, EncounterId::ExordiumThugs],
        Some(EncounterId::TwoLouse) => vec![EncounterId::ThreeLouse],
        Some(EncounterId::SmallSlimes) => vec![EncounterId::LargeSlime, EncounterId::LotsOfSlimes],
        _ => vec![],
    }
}

// ============================================================================
// Act 2: The City
// ============================================================================

fn generate_the_city(
    monster_list: &mut Vec<EncounterId>,
    elite_list: &mut Vec<EncounterId>,
    monster_rng: &mut StsRng,
) {
    // Weak enemies (2)
    // Java: SphericGuardian(2.0), Chosen(2.0), ShellParasite(2.0), 3Byrds(2.0), 2Thieves(2.0)
    let mut weak = vec![
        MonsterInfo {
            encounter: EncounterId::SphericGuardian,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::ChosenAlone,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::ShellParasite,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeByrds,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::TwoThieves,
            weight: 2.0,
        },
    ];
    normalize_weights(&mut weak);
    populate_monster_list(monster_list, &weak, 2, false, monster_rng);

    // Strong enemies (12)
    // Java: ChosenAndByrds(2.0), SentryAndSphere(2.0), SnakePlant(6.0), Snecko(4.0),
    //       CenturionAndHealer(6.0), CultistAndChosen(3.0), 3Cultists(3.0), ShelledParasiteAndFungi(3.0)
    let mut strong = vec![
        MonsterInfo {
            encounter: EncounterId::ChosenAndByrds,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::SentryAndSphere,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::SnakePlant,
            weight: 6.0,
        },
        MonsterInfo {
            encounter: EncounterId::Snecko,
            weight: 4.0,
        },
        MonsterInfo {
            encounter: EncounterId::CenturionAndHealer,
            weight: 6.0,
        },
        MonsterInfo {
            encounter: EncounterId::CultistAndChosen,
            weight: 3.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeCultists,
            weight: 3.0,
        },
        MonsterInfo {
            encounter: EncounterId::ShelledParasiteAndFungi,
            weight: 3.0,
        },
    ];
    normalize_weights(&mut strong);
    let exclusions = city_exclusions(monster_list);
    populate_first_strong_enemy(monster_list, &strong, &exclusions, monster_rng);
    populate_monster_list(monster_list, &strong, 12, false, monster_rng);

    // Elites (10)
    // Java: GremlinLeader(1.0), Slavers(1.0), BookOfStabbing(1.0)
    let mut elites = vec![
        MonsterInfo {
            encounter: EncounterId::GremlinLeader,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::Slavers,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::BookOfStabbing,
            weight: 1.0,
        },
    ];
    normalize_weights(&mut elites);
    populate_monster_list(elite_list, &elites, 10, true, monster_rng);
}

/// Java: TheCity.generateExclusions()
fn city_exclusions(monster_list: &[EncounterId]) -> Vec<EncounterId> {
    let last = monster_list.last().copied();
    match last {
        Some(EncounterId::SphericGuardian) => vec![EncounterId::SentryAndSphere],
        Some(EncounterId::ThreeByrds) => vec![EncounterId::ChosenAndByrds],
        Some(EncounterId::ChosenAlone) => {
            vec![EncounterId::ChosenAndByrds, EncounterId::CultistAndChosen]
        }
        _ => vec![],
    }
}

// ============================================================================
// Act 3: The Beyond
// ============================================================================

fn generate_the_beyond(
    monster_list: &mut Vec<EncounterId>,
    elite_list: &mut Vec<EncounterId>,
    monster_rng: &mut StsRng,
) {
    // Weak enemies (2)
    // Java: 3Darklings(2.0), OrbWalker(2.0), 3Shapes(2.0)
    let mut weak = vec![
        MonsterInfo {
            encounter: EncounterId::ThreeDarklings,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::OrbWalker,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeShapes,
            weight: 2.0,
        },
    ];
    normalize_weights(&mut weak);
    populate_monster_list(monster_list, &weak, 2, false, monster_rng);

    // Strong enemies (12)
    // Java: SpireGrowth(1.0), Transient(1.0), 4Shapes(1.0), Maw(1.0),
    //       SphereAndTwoShapes(1.0), JawWormHorde(1.0), 3Darklings(1.0), WrithingMass(1.0)
    let mut strong = vec![
        MonsterInfo {
            encounter: EncounterId::SpireGrowth,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::Transient,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::FourShapes,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::TheMaw,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::SphereAndTwoShapes,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::JawWormHorde,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::ThreeDarklings,
            weight: 1.0,
        },
        MonsterInfo {
            encounter: EncounterId::WrithingMass,
            weight: 1.0,
        },
    ];
    normalize_weights(&mut strong);
    let exclusions = beyond_exclusions(monster_list);
    populate_first_strong_enemy(monster_list, &strong, &exclusions, monster_rng);
    populate_monster_list(monster_list, &strong, 12, false, monster_rng);

    // Elites (10)
    // Java: GiantHead(2.0), Nemesis(2.0), Reptomancer(2.0)
    let mut elites = vec![
        MonsterInfo {
            encounter: EncounterId::GiantHead,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::TheNemesis,
            weight: 2.0,
        },
        MonsterInfo {
            encounter: EncounterId::Reptomancer,
            weight: 2.0,
        },
    ];
    normalize_weights(&mut elites);
    populate_monster_list(elite_list, &elites, 10, true, monster_rng);
}

/// Java: TheBeyond.generateExclusions()
fn beyond_exclusions(monster_list: &[EncounterId]) -> Vec<EncounterId> {
    let last = monster_list.last().copied();
    match last {
        Some(EncounterId::ThreeDarklings) => vec![EncounterId::ThreeDarklings],
        Some(EncounterId::OrbWalker) => vec![EncounterId::OrbWalker],
        Some(EncounterId::ThreeShapes) => vec![EncounterId::FourShapes],
        _ => vec![],
    }
}

/// Generate the boss list for the given act.
/// Java: initializeBoss() in Exordium/TheCity/TheBeyond.
/// Shuffles 3 bosses with `Collections.shuffle(bossList, new Random(monsterRng.randomLong()))`.
/// For the simulator we always take the "all bosses seen" path (shuffle all 3).
pub fn generate_boss_list(
    act: u8,
    monster_rng: &mut crate::runtime::rng::StsRng,
) -> Vec<EncounterId> {
    let mut bosses = match act {
        1 => vec![
            EncounterId::TheGuardian,
            EncounterId::Hexaghost,
            EncounterId::SlimeBoss,
        ],
        2 => vec![
            EncounterId::Automaton,
            EncounterId::Collector,
            EncounterId::TheChamp,
        ],
        3 => vec![
            EncounterId::AwakenedOne,
            EncounterId::TimeEater,
            EncounterId::DonuAndDeca,
        ],
        _ => vec![],
    };

    // Java: Collections.shuffle(bossList, new java.util.Random(monsterRng.randomLong()))
    crate::runtime::rng::shuffle_with_random_long(&mut bosses, monster_rng);

    bosses
}
