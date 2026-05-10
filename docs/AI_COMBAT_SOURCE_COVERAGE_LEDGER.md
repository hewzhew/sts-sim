# AI Combat Source Coverage Ledger

This ledger is the accountability surface between the decompiled Java source and
the Rust combat simulator. It exists so implementation cannot silently drop a
mechanic, hide an engine global, or call an incomplete Rust type "done".

Source root:

```text
D:\rust\cardcrawl
```

The ledger is not optional. Every combat-relevant source field, method family,
queue, RNG stream, screen state, and hook path must be classified before the
combat kernel can claim source coverage.

## Classification

```text
modeled:
  stored directly in CombatStateSnapshot

derived:
  recomputed deterministically from modeled state

render_only:
  UI/animation state that no mechanic reads

run_level_materialized:
  owned by future run-level code, but its combat effect must be materialized
  before CombatKernel::start

non_combat:
  not part of combat mechanics

unsupported_abort:
  known combat path not implemented; reaching it is non-trainable KernelAbort
```

`unsupported_abort` is not implementation. It is a blocker with a source
reference and exact missing behavior.

## Required Columns

Each row must contain:

```text
source_file
source_class
source_member
mechanic_role
classification
schema_path
public_visibility
replay_required
rust_owner_module
rust_status
migration_decision
acceptance_check
notes
```

## Initial Source Inventory

This is the first mandatory inventory. It is not complete enough to mark the
kernel done; it is the minimum table implementation must extend.

