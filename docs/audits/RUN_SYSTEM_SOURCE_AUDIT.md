# Run System Source Audit

This is the global audit entrypoint for Slay the Spire run mechanics that sit
above individual cards. The goal is not to design policy. The goal is to make
the simulator's run-level truth source-checkable before AI rollout work resumes.

Single-card, single-event, and single-relic audits are not enough. Many Java
rules are split between:

- content classes, such as an event or potion implementation;
- dungeon generation and pool filtering;
- room, reward, shop, map, or screen state;
- UI-hosted classes that carry real mechanical effects.

Woman in Blue is the canonical warning case: the event buttons do not check
gold, but `AbstractDungeon` filters the event out unless the player has at
least 50 gold. Both facts are true, and they belong to different layers.

## Audit Rule

For every run-level mechanic, record all of:

1. Java source files that define eligibility, execution, visibility, and RNG.
2. Rust files that implement the equivalent state transition.
3. The normal reachable condition, separate from handler behavior under a
   directly constructed or replayed state.
4. Public information visible to the player, separate from internal/oracle
   state.
5. Validation tests or a clear reason the path is still open.

Do not mark a mechanic clean if only the content handler was checked. Generation
and reachability gates must be checked separately.

## Source Areas

| Area | Java source roots | Rust source roots | Main risks | Status |
| --- | --- | --- | --- | --- |
| Event pools and one-time events | `dungeons/AbstractDungeon.java`, `events/**` | `src/events/generator.rs`, `src/content/events`, `src/engine/event_handler.rs` | event availability gates split from event button handlers; one-time removal; act/floor/gold/HP/relic/deck predicates | partial |
| Monster encounter pools | `dungeons/Exordium.java`, `dungeons/TheCity.java`, `dungeons/TheBeyond.java`, `monsters/**`, `helpers/MonsterHelper.java` | `src/monsters`, `src/encounters`, monster audit docs | pool weights, hard/elite/boss encounter selection, monster HP RNG, encounter history rules | open |
| Boss selection and visibility | `AbstractDungeon`, dungeon subclasses, map/boss key code | `src/state/run.rs`, map/encounter generation code | selected boss is known to player before the act boss; boss list/pool mutations; Act 4 special path | open |
| Map generation and path visibility | `map/**`, `dungeons/AbstractDungeon.java`, `rooms/**` | map/run state modules | visible route graph, next-node legality, winged movement, room symbols, hidden room contents vs known node types | open |
| Relic pools and `canSpawn` gates | `relics/**`, `helpers/RelicLibrary.java`, `AbstractDungeon` relic pool methods | `src/content/relics`, reward/shop/chest code | common/uncommon/rare/shop/boss pools, class-specific gates, floor/shop exclusions, replacement/removal, on-equip side effects | partial |
| Potion lifetime and legality | `potions/AbstractPotion.java`, `potions/**`, `ui/panels/PotionPopUp.java`, `rooms/**` | `src/content/potions`, combat legal moves, reward/shop/event obtain paths | discard availability, out-of-combat use exceptions, event-specific blocking, Sozu, Sacred Bark, Fairy/Blood passive or non-combat paths | partial |
| Reward generation and screens | `rooms/AbstractRoom.java`, `rewards/RewardItem.java`, `screens/CombatRewardScreen.java`, `screens/CardRewardScreen.java` | `src/rewards`, reward screen state, event reward handlers | card/relic/potion/gold reward order, no-card rewards, boss relic screens, skip/take-all semantics, reward RNG consumers | partial |
| Shop, remove, and purchase systems | `shop/**`, `rooms/ShopRoom.java`, relic shop hooks | `src/shop`, `src/engine/shop*`, relic purchase hooks | sale prices, purge cost, Courier refill, Smiling Mask, Membership Card, The Courier, Sozu shop potion behavior | partial |
| Rest site and campfire actions | `rooms/RestRoom.java`, `vfx/campfire/**`, campfire relic hooks | `src/engine/campfire_handler.rs`, relic hooks | rest/smith/toke/dig/lift/recall, UI-hosted mechanical effects, Dream Catcher, Girya, Shovel, Peace Pipe | partial |
| Chest and treasure rooms | `rooms/TreasureRoom*.java`, chest/relic code | treasure/reward/run state | chest reward RNG, Matryoshka, Cursed Key, Tiny Chest, boss chest vs normal chest hooks | open |
| Neow and run start | `neow/**`, character start deck/relic code | `src/state/run.rs`, Neow handlers | starting deck/relic correctness, choice visibility, reward RNG, class-specific starter replacements | partial |
| Between-act transition | `dungeons/AbstractDungeon.java`, boss proceed flow | `src/state/run.rs`, boss reward handlers | one-time heal, act increment, map/list reset, RNG counter alignment, boss chest timing | partial |
| Global visibility contract | map screens, reward screens, potion panel, top panel, AbstractDungeon fields | AI observation docs and run state | what policy may legally know: boss, map graph, event contents, reward choices, potion use/discard, relic counters | open |

