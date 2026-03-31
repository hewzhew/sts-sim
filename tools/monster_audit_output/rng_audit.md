# Monster RNG Consumption Audit

Java `rollMove()` always calls `aiRng.random(99)` before `getMove(num)`.
Each Rust `roll_move` must consume this RNG call as its first operation.

## 🔴 Missing Initial `random(99)` (47 monsters)

These Rust `roll_move` functions do NOT consume `random(99)` at the start,
causing RNG sequence desync with Java.

| Monster | Java `num` Used? | Java RNG Calls in getMove | Rust RNG Calls in roll_move | Rust File |
|---------|-----------------|---------------------------|-----------------------------|-----------|
| AcidSlime_M | ✅ Yes | randomBoolean(), randomBoolean(0.5f), randomBoolean(0.4f), randomBoolean(), randomBoolean(0.4f), randomBoolean(0.4f) | random_boolean() | acid_slime.rs |
| AcidSlime_S | ❌ No | randomBoolean() | random_boolean() | acid_slime.rs |
| Cultist | ❌ No | (none) | (none) | cultist.rs |
| GremlinFat | ❌ No | (none) | (none) | gremlin_fat.rs |
| GremlinNob | ✅ Yes | (none) | random(99) | gremlin_nob.rs |
| GremlinThief | ❌ No | (none) | (none) | gremlin_thief.rs |
| GremlinTsundere | ❌ No | (none) | (none) | gremlin_tsundere.rs |
| GremlinWarrior | ❌ No | (none) | (none) | gremlin_warrior.rs |
| GremlinWizard | ❌ No | (none) | (none) | gremlin_wizard.rs |
| Hexaghost | ❌ No | (none) | (none) | hexaghost.rs |
| Lagavulin | ❌ No | (none) | (none) | lagavulin.rs |
| Looter | ❌ No | (none) | random_boolean() | looter.rs |
| Sentry | ❌ No | (none) | (none) | sentry.rs |
| SlimeBoss | ❌ No | (none) | (none) | slime_boss.rs |
| SpikeSlime_M | ✅ Yes | (none) | (none) | spike_slime.rs |
| SpikeSlime_S | ❌ No | (none) | (none) | spike_slime.rs |
| TheGuardian | ❌ No | (none) | (none) | the_guardian.rs |
| BanditBear | ❌ No | (none) | (none) | bandit_bear.rs |
| BanditLeader | ❌ No | (none) | (none) | bandit_leader.rs |
| BanditPointy | ❌ No | (none) | (none) | bandit_pointy.rs |
| BronzeAutomaton | ❌ No | (none) | (none) | bronze_automaton.rs |
| Byrd | ✅ Yes | randomBoolean(0.375f), randomBoolean(0.4f), randomBoolean(0.375f), randomBoolean(0.2857f) | random_boolean_chance(0.375), random_range(0, 99), random_boolean_chance(0.4), random_boolean_chance(0.375), random_boolean_chance(0.2857) | byrd.rs |
| Centurion | ✅ Yes | (none) | (none) | centurion.rs |
| Champ | ✅ Yes | (none) | random_range(0, 99) | champ.rs |
| Chosen | ✅ Yes | (none) | random_range(0, 99) | chosen.rs |
| GremlinLeader | ✅ Yes | random(50, 99), random(0, 80) | (none) | gremlin_leader.rs |
| Healer | ✅ Yes | (none) | (none) | healer.rs |
| Mugger | ❌ No | (none) | (none) | mugger.rs |
| ShelledParasite | ✅ Yes | randomBoolean(), random(20, 99) | random_boolean(), random_range(0, 99) | shelled_parasite.rs |
| SphericGuardian | ❌ No | (none) | (none) | spheric_guardian.rs |
| Taskmaster | ❌ No | (none) | (none) | taskmaster.rs |
| TheCollector | ✅ Yes | (none) | (none) | the_collector.rs |
| TorchHead | ❌ No | (none) | (none) | torch_head.rs |
| AwakenedOne | ✅ Yes | (none) | random_range(0, 99), random_range(0, 99) | awakened_one.rs |
| Deca | ❌ No | (none) | (none) | deca.rs |
| Donu | ❌ No | (none) | (none) | donu.rs |
| Exploder | ❌ No | (none) | (none) | exploder.rs |
| Reptomancer | ✅ Yes | random(33, 99), random(65) | random_range(0, 99) | reptomancer.rs |
| Repulsor | ✅ Yes | (none) | (none) | repulsor.rs |
| SnakeDagger | ❌ No | (none) | (none) | snake_dagger.rs |
| Spiker | ✅ Yes | (none) | (none) | spiker.rs |
| SpireGrowth | ✅ Yes | (none) | random_range(0, 99) | spire_growth.rs |
| TimeEater | ✅ Yes | random(50, 99), randomBoolean(0.66f), random(74) | random_range(0, 99), random_range(50, 99), random_boolean_chance(0.66), random_range(0, 74) | time_eater.rs |
| Transient | ❌ No | (none) | (none) | transient.rs |
| CorruptHeart | ❌ No | randomBoolean() | random_boolean() | corrupt_heart.rs |
| SpireShield | ❌ No | randomBoolean() | random_boolean() | spire_shield.rs |
| SpireSpear | ❌ No | randomBoolean() | random_boolean() | spire_spear.rs |

## ✅ Correct (`random(99)` Present) (18 monsters)

| Monster | Java `num` Used? | Rust File |
|---------|------------------|-----------|
| AcidSlime_L | ✅ Yes | acid_slime_l.rs |
| FungiBeast | ✅ Yes | fungi_beast.rs |
| JawWorm | ✅ Yes | jaw_worm.rs |
| LouseDefensive | ✅ Yes | louse_defensive.rs |
| LouseNormal | ✅ Yes | louse_normal.rs |
| SlaverBlue | ✅ Yes | slaver_blue.rs |
| SlaverRed | ✅ Yes | slaver_red.rs |
| SpikeSlime_L | ✅ Yes | spike_slime_l.rs |
| BookOfStabbing | ✅ Yes | book_of_stabbing.rs |
| BronzeOrb | ✅ Yes | bronze_orb.rs |
| SnakePlant | ✅ Yes | snake_plant.rs |
| Snecko | ✅ Yes | snecko.rs |
| Darkling | ✅ Yes | darkling.rs |
| GiantHead | ✅ Yes | giant_head.rs |
| Maw | ✅ Yes | maw.rs |
| Nemesis | ✅ Yes | nemesis.rs |
| OrbWalker | ✅ Yes | orb_walker.rs |
| WrithingMass | ✅ Yes | writhing_mass.rs |

## ⚠️ No Rust File Found (3 monsters)

- ApologySlime (expected: None)
- HexaghostBody (expected: None)
- HexaghostOrb (expected: None)