| Source file | Source member | Mechanic role | Classification | Schema path | Rust status |
| --- | --- | --- | --- | --- | --- |
| `dungeons/AbstractDungeon.java` | `player` | combat player singleton used by card, relic, potion, power, and monster code | modeled | `CombatStateSnapshot.player` | rewrite |
| `dungeons/AbstractDungeon.java` | `actionManager` | central action/card/monster queue driver | modeled | `CombatStateSnapshot.action_manager` | rewrite |
| `dungeons/AbstractDungeon.java` | `monsterRng`, `monsterHpRng`, `aiRng`, `shuffleRng`, `cardRandomRng`, `cardRng`, `miscRng`, `potionRng`, `relicRng`, `treasureRng` | combat RNG streams and possible combat-entry/end consumers | modeled or run_level_materialized per stream | `CombatStateSnapshot.rng` | rewrite |
| `dungeons/AbstractDungeon.java` | `gridSelectScreen`, `handCardSelectScreen`, `isScreenUp` | input boundary for card selection effects | modeled | `CombatStateSnapshot.choice_screens` | rewrite |
| `characters/AbstractPlayer.java` | `masterDeck`, `drawPile`, `hand`, `discardPile`, `exhaustPile`, `limbo` | card zones and card instance movement | modeled | `card_store`, `card_zones`, `player` | rewrite |
| `characters/AbstractPlayer.java` | `relics`, `blights`, `potions` | combat hooks, potion legality, inventory state | modeled | `relics`, `blights`, `potions` | rewrite |
| `characters/AbstractPlayer.java` | `energy`, `orbs`, `stance`, `cardInUse`, `damagedThisCombat`, hand-size fields | class combat state and legal action state | modeled | `player`, `orbs`, `stance`, `action_manager` | rewrite |
| `core/AbstractCreature.java` | `powers`, HP/block/lifecycle flags, `lastDamageTaken`, escape/death flags | shared player/monster combat state | modeled | `CreatureState` | rewrite |
| `cards/AbstractCard.java` | `uuid`, `cardID`, type/color/rarity/target/tags, upgrade/misc | concrete card identity and semantics | modeled | `CardInstance` | rewrite |
| `cards/AbstractCard.java` | `cost`, `costForTurn`, cost flags, `energyOnUse`, free/autoplay flags | legal play and X-cost behavior | modeled | `CardInstance` | rewrite |
| `cards/AbstractCard.java` | damage/block/magic/heal/draw/discard fields, `multiDamage`, damage types | rendered values and effect payload | modeled/derived | `CardInstance`, `DerivedCombatValues` | rewrite |
| `cards/CardGroup.java` | `group`, `type`, queued/in-hand bookkeeping when mechanical | ordered zones | modeled | `CardZone` | rewrite |
| `cards/CardQueueItem.java` | card, target, X-cost energy, ignore/autoplay/random/end-turn flags | queued card execution | modeled | `CardQueueItemState` | rewrite |
| `actions/GameActionManager.java` | `actions`, `preTurnActions`, `cardQueue`, `monsterQueue`, current/previous actions | pending execution state | modeled | `ActionManagerState` | rewrite |
| `actions/GameActionManager.java` | cards played this turn/combat, orb/stance history, damage/discard/energy counters, turn | public history and hook state | modeled | `ActionManagerState` | rewrite |
| `actions/AbstractGameAction.java` and subclasses | action type, source/target, amount, duration/done, subclass payload | resumable action queue | modeled or unsupported_abort per subclass | `ActionState` | rewrite |
| `rooms/AbstractRoom.java` | phase, monsters, battle-over flags, cannot-lose, elite/combat-event/smoke/mug flags | room combat lifecycle | modeled | `RoomCombatState` | rewrite |
| `rooms/AbstractRoom.java` | reward timing and rarity chance fields touched at combat end | terminal boundary and reward-RNG guard | modeled/run_level_materialized | `RoomCombatState`, `CombatLifecycleState` | rewrite |
| `monsters/AbstractMonster.java` | id, HP/block/powers/lifecycle, `halfDead`, escape/death flags | monster state | modeled | `MonsterState` | rewrite |
| `monsters/AbstractMonster.java` | `move`, `nextMove`, `moveHistory`, `damage`, intent fields | AI move and visible intent | modeled | `MonsterMoveState`, `IntentState` | rewrite |
| `monsters/MonsterGroup.java` | ordered monster list, spawn insertion, random monster selection | target identity and random target behavior | modeled | `MonsterGroupState` | rewrite |
| `monsters/EnemyMoveInfo.java` | next move, intent, base damage, multiplier, multi-damage flag | compact monster move payload | modeled | `EnemyMoveInfoState` | rewrite |
| `powers/AbstractPower.java` | owner, id, amount, priority, type, turn/post-action flags | power hooks and ordering | modeled | `PowerInstance` | rewrite |
| `powers/*Power.java` | concrete payload fields | power-specific mechanics | modeled or unsupported_abort per class | `PowerInstance.concrete_payload` | rewrite |
| `relics/AbstractRelic.java` | id, counter, used-up/grayscale, energy-based, discarded | relic hook state | modeled | `RelicInstance` | rewrite |
| `relics/*Relic.java` | concrete payload fields and combat hooks | relic-specific mechanics | modeled or unsupported_abort per class | `RelicInstance.concrete_payload` | rewrite |
| `potions/AbstractPotion.java` | id, slot, potency, can-use, target-required, thrown/discarded | potion legality/use state | modeled | `PotionInstance` | rewrite |
| `potions/*Potion.java` | concrete potion behavior and payload | potion-specific mechanics | modeled or unsupported_abort per class | `PotionInstance.concrete_payload` | rewrite |
| `orbs/AbstractOrb.java` | id, slot order, base/current passive/evoke amounts | Defect state and hooks | modeled | `OrbInstance` | rewrite |
| `stances/AbstractStance.java` | id and concrete stance behavior | Watcher stance hooks | modeled | `StanceState` | rewrite |
| `screens/select/GridCardSelectScreen.java` | selected cards, target group, amount, confirm/cancel, upgrade/transform/purge/any-number flags | grid choice state | modeled | `ChoiceScreenState.grid_select` | rewrite |
| `screens/select/HandCardSelectScreen.java` | selected cards, count, can-pick-zero/up-to/any-number/transform/upgrade/retrieval flags | hand choice state | modeled | `ChoiceScreenState.hand_select` | rewrite |
| `random/Random.java` | `RandomXS128` state and `counter` | deterministic RNG replay | modeled | `CombatRngState.RngStreamState` | keep/rewrite audit |

