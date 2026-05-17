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
     Fruit, Entropic, Sozu, Sacred Bark, and Toy Ornithopter.

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

4. Monster and boss generation:
   - verify encounter pools and boss visibility before touching AI pathing;
   - keep monster runtime intent/AI audit separate from encounter selection.
   - current progress: boss selection now separates Java `bossKey` from the
     internal `bossList`. Public observations use `bossKey`; `bossList` keeps
     the full Java queue so A20 Act 3 double-boss logic can test the
     post-entry `bossList.size() == 2` condition. Act 4 now initializes the
     The Ending encounter lists to Shield/Spear and the boss key/list to Heart.

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

Rust result:

- `MapState::can_travel_to(..., has_flight=true)` now allows same-row flight
  across the next reachable row only.
- Full-run legal map actions include `FlyToNode(x, next_y)` only when Wing Boots
  has charges and normal edge travel would not already reach that node.
- Public next-node observation marks Wing Boots targets reachable without
  exposing multi-row jumps.

Coverage:

- `wing_boots_matches_java_next_row_only_semantics`
- `legal_map_actions_expose_wing_boots_only_on_next_row`

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
| `DeadAdventurer` | floor greater than 6 | `floor_num > 6` |
| `Mushrooms` | floor greater than 6 | `floor_num > 6` |
| `MoaiHead` | has Golden Idol or HP percentage is at most 50% | `has_golden_idol || hp_pct <= 0.5` |
| `Cleric` | at least 35 gold | `gold >= 35` |
| `Beggar` | at least 75 gold | `gold >= 75` |
| `Colosseum` | current map node is past the map midpoint | currently modeled as `floor_num > 7`; this is a proxy and remains a map-state follow-up |

Important boundary:

- Java decides `NoteForYourself` availability when initializing the special
  one-time event list. Rust still keeps the event in the pool and filters it at
  candidate selection time. This is mechanically acceptable for ordinary
  candidate generation, but exact empty-pool/fallback behavior remains open
  until event-list initialization is made context-aware.
- Java `SecretPortal` is a special one-time Act 3 portal event, but the Rust
  simulator intentionally excludes it from `EventId`; this remains documented in
  `EVENT_SOURCE_AUDIT.md` rather than silently treated as implemented.

Validation:

- `cargo test events::generator --all-targets`
- Latest full-suite validation after boss chest flow work:
  `cargo test --all-targets` -> `1022 passed`.

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