## Immediate Execution Order

1. Event and shrine pool reachability:
   - compare every special event gate in `AbstractDungeon.java`;
   - add tests for Rust event generator eligibility;
   - keep handler semantics separate from normal reachability.

2. Potion global lifetime:
   - audit `canUse`, `canDiscard`, slot destruction, and out-of-combat
     exceptions;
   - connect combat potion audit with events, rewards, shop, and top-panel
     behavior.
   - current progress: observation now reflects Java top-panel affordances for
     Blood/Fruit/Entropic, `FairyPotion`, and `WeMeetAgain`; run-level
     non-combat use/discard execution is implemented for discard, Blood,
     Fruit, Entropic, Sozu, Sacred Bark, and Toy Ornithopter. Combat reward
     potion generation now follows Java's split: reward generation ignores
     Sozu, while reward claiming consumes the potion reward without obtaining
     a potion under Sozu.
   - current progress: Act 1/2 boss combat rewards no longer include a normal
     relic; boss relics are generated by the boss chest / boss relic selection
     path after the combat reward screen.
   - current progress: combat reward generation now follows the Java ordering
     `gold -> elite dropReward relics/key -> potion roll -> card rewards`; this
     matters because `addPotionToRewards()` checks the room reward count before
     rolling.
   - current progress: event-combat rewards no longer borrow the full monster
     combat reward generator just to obtain card rewards; event combat now keeps
     event rewards, rolls the EventRoom potion reward, and appends card rewards
     through separate helpers.
   - current progress: combat-time rewards, such as stolen gold, are now treated
     as pre-existing room rewards before ordinary room gold/relic/potion/card
     generation. Smoked combats still consume hidden room reward RNG but expose
     no visible reward items, matching `openCombat(..., smoked=true)`.
   - current progress: ordinary MonsterRoom rewards now respect Java
     `MonsterGroup.haveMonstersEscaped()`: if every monster escaped, standard
     monster gold is skipped and the room potion base chance starts at 0, while
     the potion RNG/miss-path still runs and White Beast Statue can still force
     the potion afterward.
   - current progress: event/start/relic potion sources now distinguish Java
     `PotionHelper.getRandomPotion()` from
     `AbstractDungeon.returnRandomPotion()`. Lab, Woman in Blue, Knowing Skull,
     and Neow's three-potion reward use the flat class potion pool; Neow opens
     potion rewards instead of directly filling slots.
   - current progress: `?` map nodes now follow Java's two-stage room entry:
     enter-room relic hooks see the original EventRoom node, then
     `EventHelper.roll` consumes `eventRng` and replaces the room with event,
     monster, shop, or treasure. Tiny Chest forces the post-roll result to
     treasure after still consuming the room-roll RNG; Juzu and previous-shop
     shop suppression are handled in the room roll, not in event selection.
   - current progress: exhausted event/shrine pools no longer fabricate a
     backup event. Java has no `Cleric` / `Golden Idol` / `Golden Shrine`
     fallback when all candidate pools are empty, so Rust now returns `None` in
     the inspectable event-generator path and fails explicitly in the ordinary
     wrapper.

3. Relic pool and `canSpawn` closure:
   - turn the existing relic audit into pool-level validation, not just
     per-relic behavior;
   - verify boss/shop/class/floor exclusions and replacement rules.
   - current progress: normal reward relic draws now model Java
     `returnRandomRelicKey` front-of-pool consumption, shop/end draws model
     `returnEndRandomRelicKey` back-of-pool consumption, and both paths apply
     the same `canSpawn` context. This fixes the previous single
     `random_relic_by_tier` path that treated all relic rewards like shop/end
     draws.
   - current progress: all Java relic `canSpawn()` overrides have been checked
     against `RelicSpawnContext` for standard non-Endless runs. The modeled
     gates cover starter-upgrade boss relic requirements, bottled relic deck
     predicates, floor cutoffs, current-ShopRoom exclusions, Ectoplasm's Act 1
     gate, campfire relic mutual exclusion, and Red Circlet/Circlet fallbacks.