## Field Ledger: `dungeons/AbstractDungeon.java`

This table covers fields declared in `AbstractDungeon.java`. Abstract methods are
not fields and are not included here.

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `uiStrings` | non_combat | none | localization only |
| `TEXT` | non_combat | none | localization only |
| `name` | run_level_materialized | `DungeonCombatContext.dungeon_name` | dungeon metadata |
| `levelNum` | run_level_materialized | `DungeonCombatContext.level_num` | display/run metadata |
| `id` | run_level_materialized | `DungeonCombatContext.dungeon_id` | dungeon id can contextualize combat |
| `floorNum` | modeled | `DungeonCombatContext.floor_num` | combat/relic logic may inspect floor |
| `actNum` | modeled | `DungeonCombatContext.act_num` | combat/reward/end hooks may inspect act |
| `player` | modeled | `CombatStateSnapshot.player` | canonical player state |
| `unlocks` | non_combat | none | unlock UI/progression |
| `shrineChance` | non_combat | none | room generation only |
| `cardUpgradedChance` | run_level_materialized | `SourceManifest` / future run kernel | card reward/generation context, not combat step |
| `transformedCard` | modeled | `GlobalCombatTempState.transformed_card_ref` | temporary transform output used by generated/transform actions |
| `loading_post_combat` | modeled | `GlobalCombatTempState.loading_post_combat` | affects monster pre-battle/reward paths |
| `is_victory` | modeled | `GlobalCombatTempState.is_victory` | terminal/global victory state |
| `eventBackgroundImg` | render_only | none | texture |
| `monsterRng` | modeled | `CombatRngState.monster_rng` | monster selection/AI consumers |
| `mapRng` | run_level_materialized | future run kernel | map generation, not combat step |
| `eventRng` | run_level_materialized | future run kernel | event generation; materialize combat effects before start |
| `merchantRng` | run_level_materialized | future run kernel | shop generation |
| `cardRng` | modeled | `CombatRngState.card_rng` | card generation can occur in combat |
| `treasureRng` | run_level_materialized | `CombatRngState.treasure_rng_if_combat_consumed` | normally reward/chest; include only if combat path consumes |
| `relicRng` | run_level_materialized | `CombatRngState.relic_rng_if_combat_consumed` | normally reward; include if combat hook consumes |
| `potionRng` | modeled | `CombatRngState.potion_rng` | potion generation/use paths can occur during combat |
| `monsterHpRng` | modeled | `CombatRngState.monster_hp_rng` | encounter HP rolls |
| `aiRng` | modeled | `CombatRngState.ai_rng` | monster AI random choices |
| `shuffleRng` | modeled | `CombatRngState.shuffle_rng` | draw pile shuffle |
| `cardRandomRng` | modeled | `CombatRngState.card_random_rng` | random target/card effects |
| `miscRng` | modeled | `CombatRngState.misc_rng` | miscellaneous combat randomness |
| `srcColorlessCardPool` | modeled | `CombatContentPoolState.src_colorless_card_pool` | random card generation source |
| `srcCurseCardPool` | modeled | `CombatContentPoolState.src_curse_card_pool` | random curse generation source |
| `srcCommonCardPool` | modeled | `CombatContentPoolState.src_common_card_pool` | transform/random generation source |
| `srcUncommonCardPool` | modeled | `CombatContentPoolState.src_uncommon_card_pool` | transform/random generation source |
| `srcRareCardPool` | modeled | `CombatContentPoolState.src_rare_card_pool` | transform/random generation source |
| `colorlessCardPool` | modeled | `CombatContentPoolState.colorless_card_pool` | generated card choices |
| `curseCardPool` | modeled | `CombatContentPoolState.curse_card_pool` | generated curse choices |
| `commonCardPool` | modeled | `CombatContentPoolState.common_card_pool` | generated card choices |
| `uncommonCardPool` | modeled | `CombatContentPoolState.uncommon_card_pool` | generated card choices |
| `rareCardPool` | modeled | `CombatContentPoolState.rare_card_pool` | generated card choices |
| `commonRelicPool` | run_level_materialized | `CombatContentPoolState.common_relic_pool` | run/reward pool; keep if combat hook can inspect |
| `uncommonRelicPool` | run_level_materialized | `CombatContentPoolState.uncommon_relic_pool` | run/reward pool |
| `rareRelicPool` | run_level_materialized | `CombatContentPoolState.rare_relic_pool` | run/reward pool |
| `shopRelicPool` | run_level_materialized | `CombatContentPoolState.shop_relic_pool` | shop/reward pool |
| `bossRelicPool` | run_level_materialized | `CombatContentPoolState.boss_relic_pool` | boss reward pool |
| `lastCombatMetricKey` | non_combat | none | metrics/logging |
| `monsterList` | run_level_materialized | `CombatContentPoolState.monster_list` | encounter pool before combat |
| `eliteMonsterList` | run_level_materialized | `CombatContentPoolState.elite_monster_list` | encounter pool before combat |
| `bossList` | run_level_materialized | `CombatContentPoolState.boss_list` | boss encounter pool |
| `bossKey` | run_level_materialized | `DungeonCombatContext.boss_key` | known boss context, run-level |
| `eventList` | non_combat | none | event generation |
| `shrineList` | non_combat | none | event generation |
| `specialOneTimeEventList` | non_combat | none | event generation |
| `actionManager` | modeled | `CombatStateSnapshot.action_manager` | queue driver |
| `topLevelEffects` | render_only | none | visual effects |
| `topLevelEffectsQueue` | render_only | none | visual effects |
| `effectList` | render_only | none | visual effects |
| `effectsQueue` | render_only | none | visual effects |
| `turnPhaseEffectActive` | modeled | `GlobalCombatTempState.turn_phase_effect_active` | phase transition state; model until proven render-only |
| `colorlessRareChance` | modeled | `GlobalCombatTempState.colorless_rare_chance_bits` | generated colorless rarity state stored as raw float bits |
| `shopRoomChance` | non_combat | none | map generation |
| `restRoomChance` | non_combat | none | map generation |
| `eventRoomChance` | non_combat | none | map generation |
| `eliteRoomChance` | non_combat | none | map generation |
| `treasureRoomChance` | non_combat | none | map generation |
| `smallChestChance` | non_combat | none | chest generation |
| `mediumChestChance` | non_combat | none | chest generation |
| `largeChestChance` | non_combat | none | chest generation |
| `commonRelicChance` | non_combat | none | reward generation outside combat kernel |
| `uncommonRelicChance` | non_combat | none | reward generation outside combat kernel |
| `rareRelicChance` | non_combat | none | reward generation outside combat kernel |
| `scene` | render_only | none | scene rendering |
| `currMapNode` | run_level_materialized | `DungeonCombatContext.curr_map_node_ref` | room identity/context |
| `map` | non_combat | none | pathing/run kernel |
| `leftRoomAvailable` | non_combat | none | map UI |
| `centerRoomAvailable` | non_combat | none | map UI |
| `rightRoomAvailable` | non_combat | none | map UI |
| `firstRoomChosen` | non_combat | none | map/run state |
| `MAP_HEIGHT` | non_combat | none | map constant |
| `MAP_WIDTH` | non_combat | none | map constant |
| `MAP_DENSITY` | non_combat | none | map constant |
| `FINAL_ACT_MAP_HEIGHT` | non_combat | none | map constant |
| `rs` | render_only | none | render scene |
| `pathX` | non_combat | none | map path display/run history |
| `pathY` | non_combat | none | map path display/run history |
| `topGradientColor` | render_only | none | rendering |
| `botGradientColor` | render_only | none | rendering |
| `floorY` | render_only | none | rendering/position |
| `topPanel` | render_only | none | UI; potion inventory modeled separately |
| `cardRewardScreen` | non_combat | none | run-level reward decision |
| `combatRewardScreen` | non_combat | none | post-combat reward screen excluded from kernel |
| `bossRelicScreen` | non_combat | none | run-level reward decision |
| `deckViewScreen` | render_only | none | UI view |
| `discardPileViewScreen` | render_only | none | UI view |
| `gameDeckViewScreen` | render_only | none | UI view |
| `exhaustPileViewScreen` | render_only | none | UI view |
| `settingsScreen` | non_combat | none | settings UI |
| `inputSettingsScreen` | non_combat | none | settings UI |
| `dungeonMapScreen` | non_combat | none | map UI |
| `gridSelectScreen` | modeled | `ChoiceScreenState.grid_select` | combat card selection boundary |
| `handCardSelectScreen` | modeled | `ChoiceScreenState.hand_select` | combat hand selection boundary |
| `shopScreen` | non_combat | none | shop/run kernel |
| `creditsScreen` | non_combat | none | UI |
| `ftue` | non_combat | none | tutorial UI |
| `deathScreen` | non_combat | none | terminal UI |
| `victoryScreen` | non_combat | none | terminal UI |
| `unlockScreen` | non_combat | none | unlock UI |
| `gUnlockScreen` | non_combat | none | unlock UI |
| `isScreenUp` | modeled | `DungeonCombatContext.screen_state` | input boundary guard |
| `overlayMenu` | render_only | none | UI; cancel semantics modeled in choice state |
| `screen` | modeled | `DungeonCombatContext.screen_state` | active input boundary |
| `previousScreen` | modeled | `DungeonCombatContext.screen_state` | restore/cancel context if input boundary needs it |
| `dynamicBanner` | render_only | none | UI |
| `screenSwap` | render_only | none | UI transition |
| `isDungeonBeaten` | non_combat | none | run terminal outside combat kernel |
| `cardBlizzStartOffset` | run_level_materialized | `GlobalCombatTempState.card_blizz_start_offset` | reward card RNG randomizer |
| `cardBlizzRandomizer` | run_level_materialized | `GlobalCombatTempState.card_blizz_randomizer` | reward card RNG randomizer |
| `cardBlizzGrowth` | run_level_materialized | `GlobalCombatTempState.card_blizz_growth` | reward card RNG randomizer |
| `cardBlizzMaxOffset` | run_level_materialized | `GlobalCombatTempState.card_blizz_max_offset` | reward card RNG randomizer |
| `isFadingIn` | render_only | none | transition UI |
| `isFadingOut` | render_only | none | transition UI |
| `waitingOnFadeOut` | render_only | none | transition UI |
| `fadeTimer` | render_only | none | transition UI |
| `fadeColor` | render_only | none | transition UI |
| `sourceFadeColor` | render_only | none | transition UI |
| `nextRoom` | non_combat | none | map/run transition |
| `sceneOffsetY` | render_only | none | rendering |
| `relicsToRemoveOnStart` | run_level_materialized | `GlobalCombatTempState.relics_to_remove_on_start` | load/start cleanup; must be resolved before combat when possible |
| `bossCount` | run_level_materialized | `GlobalCombatTempState.boss_count` | run/reward context |
| `SCENE_OFFSET_TIME` | render_only | none | rendering constant |
| `isAscensionMode` | modeled | `DungeonCombatContext.is_ascension_mode` | ascension rules context |
| `ascensionLevel` | modeled | `DungeonCombatContext.ascension_level` | combat rules context |
| `blightPool` | run_level_materialized | future run kernel | pool only; owned blights modeled on player |
| `ascensionCheck` | non_combat | none | unlock/UI |
| `LOGGER` | non_combat | none | logging only |

