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
| Event pools and one-time events | `dungeons/AbstractDungeon.java`, `events/**` | `src/events/generator.rs`, `src/content/events`, `src/engine/event_handler.rs` | event availability gates split from event button handlers; one-time removal; act/floor/gold/HP/relic/deck predicates | open |
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

3. Relic pool and `canSpawn` closure:
   - turn the existing relic audit into pool-level validation, not just
     per-relic behavior;
   - verify boss/shop/class/floor exclusions and replacement rules.

4. Monster and boss generation:
   - verify encounter pools and boss visibility before touching AI pathing;
   - keep monster runtime intent/AI audit separate from encounter selection.

5. Map and room visibility:
   - define what the player knows on the map at each point;
   - verify legal next-node movement and special movement relics.

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