4. Monster and boss generation:
   - verify encounter pools and boss visibility before touching AI pathing;
   - keep monster runtime intent/AI audit separate from encounter selection.
   - current progress: boss selection now separates Java `bossKey` from the
     internal `bossList`. Public observations use `bossKey`; `bossList` keeps
     the full Java queue so A20 Act 3 double-boss logic can test the
     post-entry `bossList.size() == 2` condition. Act 4 now initializes the
     The Ending encounter lists to Shield/Spear and the boss key/list to Heart.
     Encounter-list generation now has invariant coverage for Java list
     lengths, normal/elite repeat rules, first-strong exclusion handling, and
     Act 4 Shield/Spear lists.

5. Map and room visibility:
   - define what the player knows on the map at each point;
   - verify legal next-node movement and special movement relics.
   - current progress: Wing Boots movement now follows Java
     `MapRoomNode.wingedIsConnectedTo`: it can target other nodes on the next
     outgoing edge row, but cannot skip arbitrary future rows. Full-run legal
     actions now expose those flight choices as `FlyToNode`.

6. Reward, shop, rest, and chest screens:
   - source-check every screen that hosts mechanical state;
   - ignore render-only UI fields, but keep UI-hosted mechanics.

## Acceptance Standard

A run-level mechanic is not closed until it has:

- Java source references for eligibility and execution;
- a Rust implementation reference;
- a test for the normal reachable path;
- a test or explicit note for directly constructed/replay states when handler
  behavior differs from normal reachability;
- a public-visibility note for future AI observation work.

If any of these are missing, mark the mechanic `open` or `partial`, not clean.

## Boss Selection and Final Act Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/dungeons/Exordium.java`
- `D:/rust/cardcrawl/dungeons/TheCity.java`
- `D:/rust/cardcrawl/dungeons/TheBeyond.java`
- `D:/rust/cardcrawl/dungeons/TheEnding.java`
- `D:/rust/cardcrawl/rooms/MonsterRoomBoss.java`
- `D:/rust/cardcrawl/ui/buttons/ProceedButton.java`
- `D:/rust/cardcrawl/rooms/AbstractRoom.java`

Key source facts:

- Dungeon constructors call `initializeBoss()`, then `setBoss(bossList.get(0))`.
  The selected `bossKey` is the visible map boss.
- `initializeBoss()` does not always shuffle all bosses. In normal non-daily
  runs it first asks `UnlockTracker.isBossSeen(...)` in an act-specific order
  and forces the first unseen boss. If only one boss is selected, Java duplicates
  it. Only the all-seen path and daily runs shuffle all three bosses with one
  `monsterRng.randomLong()` seed.
- Exordium applies `Settings.isDemo` after the normal/daily boss-generation
  branch, clearing the list to only `Hexaghost`.
- `MonsterRoomBoss.onPlayerEntry()` calls `getBoss()` using `bossKey`, then
  removes `bossList[0]`.
- A20 Act 3 double boss is keyed off the post-entry queue size:
  `ascensionLevel >= 20 && bossList.size() == 2`.
- TheBeyond and TheEnding boss rooms skip normal combat reward-screen opening.
- TheEnding generates a fixed map and fills both normal and elite encounter
  lists with `Shield and Spear`; its boss list is `The Heart` repeated.

Rust result:

- `RunState::boss_key` now models the public Java `bossKey`.
- `RunState::boss_list` remains the internal Java boss queue and is no longer
  truncated at act start.
- Boss generation now has explicit `BossGenerationSettings` for daily/demo and
  seen-boss state. Standard simulator runs still use the all-bosses-seen
  profile, but the Java unseen-boss forcing order and its no-shuffle RNG
  behavior are tested rather than hidden in a comment.
- `RunState::next_boss()` uses `boss_key` and then removes the front of
  `boss_list`, matching `MonsterRoomBoss.onPlayerEntry()`.
- `RunState::should_start_act3_double_boss()` models the A20 post-entry queue
  test. The run loop now transitions from the first Act 3 boss directly to the
  second boss without generating normal rewards.
- `RunState::enter_final_act()` initializes the Act 4 map, encounter lists,
  boss key/list, event pools, and card-upgrade chance.