## Field Ledger: `characters/AbstractPlayer.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `tutorialStrings` | non_combat | none | localization/tutorial |
| `MSG` | non_combat | none | localization/tutorial |
| `LABEL` | non_combat | none | localization/tutorial |
| `chosenClass` | modeled | `PlayerCombatState.player_class` | class-specific mechanics |
| `gameHandSize` | modeled | `PlayerCombatState.game_hand_size` | draw/hand limit |
| `masterHandSize` | modeled | `PlayerCombatState.master_hand_size` | hand size baseline |
| `startingMaxHP` | run_level_materialized | `PlayerCombatState.starting_max_hp` | run identity; not usually combat-mutated |
| `masterDeck` | modeled | `CardZoneState.master_deck` | deck identity for combat effects |
| `drawPile` | modeled | `CardZoneState.draw_pile` | ordered hidden draw state |
| `hand` | modeled | `CardZoneState.hand` | public hand state |
| `discardPile` | modeled | `CardZoneState.discard_pile` | discard state |
| `exhaustPile` | modeled | `CardZoneState.exhaust_pile` | exhaust state |
| `limbo` | modeled | `CardZoneState.limbo` | card-in-transition state |
| `relics` | modeled | `RelicState.relic_order` | relic hook order |
| `blights` | modeled | `BlightState.blight_order` | blight hook order |
| `potionSlots` | modeled | `PotionBeltState.slots` | potion capacity |
| `potions` | modeled | `PotionBeltState.potions` | potion inventory |
| `energy` | modeled | `PlayerCombatState.energy` | legal play state |
| `isEndingTurn` | modeled | `PlayerCombatState.is_ending_turn` | room update checks this during end turn |
| `viewingRelics` | render_only | none | controller/UI navigation gate |
| `inspectMode` | render_only | none | controller/UI navigation gate |
| `inspectHb` | render_only | none | hitbox only |
| `poisonKillCount` | non_combat | none | achievement metric |
| `damagedThisCombat` | modeled | `PlayerCombatState.damaged_this_combat` | combat/event/relic logic |
| `title` | non_combat | none | display |
| `orbs` | modeled | `OrbState.orb_refs_in_order` | Defect mechanics |
| `masterMaxOrbs` | modeled | `PlayerCombatState.master_max_orbs` | orb capacity baseline |
| `maxOrbs` | modeled | `OrbState.max_orbs` | current orb capacity |
| `stance` | modeled | `StanceState` | Watcher mechanics |
| `cardsPlayedThisTurn` | modeled | `PlayerCombatState.deprecated_cards_played_this_turn_counter` | deprecated but source-updated; keep for parity |
| `isHoveringCard` | render_only | none | UI targeting |
| `isHoveringDropZone` | render_only | none | UI targeting |
| `hoverStartLine` | render_only | none | UI targeting |
| `passedHesitationLine` | render_only | none | UI targeting |
| `hoveredCard` | render_only | none | UI targeting; legal actions use descriptors |
| `toHover` | render_only | none | UI targeting |
| `cardInUse` | modeled | `PlayerCombatState.card_in_use_ref` | card currently resolving |
| `isDraggingCard` | render_only | none | UI targeting |
| `isUsingClickDragControl` | render_only | none | UI targeting |
| `clickDragTimer` | render_only | none | UI targeting |
| `inSingleTargetMode` | render_only | none | UI targeting; target legality modeled by action descriptors |
| `hoveredMonster` | render_only | none | UI targeting; target refs modeled separately |
| `hoverEnemyWaitTimer` | render_only | none | UI tooltip delay |
| `HOVER_ENEMY_WAIT_TIME` | render_only | none | UI constant |
| `isInKeyboardMode` | render_only | none | UI input mode |
| `skipMouseModeOnce` | render_only | none | UI input mode |
| `keyboardCardIndex` | render_only | none | UI input mode |
| `customMods` | run_level_materialized | `PlayerCombatState.custom_mods` | custom mode can affect run/combat setup |
| `touchscreenInspectCount` | render_only | none | UI input |
| `img` | render_only | none | texture |
| `shoulderImg` | render_only | none | texture |
| `shoulder2Img` | render_only | none | texture |
| `corpseImg` | render_only | none | texture |
| `ARROW_COLOR` | render_only | none | targeting UI |
| `arrowScale` | render_only | none | targeting UI |
| `arrowScaleTimer` | render_only | none | targeting UI |
| `arrowX` | render_only | none | targeting UI |
| `arrowY` | render_only | none | targeting UI |
| `ARROW_TARGET_SCALE` | render_only | none | targeting UI constant |
| `TARGET_ARROW_W` | render_only | none | targeting UI constant |
| `HOVER_CARD_Y_POSITION` | render_only | none | layout constant |
| `endTurnQueued` | modeled | `PlayerCombatState.end_turn_queued` | source queues end-turn through player update |
| `SEGMENTS` | render_only | none | targeting UI constant |
| `points` | render_only | none | targeting UI |
| `controlPoint` | render_only | none | targeting UI |
| `arrowTmp` | render_only | none | targeting UI |
| `startArrowVector` | render_only | none | targeting UI |
| `endArrowVector` | render_only | none | targeting UI |
| `renderCorpse` | render_only | none | rendering |
| `uiStrings` | non_combat | none | localization |

