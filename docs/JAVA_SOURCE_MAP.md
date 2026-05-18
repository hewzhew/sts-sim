# Java Source Map

This file is the durable index for `D:\rust\cardcrawl`.

Use it before making a Java-backed mechanics claim. It exists because package
names are not enough: many real mechanics live inside screens, VFX, rooms, or
reward UI classes.

## Rules

- Do not claim a Java path from memory.
- Open the exact Java file before changing Rust behavior or adding a regression.
- Record the exact Java files in the test comment, commit message, handoff, or
  audit ledger.
- UI/VFX classes are not automatically render-only. Treat them as source
  witnesses when they mutate deck, rewards, relics, RNG, choices, or room state.
- If a search reveals a new source owner, update this map instead of relying on
  the next context window to remember it.

## Source Roots

| Alias | Path | Notes |
| --- | --- | --- |
| Java source | `D:\rust\cardcrawl` | Decompiled game source used as semantic reference |
| Rust simulator | `D:\rust\sts_simulator` | Headless simulator under repair |
| CommunicationMod | `D:\rust\CommunicationMod` | Live bridge/protocol reference, not simulator truth |

## Common Java Owners

| Mechanic family | Primary Java files | Rust owners seen so far | Notes |
| --- | --- | --- | --- |
| Ordinary delayed card obtain | `vfx/cardManip/ShowCardAndObtainEffect.java`, `cards/Soul.java`, `helpers/CardHelper.java` | `src/state/run.rs`, event modules | Constructor can Omamori-intercept curses. `update()` runs relic `onObtainCard`, then `souls.obtain`, then `onMasterDeckChange`. |
| Fast card obtain | `vfx/FastCardObtainEffect.java`, `cards/Soul.java` | `src/rewards/handler.rs`, `src/engine/shop_handler.rs`, `src/state/run.rs` | Used by reward screen, shop, Neow, and some grid flows. Same core obtain ordering as `ShowCardAndObtainEffect`. |
| Manual card obtain | Event/relic-specific Java files such as `events/shrines/NoteForYourself.java` | Event/relic modules plus `src/state/run.rs` | May bypass Omamori interception but still manually call relic `onObtainCard`. Audit each source separately. |
| Reward item claim | `rewards/RewardItem.java`, `screens/CombatRewardScreen.java`, `screens/CardRewardScreen.java` | `src/rewards/handler.rs`, `src/rewards/generator.rs` | `CARD` claim opens card reward screen and returns false; selecting card later queues `FastCardObtainEffect`. |
| Shop cards/relics/potions | `shop/ShopScreen.java`, `shop/StoreRelic.java`, `shop/StorePotion.java` | `src/engine/shop_handler.rs`, `src/shop/*` | Buying cards queues `FastCardObtainEffect` before spending gold; actual obtain hook resolves later. |
| Chest open and chest rewards | `rewards/chests/AbstractChest.java`, `rewards/chests/*Chest.java`, `rooms/TreasureRoom.java`, `relics/CursedKey.java` | `src/engine/run_loop.rs`, `src/rewards/*` | `onChestOpen` hooks precede chest gold/relic/key rewards; Cursed Key queues card obtain effect. |
| Combat-end room rewards | `rooms/AbstractRoom.java`, `rooms/MonsterRoom*.java`, `rewards/RewardItem.java` | `src/engine/run_loop.rs`, `src/rewards/generator.rs` | Boss/elite/normal gold and reward order live here, not in reward generator alone. |
| Card groups and master deck order | `cards/CardGroup.java`, `cards/Soul.java` | `src/runtime/combat.rs`, `src/state/run.rs`, `src/deck/*` | Java top card is array end; Rust draw pile top is currently index 0. Always check conversion API. |
| Card copy semantics | `cards/AbstractCard.java`, concrete card classes | `src/state/run.rs`, `src/runtime/combat.rs`, `src/content/cards/*` | `makeCopy` and `makeStatEquivalentCopy` preserve different state. Audit per producer. |
| Selection screens | `screens/select/GridCardSelectScreen.java`, `screens/select/HandCardSelectScreen.java`, `screens/CardRewardScreen.java` | `src/engine/pending_choices.rs`, `src/state/core.rs`, event modules | Screen code owns real selection state. Do not model UI; do model candidates, constraints, selected refs, and follow-up effects. |
| Map movement and visibility | `map/MapRoomNode.java`, `map/DungeonMap.java`, `dungeons/AbstractDungeon.java`, `ui/panels/TopPanel.java`, `ui/buttons/ProceedButton.java` | `src/map/*`, `src/state/run.rs`, full-run observation/action code | Boss key, top-panel keys, map graph, current node, boss node availability, Wing Boots reachability, and Emerald elite marker are public mechanics. Do not expose hidden `?` results or future event contents through observation. |
| Events | `events/**.java` | `src/content/events/*`, `src/events/generator.rs` | Event source controls option gates, immediate effects, delayed effects, RNG stream, and post-selection callbacks. |
| Event pools and `?` room roll | `dungeons/AbstractDungeon.java`, dungeon subclass `initializeEventList` / `initializeShrineList`, `helpers/EventHelper.java` | `src/events/generator.rs`, `src/state/run.rs`, `src/engine/run_loop.rs` | Pool reachability is separate from event button behavior. Preserve ordinary event gates, one-time shrine gates, pool removal, duplicate event RNG, `EventHelper.roll` probabilities, Juzu, Tiny Chest, and previous-shop suppression. |
| Relic base/hooks | `relics/AbstractRelic.java`, concrete `relics/*.java` | `src/content/relics/*`, `src/state/run.rs`, engine modules | Hook timing matters more than the relic id. Check `onEquip`, `instantObtain`, `onObtainCard`, `onMasterDeckChange`, `onChestOpen`, `onEnterRoom`, combat hooks. |
| Potions | `potions/AbstractPotion.java`, concrete `potions/*.java`, `ui/panels/PotionPopUp.java`, top panel use/discard code | `src/content/potions/*`, `src/engine/run_loop.rs`, combat handlers | Some potions are usable outside combat; discard is a top-panel affordance. Check `canUse`, `canDiscard`, target, slot behavior, and Sozu/Sacred Bark/Toy Ornithopter interactions. |
| Monster AI and intent | `monsters/AbstractMonster.java`, `monsters/**.java`, `monsters/EnemyMoveInfo.java`, `monsters/MonsterGroup.java` | `src/content/monsters/*`, `src/engine/targeting.rs` | Do not infer private fields from move history unless Java only uses history. Preserve visible intent separately from hidden move state. |
| Monster encounter scheduling and factory | `dungeons/AbstractDungeon.java`, `dungeons/Exordium.java`, `dungeons/TheCity.java`, `dungeons/TheBeyond.java`, `dungeons/TheEnding.java`, `monsters/MonsterInfo.java`, `helpers/MonsterHelper.java`, `rooms/MonsterRoom*.java` | `src/content/monsters/encounter_pool.rs`, `src/content/monsters/factory.rs`, `src/state/run.rs`, combat entrypoints | Encounter lists are generated up front from Java weighted pools and repeat rules. Java `MonsterHelper.getEncounter` owns encounter composition and constructor RNG side effects. Java room creation reads the front encounter and removes/regenerates when leaving the room; Rust currently consumes at combat creation, so room-lifecycle parity remains a named partial risk. |
| Actions and queue semantics | `actions/GameActionManager.java`, `actions/AbstractGameAction.java`, concrete `actions/**/*.java` | `src/runtime/action.rs`, `src/engine/action_handlers/*` | Queue insertion order, execution-time reads, and target liveness are frequent bug sources. |
| RNG implementation | `random/Random.java`, call sites in owner classes | `src/runtime/rng.rs`, `src/state/run.rs` | Every source family must name which Java RNG stream it consumes. |