Coverage:

- `boss_key_is_public_boss_while_boss_list_keeps_java_queue`
- `final_act_initializes_shield_spear_and_heart_context`
- `act3_a20_first_boss_starts_second_boss_without_reward_or_victory`
- `act3_boss_with_all_keys_enters_initialized_final_act`
- `boss_lists_preserve_java_seen_boss_unlock_order`
- `boss_lists_shuffle_all_three_only_for_daily_or_all_seen_paths`
- `exordium_demo_overrides_after_java_boss_generation_branch`

## Monster Encounter Scheduling Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/dungeons/Exordium.java`
- `D:/rust/cardcrawl/dungeons/TheCity.java`
- `D:/rust/cardcrawl/dungeons/TheBeyond.java`
- `D:/rust/cardcrawl/dungeons/TheEnding.java`
- `D:/rust/cardcrawl/monsters/MonsterInfo.java`

Key source facts:

- `MonsterInfo.normalizeWeights` sorts entries by weight, divides by total,
  and `MonsterInfo.roll` uses the cumulative normalized list.
- Normal `populateMonsterList` rejects an encounter equal to the previous entry
  or the entry two positions back.
- Elite `populateMonsterList` rejects only immediate repeats.
- `populateFirstStrongEnemy` is a separate path: it rerolls only the
  act-specific exclusion list and does not apply the two-back normal-repeat
  rule.
- The Ending fills both normal and elite lists with `Shield and Spear`.

Rust result:

- Encounter scheduling uses the same weighted roll shape, repeat rules, and
  first-strong exclusion boundary.
- Exordium, City, and Beyond first-strong exclusion tables are covered directly
  against the Java `generateExclusions` cases.
- Act 1 generates 3 weak encounters, then 1 first strong encounter, then 12
  additional strong encounters, matching Java's call sequence rather than the
  misleading method name alone.
- Act 2 and Act 3 generate 2 weak encounters, then 1 first strong encounter,
  then 12 additional strong encounters.
- Act 4 encounter lists are fixed to Shield/Spear.

Coverage:

- `first_strong_exclusion_tables_match_java_sources`
- `encounter_lists_preserve_java_generation_invariants`

## Map Movement and Visibility Pass

Java sources checked:

- `D:/rust/cardcrawl/map/MapRoomNode.java`
- `D:/rust/cardcrawl/map/DungeonMap.java`

Key source facts:

- Normal map movement uses edge `(dstX, dstY)` matching.
- Winged Greaves / Wing Boots is not arbitrary vertical flight. The Java method
  checks every outgoing edge and allows a winged target when `node.y == edge.dstY`.
  It ignores X, but still stays on the next edge row.
- Boss entry is a special map boss hitbox path from row 14 to a synthetic boss
  room node. It is not a Wing Boots jump.
- TheEnding is the exception in `DungeonMap.update()`: the boss hitbox is
  available when the current node is the Shield/Spear node at row 2.

Rust result:

- `MapState::can_travel_to(..., has_flight=true)` now allows same-row flight
  across the next reachable row only.
- Public map observation derives `boss_node_available` from Java map position
  rules instead of relying only on a stored protocol/import field: normal acts
  expose it from row 14, and TheEnding exposes it when the current node has an
  outgoing edge to a `MonsterRoomBoss`.
- `MapState::set_current_room_type` is used when a Java `?` node rolls into
  an actual generated room, mirroring `nextRoom.room = generateRoom(roomResult)`
  rather than pretending every `?` is a real event.
- Full-run legal map actions include `FlyToNode(x, next_y)` only when Wing Boots
  has charges and normal edge travel would not already reach that node.
- Public next-node observation marks Wing Boots targets reachable without
  exposing multi-row jumps.
- Map observations now keep two Emerald-key concepts separate:
  `RunMapNodeObservationV0.has_emerald_key` is the Java
  `MapRoomNode.hasEmeraldKey` marker for the burning elite, while
  `RunMapObservationV0.has_emerald_key` is the player's owned key state
  (`Settings.hasEmeraldKey` / `RunState.keys[2]`).
- The full-run observation now also exposes all three top-panel key states
  explicitly as Ruby, Sapphire, and Emerald. These are public Java
  `Settings.hasRubyKey`, `Settings.hasSapphireKey`, and `Settings.hasEmeraldKey`
  state, not oracle data.

Coverage:

- `wing_boots_matches_java_next_row_only_semantics`
- `legal_map_actions_expose_wing_boots_only_on_next_row`
- `map_observation_separates_owned_emerald_key_from_emerald_elite_marker`
- `boss_node_availability_is_derived_from_java_map_position`
- `map_observation_derives_boss_node_availability_from_position`
- `run_observation_exposes_all_top_panel_keys`

## Between-Act Transition Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`

Key source facts:

- `dungeonTransitionSetup()` increments `actNum`, clears path/event/monster/
  elite/boss lists, resets event probabilities, and heals exactly once.
- Before the heal, Java aligns `cardRng.counter` upward to the next act band:
  1..249 -> 250, 251..499 -> 500, 501..749 -> 750. `Random.setCounter`
  consumes `randomBoolean()` calls instead of assigning the counter directly.
- Java resets `AbstractRoom.blizzardPotionMod` on every dungeon transition,
  including the transition into `TheEnding`.
- At Ascension 5+, the heal is
  `round((maxHealth - currentHealth) * 0.75)`.
- Below Ascension 5, the heal is full.

Rust result:

- `RunState::advance_act()` no longer applies the between-act heal twice.
- `RunState::advance_act()` now mirrors Java card reward RNG band alignment and
  advances the underlying RNG state while moving the counter.
- `RunState::advance_act()` and `RunState::enter_final_act()` now share the
  Java transition effects: card RNG alignment, potion pity reset, and healing.

Coverage:

- `advance_act_heals_once_like_java_dungeon_transition_setup`
- `advance_act_aligns_card_rng_counter_like_java_dungeon_transition_setup`
- `advance_act_resets_potion_drop_chance_like_java_dungeon_transition_setup`
- `final_act_initializes_shield_spear_and_heart_context`
- `advance_counter_to_matches_java_set_counter_random_boolean_consumption`

## Event Pool Reachability Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/dungeons/Exordium.java`
- `D:/rust/cardcrawl/dungeons/TheCity.java`
- `D:/rust/cardcrawl/dungeons/TheBeyond.java`

Rust sources checked:

- `src/events/context.rs`
- `src/events/generator.rs`
- `src/state/run.rs`

Source-backed candidate gates now covered by direct generator tests:

| Event | Java reachability rule | Rust rule |
| --- | --- | --- |
| `FountainOfCurseCleansing` | special one-time; player must be cursed | `ctx.has_curses` |
| `Designer` | Act 2 or Act 3; at least 75 gold | `(act == 2 || act == 3) && gold >= 75` |
| `Duplicator` | Act 2 or Act 3 | `act == 2 || act == 3` |
| `FaceTrader` | Act 1 or Act 2 | `act == 1 || act == 2` |
| `KnowingSkull` | Act 2; current HP greater than 12 | `act == 2 && current_hp > 12` |
| `Nloth` | Act 2; at least two relics | `act == 2 && relic_count >= 2` |
| `TheJoust` | Act 2; at least 50 gold | `act == 2 && gold >= 50` |
| `WomanInBlue` | at least 50 gold | `gold >= 50` |
| `NoteForYourself` | non-daily; A0, or A1-A14 lower than profile's highest unlocked ascension | explicit `is_daily_run`, `ascension_level`, and `highest_unlocked_ascension_level` in `EventContext` |
| `SecretPortal` | Act 3 and `CardCrawlGame.playtime >= 800.0f` | `act_num == 3 && playtime_seconds >= 800.0` |
| `DeadAdventurer` | floor greater than 6 | `floor_num > 6` |
| `Mushrooms` | floor greater than 6 | `floor_num > 6` |
| `MoaiHead` | has Golden Idol or HP percentage is at most 50% | `has_golden_idol || hp_pct <= 0.5` |
| `Cleric` | at least 35 gold | `gold >= 35` |
| `Beggar` | at least 75 gold | `gold >= 75` |
| `Colosseum` | current map node is past the map midpoint | `map_current_y > map_height / 2`, with `None` current node rejected |

Important boundary:

- Java decides `NoteForYourself` availability when initializing the special
  one-time event list. Rust now applies that initialization-time presence for
  normal `RunState::new` runs using the run's daily/ascension defaults; custom
  profile-highest-ascension fixtures must still construct the desired event pool
  explicitly if they diverge from those defaults.
- Java initializes the special one-time event list once in Exordium and carries
  the same list into later acts. Rust now preserves that lifetime: an exhausted
  one-time event pool is not repopulated by act transitions.