## Field Ledger: `actions/GameActionManager.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `nextCombatActions` | modeled | `ActionManagerState.next_combat_actions` | queued at combat start |
| `actions` | modeled | `ActionManagerState.actions` | main action queue |
| `preTurnActions` | modeled | `ActionManagerState.pre_turn_actions` | start-turn action queue |
| `cardQueue` | modeled | `ActionManagerState.card_queue` | queued card execution |
| `monsterQueue` | modeled | `ActionManagerState.monster_queue` | queued monster turns |
| `cardsPlayedThisTurn` | modeled | `ActionManagerState.cards_played_this_turn` | card-count mechanics |
| `cardsPlayedThisCombat` | modeled | `ActionManagerState.cards_played_this_combat` | combat history mechanics |
| `orbsChanneledThisCombat` | modeled | `ActionManagerState.orbs_channeled_this_combat` | orb history mechanics |
| `orbsChanneledThisTurn` | modeled | `ActionManagerState.orbs_channeled_this_turn` | orb history mechanics |
| `uniqueStancesThisCombat` | modeled | `ActionManagerState.unique_stances_this_combat` | Watcher/history mechanics |
| `mantraGained` | modeled | `ActionManagerState.mantra_gained` | Watcher mantra history |
| `currentAction` | modeled | `ActionManagerState.current_action` | resumable execution |
| `previousAction` | modeled | `ActionManagerState.previous_action` | action history/resume |
| `turnStartCurrentAction` | modeled | `ActionManagerState.turn_start_current_action` | turn-start resume |
| `lastCard` | modeled | `ActionManagerState.last_card_ref` | card queue handling |
| `phase` | modeled | `ActionManagerState.phase` | waiting/executing boundary |
| `hasControl` | modeled | `ActionManagerState.has_control` | player-control boundary |
| `turnHasEnded` | modeled | `ActionManagerState.turn_has_ended` | turn lifecycle |
| `usingCard` | modeled | `ActionManagerState.using_card` | queue execution state |
| `monsterAttacksQueued` | modeled | `ActionManagerState.monster_attacks_queued` | monster turn lifecycle |
| `totalDiscardedThisTurn` | modeled | `ActionManagerState.total_discarded_this_turn` | discard hooks |
| `damageReceivedThisTurn` | modeled | `ActionManagerState.damage_received_this_turn` | turn damage hooks |
| `damageReceivedThisCombat` | modeled | `ActionManagerState.damage_received_this_combat` | combat damage hooks |
| `hpLossThisCombat` | modeled | `ActionManagerState.hp_loss_this_combat` | hp loss hooks/metrics |
| `playerHpLastTurn` | modeled | `ActionManagerState.player_hp_last_turn` | end-turn damage/loss state |
| `energyGainedThisCombat` | modeled | `ActionManagerState.energy_gained_this_combat` | energy history hooks |
| `turn` | modeled | `ActionManagerState.turn_index` | turn number |

## Blocker Rules

An implementation is not allowed to claim source coverage while any row has:

```text
schema_path = unknown
rust_owner_module = unknown
classification = unsupported_abort without source behavior description
classification = modeled without acceptance_check
```

## Acceptance

The ledger is usable for implementation only when:

```text
1. Every required source row above has a Rust owner decision.
2. Every reused Rust type has a migration-ledger entry.
3. Every unsupported row produces non-trainable KernelAbort.
4. The first combat probe references ledger rows for all mechanics it touches.
5. Any replay mismatch adds or corrects a ledger row before code is patched.
```