## Recently Locked Obtain Entrypoints

| Entrypoint | Java files | Rust tests/commits |
| --- | --- | --- |
| Event `ShowCardAndObtainEffect` obtains | `vfx/cardManip/ShowCardAndObtainEffect.java`, many `events/**/*.java` | `81789e4`, `56bec7f`, `525fe0b`, `3426913`, `298b8b1`, `a3b9be9`, `1022eb3`, `b84fd78`, `a69430d` |
| `NoteForYourself` manual obtain | `events/shrines/NoteForYourself.java`, `cards/CardGroup.java` | `4d3d455` |
| Reward card selection | `rewards/RewardItem.java`, `screens/CardRewardScreen.java`, `vfx/FastCardObtainEffect.java`, `cards/Soul.java` | `dcec769` |
| Shop card purchase | `shop/ShopScreen.java`, `vfx/FastCardObtainEffect.java` | `b2cc6ce` |
| Cursed Key chest curse | `relics/CursedKey.java`, `rewards/chests/AbstractChest.java`, `helpers/CardLibrary.java` | `4895ac6` |
| Non-boss chest hook ordering | `rewards/chests/AbstractChest.java`, `rewards/chests/SmallChest.java`, `rewards/chests/MediumChest.java`, `rewards/chests/LargeChest.java`, `rooms/TreasureRoom.java`, `relics/Matryoshka.java`, `relics/NlothsMask.java`, `relics/CursedKey.java` | `72d5620` |
| Boss chest relic choice and hook exclusion | `rewards/chests/BossChest.java`, `screens/select/BossRelicSelectScreen.java`, `relics/AbstractRelic.java`, `rooms/TreasureRoomBoss.java`, `relics/CursedKey.java`, `relics/Matryoshka.java`, `relics/NlothsMask.java` | `03779ed` |
| Calling Bell curse and relic rewards | `relics/CallingBell.java`, `screens/select/GridCardSelectScreen.java`, `vfx/FastCardObtainEffect.java` | `72da496` |
| Necronomicon curse obtain | `relics/Necronomicon.java`, `vfx/cardManip/ShowCardAndObtainEffect.java` | `71c92b1` |
| Astrolabe transform-upgrade | `relics/Astrolabe.java`, `cards/CardGroup.java`, `dungeons/AbstractDungeon.java`, `vfx/cardManip/ShowCardAndObtainEffect.java` | `586fff0` |
| Pandora's Box starter replacement | `relics/PandorasBox.java`, `cards/CardGroup.java`, `screens/select/GridCardSelectScreen.java`, `vfx/FastCardObtainEffect.java`, `cards/Soul.java` | `0a795a8` |
| Tiny House upgrade and reward screen | `relics/TinyHouse.java`, `screens/CombatRewardScreen.java`, `rooms/AbstractRoom.java`, `rewards/RewardItem.java`, `cards/AbstractCard.java` | `72e808e` |
| Cauldron potion rewards and card-reward removal | `relics/Cauldron.java`, `helpers/PotionHelper.java`, `screens/CombatRewardScreen.java`, `rewards/RewardItem.java` | `00d2ecb` |
| Orrery card rewards and reward screen append | `relics/Orrery.java`, `rooms/AbstractRoom.java`, `screens/CombatRewardScreen.java`, `rewards/RewardItem.java` | `78aa564` |
| Dolly's Mirror stat-equivalent copy | `relics/DollysMirror.java`, `cards/AbstractCard.java`, `vfx/cardManip/ShowCardAndObtainEffect.java` | `c87f213` |
| Bottled relic equip selection | `relics/BottledFlame.java`, `relics/BottledLightning.java`, `relics/BottledTornado.java`, `cards/CardGroup.java`, `helpers/CardHelper.java` | `ff3b846` |
| Empty Cage purge selection | `relics/EmptyCage.java`, `cards/CardGroup.java`, `screens/select/GridCardSelectScreen.java` | `empty_cage_uses_java_purgeable_cards_and_auto_deletes_two_or_fewer` |
| Potion top-panel discard affordance | `ui/panels/PotionPopUp.java`, `potions/AbstractPotion.java` | `98208ad` |
| Fruit Juice combat use timing | `potions/FruitJuice.java`, `core/AbstractCreature.java`, `ui/panels/PotionPopUp.java`, `relics/MagicFlower.java`, `relics/ToyOrnithopter.java`, `actions/unique/FeedAction.java` | `combat_fruit_juice_increases_max_hp_immediately_before_toy_heal_queue`, `feed_max_hp_reward_uses_java_increase_max_hp_heal_hooks` |