- Java ordinary `eventList` is act-local and events are removed as they appear;
  it is not rebuilt mid-act when emptied. Rust now preserves an exhausted
  ordinary event pool until the next act's event-list initialization.
- Java `EventHelper.roll` classifies `?` room replacements by filling a
  100-slot `RoomResult[]`, not by a pure cumulative threshold expression. Rust
  now mirrors the source's `Math.min(99, fillIndex)` fill behavior, including
  the final-slot overwrite when ramped Monster/Shop/Treasure chances overflow
  the 100-slot table.
- Java `TheEnding.initializeEventList()` and `initializeShrineList()` are empty.
  Rust now treats Act 4 and unknown act ids as empty event/shrine pools instead
  of falling through to Act 3 pools.
- Java `SecretPortal` is now represented in Rust as a special one-time Act 3
  event with the Java playtime gate. The Rust event handler maps accepting the
  portal to the boss combat boundary instead of modeling Java's UI room
  transition objects (`MapRoomNode(-1, 15)`, `pathX`, `pathY`).

Validation:

- `cargo test events::generator --all-targets`
- Latest full-suite validation after EventHelper room-roll table work:
  `cargo test --all-targets` -> `1078 passed`.

## Encounter Pool Reachability Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/dungeons/Exordium.java`
- `D:/rust/cardcrawl/dungeons/TheCity.java`
- `D:/rust/cardcrawl/dungeons/TheBeyond.java`
- `D:/rust/cardcrawl/dungeons/TheEnding.java`

Rust sources checked:

- `src/content/monsters/encounter_pool.rs`
- `src/state/run.rs`

Source-backed result:

- Java has concrete encounter-list initialization only for the known dungeons.
  `TheEnding.generateMonsters()` fills both normal and elite lists with
  `Shield and Spear`.
- Rust already modeled Act 4 as `ShieldAndSpear` lists. It now treats unknown
  act ids as empty encounter lists instead of silently falling back to Act 1.
- Java room creation reads the front of `monsterList` / `eliteMonsterList`;
  `nextRoomTransition()` removes that front entry only when leaving the current
  MonsterRoom or MonsterRoomElite. Rust still consumes the queue at combat
  creation through `RunState::next_encounter()` / `next_elite()`, so mid-room
  save/replay semantics remain open. The CLI/full-run smoke entrypoints no
  longer hide an exhausted queue by substituting `JawWorm`; an unexpected empty
  queue is now a hard failure until the room lifecycle is modeled directly.
- The same rule now applies to boss room creation: a missing `bossKey` /
  `boss_list` front is not replaced with `Hexaghost`.

## Boss Chest Relic Flow Pass

Java sources checked:

- `D:/rust/cardcrawl/rewards/chests/BossChest.java`
- `D:/rust/cardcrawl/screens/select/BossRelicSelectScreen.java`
- `D:/rust/cardcrawl/relics/AbstractRelic.java`
- `D:/rust/cardcrawl/ui/buttons/ProceedButton.java`

Key source facts:

- `BossChest.open(true)` opens `bossRelicScreen` in the current
  `TreasureRoomBoss`; it does not transition dungeons.
- Selecting a boss relic calls `AbstractRelic.bossObtainLogic()`, which obtains
  normal boss relics immediately. The room is marked `choseRelic`, but the
  dungeon transition happens later when the boss chest is left through the
  proceed flow.
- State-interrupting boss relics therefore run their on-equip selection in the
  old act, not after `dungeonTransitionSetup()`.

Rust result:

- Boss relic selection now obtains the selected relic before advancing the act.
- If the selected relic opens a run-level selection such as Astrolabe, Rust
  defers `advance_act()` until that selection resolves.
- Boss relic offers are generated by exactly three front-pool boss relic draws,
  matching `BossChest()`; no extra retry/dedup layer is applied.
- Starter upgrade boss relics (`Black Blood`, `Ring of the Serpent`,
  `FrozenCore`, `HolyWater`) now replace relic slot 0 before the act transition,
  matching `instantObtain(player, 0, true)`.
- `Ring of the Serpent` passive draw-size state is now present before combat
  start draw. Java implements this as `masterHandSize++` on equip and copies it
  into `gameHandSize` during combat setup.
- Defect combat setup now starts with Java's three `masterMaxOrbs` empty slots
  before Cracked Core / Frozen Core / Nuclear Battery pre-battle orb effects
  resolve.
- Non-Defect combat setup with `PrismaticShard` now starts with one empty
  `masterMaxOrbs` slot, matching `PrismaticShard.onEquip`.
- Chest-open hooks were checked against Java. The only relics overriding chest
  hooks are `Cursed Key`, `Matryoshka`, and `Nloth's Mask`; boss chests pass
  `bossChest=true`, Cursed Key/Nloth's Mask do nothing in that case, and
  `BossChest.open(true)` explicitly skips Matryoshka.

Coverage:

- `boss_relic_choice_obtains_normal_relic_before_advancing_act`
- `boss_relic_choice_defers_act_transition_until_on_equip_selection_resolves`
- `boss_starter_upgrade_relic_replaces_starter_slot_before_advancing_act`
- `boss_reward_generates_three_boss_relics_by_pool_order_without_retry_layer`
- `natural_combat_start_applies_ring_of_the_serpent_opening_hand_size`
- `natural_defect_combat_start_has_java_orb_slots_before_cracked_core`
- `natural_non_defect_prismatic_shard_combat_start_has_one_empty_orb_slot`

## Reward Card Pool / Prismatic Shard Pass

Java sources checked:

- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/helpers/CardLibrary.java`
- `D:/rust/cardcrawl/cards/CardGroup.java`
- `D:/rust/cardcrawl/cards/AbstractCard.java`

Key source facts:

- `AbstractDungeon.getRewardCards()` rolls rarity for each reward card, updates
  the card blizzard rare-chance offset, then repeatedly draws until the visible
  reward row has no duplicate `cardID`.
- Without `PrismaticShard`, the draw uses `AbstractDungeon.getCard(rarity)`,
  which selects from the current character rarity pool.
- With `PrismaticShard`, the draw uses
  `CardLibrary.getAnyColorCard(rarity)`. That helper scans all card-library
  entries, filters only by exact rarity and not Curse/Status type, then calls
  `CardGroup.shuffle(AbstractDungeon.cardRng)` and
  `getRandomCard(true, rarity).makeCopy()`.
- `CardGroup.getRandomCard(true, rarity)` sorts the rarity-filtered temporary
  list by `AbstractCard.cardID` before selecting with `AbstractDungeon.cardRng`.
  Because `getAnyColorCard(rarity)` already filtered to one rarity, the shuffle
  is mechanically important for consuming `cardRng.randomLong()`, even though
  the later sort removes its ordering effect.
- The rarity-only `getAnyColorCard` overload does not filter out HEALING-tagged
  cards. That differs from the type-specific overload used by
  `ForeignInfluenceAction`.

Rust result:

- Normal card rewards continue to use the current class rarity pool in existing
  Java runtime order.
- Card rewards while `PrismaticShard` is owned now use an any-color rarity pool
  containing implemented Ironclad, Silent, Defect, Watcher, and colorless reward
  cards, excluding only Curse/Status types.
- The Prismatic branch consumes `card_rng.random_long()` before selecting, then
  selects from a `java_id`-sorted pool with `card_rng.random()`, matching the
  Java `CardGroup.shuffle(...); getRandomCard(true, rarity)` RNG shape.
- This path is shared by ordinary combat rewards and reward-screen card rewards
  produced through Dream Catcher, Orrery, Tiny House, and similar callers of
  `generate_card_reward`.

Coverage:

- `normal_reward_pool_remains_current_class_only`
- `prismatic_reward_pool_uses_any_color_cards_sorted_by_java_id`
- `prismatic_reward_selection_consumes_card_rng_shuffle_seed_before_pick`

## Shop Colored Card Pool / Rarity Fallback Pass

Java sources checked:

- `D:/rust/cardcrawl/shop/Merchant.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/cards/CardGroup.java`

Key source facts:

- `Merchant` creates the five colored card sale slots as:
  `Attack`, `Attack`, `Skill`, `Skill`, `Power`.
- Each colored card rolls rarity through `AbstractDungeon.rollRarity()` and
  then calls `AbstractDungeon.getCardFromPool(rarity, type, true)`.
- Initial merchant colored cards therefore use `cardRng` for both rarity and
  `CardGroup.getRandomCard(type, true)` selection.
- Courier colored-card restock is different: `ShopScreen.purchaseCard` rolls
  rarity through `AbstractDungeon.rollRarity()` but then calls
  `getCardFromPool(..., false)`, so the concrete card selection goes through
  LibGDX `MathUtils.random` rather than `AbstractDungeon.cardRng`.
- `getCardFromPool` uses the current dungeon rarity pools. The `PrismaticShard`
  reward-card branch is not used here; owning the relic does not by itself make
  merchant colored cards draw from all colors.
- `getCardFromPool` does not have one generic fallback list:
  - Attack/Skill `Rare` falls through to `Uncommon`, then `Common`.
  - Attack/Skill `Uncommon` falls through to `Common`.
  - Attack/Skill `Common` does not fall through upward.
  - Power `Common` retries `Uncommon`, then `Rare`.
  - Power `Uncommon` retries `Rare`.
  - Power `Rare` falls through only to `Uncommon`.
- `CardGroup.getRandomCard(type, true)` sorts the type-filtered temporary list
  by Java card id before selecting with `AbstractDungeon.cardRng`.
- The merchant rejects a duplicate second Attack or duplicate second Skill by
  rolling a fresh rarity and card again. The Java loop has no attempt cap.

Rust result:

- Shop colored card selection now uses a Java-shaped rarity path instead of the
  old generic fallback that could incorrectly promote missing Common
  Attack/Skill slots to Uncommon/Rare or route missing Uncommon Power through
  Common.
- Courier colored-card restock now consumes `card_rng` only for rarity and uses
  the isolated simulator `math_rng` for `MathUtils.random` card selection. This
  keeps the mechanical Java source fact without letting UI-only MathUtils calls
  pollute `card_rng` or `misc_rng`.
- The old fake starter-card fallback was removed. If a required shop card pool
  is missing, Rust now fails loudly instead of silently producing a non-source
  card.
- The old 12-attempt cap on duplicate second Attack/Skill rerolls was removed;
  Java rerolls until a non-duplicate card is selected.
- Colored card candidates are still sorted by `java_id` before the `card_rng`
  index draw, matching `CardGroup.getRandomCard(type, true)`.

Coverage:

- `shop_attack_and_skill_rarity_paths_match_java_fallthrough`
- `shop_power_rarity_paths_match_java_recursive_power_fallbacks`
- `courier_colored_restock_uses_card_rng_for_rarity_and_math_rng_for_card_selection`

## Shop Restock Price Rounding Pass

Java sources checked:

- `D:/rust/cardcrawl/shop/ShopScreen.java`
- `D:/rust/cardcrawl/shop/StoreRelic.java`
- `D:/rust/cardcrawl/shop/StorePotion.java`

Key source facts:

- Initial shop inventory is priced, then global shop discounts are applied by
  `ShopScreen.applyDiscount`, which rounds each pass independently.
- Courier card restock uses `ShopScreen.setPrice(AbstractCard)`: card base price
  is jittered, colorless/Courier/Membership multipliers are applied as floats,
  then the final value is truncated to `int`.
- Courier relic and potion restock use `ShopScreen.getNewPrice(StoreRelic)` and
  `getNewPrice(StorePotion)`: jitter is rounded first, Courier discount is
  rounded, then Membership Card discount is rounded.

Rust result:

- Card restock keeps the source-shaped float multiplier path with final
  truncation.
- Relic and potion restock now apply Courier and Membership Card discounts as
  sequential rounded passes instead of one combined multiplier.

Coverage:

- `courier_membership_restock_relic_potion_discounts_round_sequentially`

## Shop Sozu Potion Purchase Pass

Java sources checked:

- `D:/rust/cardcrawl/shop/StorePotion.java`

Key source facts:

- `StorePotion.purchasePotion()` first checks `player.hasRelic("Sozu")`.
- If Sozu is present, Java flashes the relic and returns immediately.
- That return happens before the gold check, `obtainPotion`, purchase metrics,
  and Courier restock branch.

Rust result:

- Shop potion purchase under Sozu is now a no-op: no gold is spent, the offered
  potion remains in the shop, no potion is obtained, and Courier does not
  refill the slot.
- Full-run shop legal actions no longer expose `BuyPotion` when Sozu is present,
  matching the source behavior rather than the old absorbed-purchase model.

Coverage:

- `sozu_shop_potion_purchase_is_blocked_without_spending_or_removing_offer`
- `courier_does_not_refill_sozu_blocked_shop_potion_purchase`
- `legal_shop_actions_block_sozu_potion_purchase_like_java_store_potion`
