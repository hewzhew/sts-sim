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
| `actions/AbstractGameAction.java` and subclasses | action type/effect, source/target, amount, damage type, duration/start duration/done, subclass fields | resumable action queue | modeled or unsupported_abort per subclass | `ActionState`, `UnsupportedActionPayload` | rewrite |
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
| `monsterQueue` | modeled | `ActionManagerState.monster_queue` | queued `MonsterQueueItem` turns |
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

## Field Ledger: `actions/AbstractGameAction.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `DEFAULT_DURATION` | source_constant | action implementation constant | default action timing |
| `duration` | modeled | `ActionState.duration_bits` | Java `float`; raw bits required for replay |
| `startDuration` | modeled | `ActionState.start_duration_bits` | Java `float`; raw bits required for replay |
| `actionType` | modeled | `ActionState.action_type` | preserve all Java `ActionType` variants |
| `attackEffect` | modeled | `ActionState.attack_effect` | preserve all Java `AttackEffect` variants |
| `damageType` | modeled | `ActionState.damage_type` | nullable per action |
| `isDone` | modeled | `ActionState.is_done` | action completion state |
| `amount` | modeled | `ActionState.amount` | generic action amount |
| `target` | modeled | `ActionState.target` | nullable combatant ref |
| `source` | modeled | `ActionState.source` | nullable combatant ref |

`ActionType` variants from Java that must not be collapsed are: `BLOCK`,
`POWER`, `CARD_MANIPULATION`, `DAMAGE`, `DEBUFF`, `DISCARD`, `DRAW`,
`EXHAUST`, `HEAL`, `ENERGY`, `TEXT`, `USE`, `CLEAR_CARD_QUEUE`, `DIALOG`,
`SPECIAL`, `WAIT`, `SHUFFLE`, and `REDUCE_POWER`.

`AttackEffect` variants from Java that must not be collapsed are:
`BLUNT_LIGHT`, `BLUNT_HEAVY`, `SLASH_DIAGONAL`, `SMASH`, `SLASH_HEAVY`,
`SLASH_HORIZONTAL`, `SLASH_VERTICAL`, `NONE`, `FIRE`, `POISON`, `SHIELD`, and
`LIGHTNING`.

Subclasses may use `ActionState.unsupported_subclass_payload` only as a
quarantine record while a typed migration row is missing. Any frame containing
that payload is not trainable and not searchable. Unknown subclass state is an
`unsupported_abort`, not an ignored field and not a stringly-typed mechanism.

## Field Ledger: Core `actions/common/*Action.java` Subclasses

| Source | Field | Classification | Schema path | Notes |
| --- | --- | --- | --- | --- |
| `DamageAction.java` | `info` | modeled | `ActionState.damage_info` | concrete damage object |
| `DamageAction.java` | `goldAmount` | modeled | `ActionPayload::Damage.gold_amount` | steal-gold damage |
| `DamageAction.java` | `skipWait` | modeled | `ActionPayload::Damage.skip_wait` | controls post-hit wait action |
| `DamageAction.java` | `muteSfx` | modeled | `ActionPayload::Damage.mute_sfx` | controls attack VFX sound |
| `DamageAllEnemiesAction.java` | `damage` | modeled | `ActionPayload::DamageAllEnemies.damage` | per-monster damage array |
| `DamageAllEnemiesAction.java` | `baseDamage` | modeled | `ActionPayload::DamageAllEnemies.base_damage` | source base for dynamic matrix |
| `DamageAllEnemiesAction.java` | `firstFrame` | modeled | `ActionPayload::DamageAllEnemies.first_frame` | delayed execution state |
| `DamageAllEnemiesAction.java` | `utilizeBaseDamage` | modeled | `ActionPayload::DamageAllEnemies.utilize_base_damage` | recompute damage matrix |
| `GainBlockAction.java` | no subclass fields beyond common action fields | modeled | `ActionState` | duration/startDuration/amount/target/source cover it |
| `DrawCardAction.java` | `shuffleCheck` | modeled | `ActionPayload::DrawCard.shuffle_check` | shuffle split state |
| `DrawCardAction.java` | `drawnCards` | modeled | `ActionStaticState.draw_card_action_drawn_cards` | static mechanical history used by follow-up actions |
| `DrawCardAction.java` | `clearDrawHistory` | modeled | `ActionPayload::DrawCard.clear_draw_history` | controls static drawn-card reset |
| `DrawCardAction.java` | `followUpAction` | modeled | `ActionPayload::DrawCard.follow_up_action` | queued after draw completes |
| `DiscardAction.java` | `p` | modeled/derived | `ActionState.target` | player target |
| `DiscardAction.java` | `isRandom` | modeled | `ActionPayload::Discard.is_random` | random discard branch |
| `DiscardAction.java` | `endTurn` | modeled | `ActionPayload::Discard.end_turn` | manual discard trigger behavior |
| `DiscardAction.java` | `numDiscarded` | modeled | `ActionStaticState.discard_action_num_discarded` | static hand-select counter |
| `ExhaustAction.java` | `p` | modeled/derived | `ActionState.target` | player target |
| `ExhaustAction.java` | `isRandom` | modeled | `ActionPayload::Exhaust.is_random` | random exhaust branch |
| `ExhaustAction.java` | `anyNumber` | modeled | `ActionPayload::Exhaust.any_number` | hand select constraint |
| `ExhaustAction.java` | `canPickZero` | modeled | `ActionPayload::Exhaust.can_pick_zero` | hand select constraint |
| `ExhaustAction.java` | `numExhausted` | modeled | `ActionStaticState.exhaust_action_num_exhausted` | static hand-select counter |
| `GainEnergyAction.java` | `energyGain` | modeled | `ActionPayload::GainEnergy.energy_gain` | energy gained and hand trigger amount |
| `EmptyDeckShuffleAction.java` | `shuffled` | modeled | `ActionPayload::EmptyDeckShuffle.shuffled` | delayed shuffle state |
| `EmptyDeckShuffleAction.java` | `vfxDone` | modeled | `ActionPayload::EmptyDeckShuffle.vfx_done` | delayed discard movement state |
| `EmptyDeckShuffleAction.java` | `count` | modeled | `ActionPayload::EmptyDeckShuffle.count` | discard movement counter |

The source files above are the first typed migration target because they cover
basic damage, block, draw, discard, exhaust, shuffle, and energy loops. Other
action subclasses must be added to this ledger before they are allowed out of
`UnsupportedActionPayload` quarantine.

## Field Ledger: `cards/DamageInfo.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `owner` | modeled | `DamageInfoState.owner` | nullable source creature |
| `name` | modeled | `DamageInfoState.name` | source field; nullable |
| `type` | modeled | `DamageInfoState.damage_type` | `NORMAL`, `THORNS`, `HP_LOSS` |
| `base` | modeled | `DamageInfoState.base` | unmodified base value |
| `output` | modeled | `DamageInfoState.output` | post power/stance/final hooks value |
| `isModified` | modeled | `DamageInfoState.is_modified` | whether source hooks changed damage |

Damage matrix helpers are behavior, not additional state. Their outputs must be
represented through concrete `DamageInfoState` entries or card rendered values
where the Java runtime stores them.

## Field Ledger: `monsters/MonsterQueueItem.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `monster` | modeled | `MonsterQueueItemState.monster_ref` | queued monster turn item |

## Field Ledger: `core/EnergyManager.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `energy` | modeled | `EnergyState.turn_energy` | turn recharge amount used by `recharge()` |
| `energyMaster` | modeled | `EnergyState.energy_master` | base per-turn energy |

## Field Ledger: `ui/panels/EnergyPanel.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `totalCount` | modeled | `EnergyState.panel_total_count` | current spendable energy |
| `fontScale` | render_only | none | energy number animation |
| `energyVfxTimer` | render_only | none | energy VFX timing |
| `ENERGY_VFX_TIME`, `VFX_ROTATE_SPEED`, texture/color/hitbox fields | render_only | none | UI only |

## Field Ledger: `core/AbstractCreature.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `name` | modeled | `CreatureState.name_id` | public identity/display id |
| `id` | modeled | `CreatureState.creature_id` | combat identity |
| `powers` | modeled | `CreatureState.powers` | ordered power refs |
| `isPlayer` | modeled | `CreatureState.is_player` | combatant kind |
| `isBloodied` | modeled | `CreatureState.is_bloodied` | source field; keep until proven render-only |
| `drawX`, `drawY`, `dialogX`, `dialogY` | render_only | none | render/tooltip position |
| `hb` | render_only | none | hitbox; target legality uses public refs instead |
| `gold` | modeled | `CreatureState.gold` | player gold can change in combat effects |
| `displayGold` | modeled | `CreatureState.display_gold` | source state; likely render, keep until parity says derived |
| `isDying` | modeled | `CreatureState.lifecycle` | death lifecycle |
| `isDead` | modeled | `CreatureState.lifecycle` | death lifecycle |
| `halfDead` | modeled | `CreatureState.half_dead` | boss/minion special death behavior |
| `flipHorizontal`, `flipVertical` | render_only | none | render orientation |
| `escapeTimer` | modeled | `CreatureState.escape_timer_bits` | Java timer controls escape completion; Rust may structurally redesign later |
| `isEscaping` | modeled | `CreatureState.escape_state` | escape lifecycle |
| `TIP_X_THRESHOLD`, `MULTI_TIP_Y_OFFSET`, `TIP_OFFSET_R_X`, `TIP_OFFSET_L_X`, `TIP_OFFSET_Y` | render_only | none | tooltip constants |
| `tips` | render_only | none | UI tips |
| `uiStrings`, `TEXT` | non_combat | none | localization |
| `healthHb`, `healthHideTimer`, health bar width/timer/color fields, `hbAlpha`, `hbYOffset` | render_only | none | health UI rendering |
| `lastDamageTaken` | modeled | `CreatureState.last_damage_taken` | damage hook/history state |
| `hb_x`, `hb_y`, `hb_w`, `hb_h` | render_only | none | hitbox geometry |
| `currentHealth` | modeled | `CreatureState.hp` | HP |
| `maxHealth` | modeled | `CreatureState.max_hp` | max HP |
| `currentBlock` | modeled | `CreatureState.block` | block |
| block/health UI constants and colors | render_only | none | rendering constants |
| `tint`, `sr`, `shakeToggle`, shake constants | render_only | none | VFX/rendering |
| `animX`, `animY`, `vX`, `vY`, `animation`, `animationTimer`, animation constants | render_only | none | animation |
| `atlas`, `skeleton`, `state`, `stateData` | render_only | none | animation runtime |
| `RETICLE_W`, `reticleAlpha`, `reticleColor`, `reticleShadowColor`, `reticleRendered`, `reticleOffset`, `reticleAnimTimer`, `RETICLE_OFFSET_DIST` | render_only | none | targeting UI |

## Field Ledger: `cards/AbstractCard.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `type` | modeled | `CardInstance.card_type` | card mechanics |
| `cost` | modeled | `CardInstance.cost` | base cost |
| `costForTurn` | modeled | `CardInstance.cost_for_turn` | current cost |
| `price` | run_level_materialized | `CardInstance.price` | shop/run value; preserved on card instance |
| `chargeCost` | modeled | `CardInstance.charge_cost` | X-cost/charge behavior |
| `isCostModified` | modeled | `CardInstance.is_cost_modified` | cost display/logic |
| `isCostModifiedForTurn` | modeled | `CardInstance.is_cost_modified_for_turn` | cost display/logic |
| `retain` | modeled | `CardInstance.retain` | end-turn discard behavior |
| `selfRetain` | modeled | `CardInstance.self_retain` | end-turn discard behavior |
| `dontTriggerOnUseCard` | modeled | `CardInstance.dont_trigger_on_use_card` | on-use hook suppression |
| `rarity` | modeled | `CardInstance.rarity` | generation/reward context |
| `color` | modeled | `CardInstance.color` | card pool/color mechanics |
| `isInnate` | modeled | `CardInstance.innate` | opening hand behavior |
| `isLocked` | non_combat | `CardInstance.is_locked` | unlock state; retained for source parity but not combat mechanics |
| `showEvokeValue` | modeled | `CardInstance.show_evoke_value` | Defect hover/orb preview; keep because source reads it in player update |
| `showEvokeOrbCount` | modeled | `CardInstance.show_evoke_orb_count` | Defect hover/orb preview |
| `keywords` | modeled | `CardInstance.keywords` | card metadata; may affect public text/observation |
| card price constants | non_combat | none | constants |
| `isUsed` | modeled | `CardInstance.is_used` | use-state flag |
| `upgraded` | modeled | `CardInstance.upgraded` | upgrade state |
| `timesUpgraded` | modeled | `CardInstance.times_upgraded` | upgrade count |
| `misc` | modeled | `CardInstance.misc` | card-specific persistent value |
| `energyOnUse` | modeled | `CardInstance.energy_on_use` | X-cost and queued use |
| `ignoreEnergyOnUse` | modeled | `CardInstance.ignore_energy_on_use` | queued/autoplay use |
| `isSeen` | non_combat | `CardInstance.is_seen` | unlock/compendium; retained for source parity |
| `upgradedCost` | modeled | `CardInstance.upgraded_cost` | upgrade display/state |
| `upgradedDamage` | modeled | `CardInstance.upgraded_damage` | upgrade display/state |
| `upgradedBlock` | modeled | `CardInstance.upgraded_block` | upgrade display/state |
| `upgradedMagicNumber` | modeled | `CardInstance.upgraded_magic_number` | upgrade display/state |
| `uuid` | modeled | `CardInstance.source_uuid` | duplicate card identity |
| `isSelected` | modeled | `CardInstance.is_selected` | grid/hand selection state |
| `exhaust` | modeled | `CardInstance.exhaust` | post-use behavior |
| `returnToHand` | modeled | `CardInstance.return_to_hand` | post-use behavior |
| `shuffleBackIntoDrawPile` | modeled | `CardInstance.shuffle_back_into_draw_pile` | post-use behavior |
| `isEthereal` | modeled | `CardInstance.ethereal` | end-turn exhaust |
| `tags` | modeled | `CardInstance.tags` | tag-based mechanics |
| `multiDamage` | modeled | `CardInstance.multi_damage` | multi-target damage |
| `isMultiDamage` | modeled | `CardInstance.is_multi_damage` | multi-target damage flag |
| base/current damage, block, magic, heal, draw, discard fields | modeled | `CardInstance.*` value fields | source fields mirrored in `CardInstance` |
| `isDamageModified` | modeled | `CardInstance.is_damage_modified` | rendered/combat value |
| `isBlockModified` | modeled | `CardInstance.is_block_modified` | rendered/combat value |
| `isMagicNumberModified` | modeled | `CardInstance.is_magic_number_modified` | rendered/combat value |
| `damageType` | modeled | `CardInstance.damage_type` | damage semantics |
| `damageTypeForTurn` | modeled | `CardInstance.damage_type_for_turn` | current damage semantics |
| `target` | modeled | `CardInstance.target` | target legality |
| `purgeOnUse` | modeled | `CardInstance.purge_on_use` | post-use behavior |
| `exhaustOnUseOnce` | modeled | `CardInstance.exhaust_on_use_once` | post-use behavior |
| `exhaustOnFire` | modeled | `CardInstance.exhaust_on_fire` | fire/exhaust behavior |
| `freeToPlayOnce` | modeled | `CardInstance.free_to_play_once` | cost behavior |
| `isInAutoplay` | modeled | `CardInstance.is_in_autoplay` | queued/autoplay behavior |
| static atlases, orb regions, portraits, colors, dimensions, hitboxes, glow timers, hover timers, card render strings | render_only | none | rendering/UI only |
| `assetUrl` | render_only | none | asset path |
| `fadingOut`, `transparency`, `targetTransparency`, `targetAngle`, `angle`, position/scale fields | render_only | none | UI animation |
| `cardsToPreview` | render_only | none | card preview UI |
| `originalName` | modeled | `CardInstance.original_name_id` | source identity/display |
| `name` | modeled | `CardInstance.name_id` | source identity/display |
| `rawDescription`, `description`, `cantUseMessage` | modeled | `CardInstance.cant_use_message` and payload | public text/unplayable reason; full text can be payload |
| `cardID` | modeled | `CardInstance.card_id` | card identity |
| `inBottleFlame` | run_level_materialized | `CardInstance.in_bottle_flame` | bottled innate/run state |
| `inBottleLightning` | run_level_materialized | `CardInstance.in_bottle_lightning` | bottled innate/run state |
| `inBottleTornado` | run_level_materialized | `CardInstance.in_bottle_tornado` | bottled innate/run state |
| `glowColor` | render_only | none | UI glow |

## Field Ledger: `cards/CardGroup.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `group` | modeled | `CardZone.ordered_card_refs` | card order |
| `HAND_START_X`, `HAND_OFFSET_X`, hand push constants, draw/discard pile coordinates | render_only | none | layout |
| `type` | modeled | `CardZone.group_type` / `zone_kind` | zone behavior |
| `handPositioningMap` | modeled | `CardZone.hand_positioning_map` | source state; likely UI but kept until parity proves derived |
| `queued` | modeled | `CardZone.queued_card_refs` | internal queue bookkeeping |
| `inHand` | modeled | `CardZone.in_hand_refs` | hand bookkeeping |

## Field Ledger: `cards/CardQueueItem.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `card` | modeled | `CardQueueItemState.card_ref` | queued card |
| `monster` | modeled | `CardQueueItemState.monster_ref` | queued target |
| `energyOnUse` | modeled | `CardQueueItemState.energy_on_use` | queued X-cost/current energy |
| `ignoreEnergyTotal` | modeled | `CardQueueItemState.ignore_energy_total` | queued cost behavior |
| `autoplayCard` | modeled | `CardQueueItemState.autoplay_card` | autoplay behavior |
| `randomTarget` | modeled | `CardQueueItemState.random_target` | random target behavior |
| `isEndTurnAutoPlay` | modeled | `CardQueueItemState.is_end_turn_auto_play` | Unceasing Top/end-turn autoplay behavior |

## Field Ledger: `monsters/AbstractMonster.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger`, `uiStrings`, `TEXT`, `MOVES`, `DIALOG` | non_combat | none | logging/localization |
| `DEATH_TIME`, `ESCAPE_TIME`, `ESCAPE`, `ROLL`, intent constants | modeled/derived | `MonsterState`, `MonsterMoveState` | byte constants influence move/escape semantics |
| `deathTimer` | modeled | `MonsterState.death_timer_bits` | revive/death timing state; Rust may redesign only with parity |
| `nameColor`, `nameBgColor`, `img`, intent textures/colors/particles/bob, disposable render assets | render_only | none | rendering |
| `tintFadeOutCalled` | modeled | `MonsterState.tint_fade_out_called` | revive actions reset/read death fade state |
| `moveSet` | modeled | `MonsterState.move_set` | move byte/name mapping |
| `escaped` | modeled | `MonsterState.escaped` | escape lifecycle |
| `escapeNext` | modeled | `MonsterState.escape_next` | escape lifecycle |
| `intentTip` | render_only | none | tooltip |
| `type` | modeled | `MonsterState.enemy_type` | normal/elite/boss classification |
| `hoverTimer`, `intentHb`, `intentAlpha`, `intentAlphaTarget`, `intentOffsetX`, `intentAngle` | render_only | none | UI/rendering |
| `cannotEscape` | modeled | `MonsterState.cannot_escape` | escape legality |
| `damage` | modeled | `MonsterState.damage_entries` | multi-hit damage list |
| `move` | modeled | `MonsterMoveState.enemy_move_info` | next move payload |
| `intentParticleTimer`, `intentVfx` | render_only | none | VFX |
| `moveHistory` | modeled | `MonsterMoveState.move_history` | AI history |
| `nextMove` | modeled | `MonsterMoveState.next_move` | next move byte |
| `intent` | modeled | `IntentState.intent_kind` | visible/mechanical intent |
| `tipIntent` | modeled | `IntentState.tip_intent_kind` | intent tooltip/public display |
| `intentDmg` | modeled | `IntentState.displayed_damage` | displayed/current intent damage |
| `intentBaseDmg` | modeled | `IntentState.base_damage` | base intent damage |
| `intentMultiAmt` | modeled | `IntentState.hit_count` | multi-hit count |
| `isMultiDmg` | modeled | `IntentState.is_multi_damage` | multi-hit flag |
| `moveName` | modeled | `MonsterMoveState.move_name_id` | ShowMoveNameAction reads and clears it |
| `sortByHitbox` | render_only | none | UI sorting |

## Field Ledger: `monsters/MonsterGroup.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging only |
| `monsters` | modeled | `MonsterGroupState.monsters_in_slot_order` and `monsters` | monster order and identities |
| `hoveredMonster` | render_only | `MonsterGroupState.hovered_monster_ref_if_mechanical` only if a mechanic reads it | normally UI tooltip/targeting |

## Field Ledger: `monsters/EnemyMoveInfo.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `nextMove` | modeled | `EnemyMoveInfoState.next_move` | move byte |
| `intent` | modeled | `EnemyMoveInfoState.intent` | intent enum |
| `baseDamage` | modeled | `EnemyMoveInfoState.base_damage` | base damage |
| `multiplier` | modeled | `EnemyMoveInfoState.multiplier` | multi-hit multiplier |
| `isMultiDamage` | modeled | `EnemyMoveInfoState.is_multi_damage` | multi-hit flag |

## Field Ledger: `rooms/AbstractRoom.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `uiStrings`, `TEXT`, `logger` | non_combat | none | localization/logging |
| `potions` | run_level_materialized | future run reward kernel | room reward/drop list, not combat step |
| `relics` | run_level_materialized | future run reward kernel | room reward/drop list |
| `rewards` | non_combat | none | post-combat reward screen excluded from `CombatKernel` |
| `souls` | render_only | none | card animation group |
| `phase` | modeled | `RoomCombatState.phase` | room combat lifecycle |
| `event` | non_combat | none | event dialog outside combat kernel |
| `monsters` | modeled | `RoomCombatState.monster_group_ref`, `MonsterGroupState` | active monster group |
| `endBattleTimer` | modeled | `RoomCombatState.combat_end_timer_state` | battle-end timing state |
| `rewardPopOutTimer` | run_level_materialized | `RoomCombatState.reward_pop_out_timer_bits` | reward transition guard; combat kernel must stop before reward RNG |
| `END_TURN_WAIT_DURATION` | derived | none | constant for turn transition |
| `mapSymbol` | run_level_materialized | `RoomCombatState.map_symbol` | room identity/context |
| `mapImg`, `mapImgOutline` | render_only | none | map rendering |
| `isBattleOver` | modeled | `RoomCombatState.is_battle_over` | combat terminal lifecycle |
| `cannotLose` | modeled | `RoomCombatState.cannot_lose` | loss prevention |
| `eliteTrigger` | modeled | `RoomCombatState.elite_trigger` | elite/relic context |
| `blizzardPotionMod` | run_level_materialized | `RoomCombatState.blizzard_potion_mod` | potion reward modifier, guard at terminal boundary |
| `BLIZZARD_POTION_MOD_AMT` | derived | none | constant |
| `mugged` | modeled | `RoomCombatState.mugged` | stolen gold/reward behavior |
| `smoked` | modeled | `RoomCombatState.smoked` | Smoke Bomb escape behavior |
| `combatEvent` | modeled | `RoomCombatState.combat_event` | event combat flag |
| `rewardAllowed` | modeled | `RoomCombatState.reward_allowed` | terminal/reward boundary |
| `rewardTime` | modeled | `RoomCombatState.reward_time` | terminal/reward boundary |
| `skipMonsterTurn` | modeled | `RoomCombatState.skip_monster_turn` | turn processing |
| `baseRareCardChance`, `baseUncommonCardChance`, `rareCardChance`, `uncommonCardChance` | run_level_materialized | `RoomCombatState.*card_chance` | card reward probabilities; model only to guard combat-end transition |
| `waitTimer` | modeled | `RoomCombatState.wait_timer_bits` | room update wait gate |

## Field Ledger: `powers/AbstractPower.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging |
| `atlas`, `region48`, `region128`, `RAW_W`, font constants, colors, effects, `img` | render_only | none | power rendering/VFX |
| `fontScale` | render_only | none | UI animation |
| `owner` | modeled | `PowerInstance.owner_ref` | power owner |
| `name` | modeled | `PowerInstance.name_id` | public identity/display |
| `description` | modeled | `PowerInstance.description_id` | public identity/display |
| `ID` | modeled | `PowerInstance.power_id` | power identity |
| `amount` | modeled | `PowerInstance.amount` | stack value |
| `priority` | modeled | `PowerInstance.priority` | hook ordering |
| `type` | modeled | `PowerInstance.power_type` | buff/debuff/neutral |
| `isTurnBased` | modeled | `PowerInstance.is_turn_based` | duration behavior |
| `isPostActionPower` | modeled | `PowerInstance.is_post_action_power` | hook timing |
| `canGoNegative` | modeled | `PowerInstance.can_go_negative` | stack behavior |
| `DESCRIPTIONS` | non_combat | none | localization |

## Field Ledger: `relics/AbstractRelic.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `tutorialStrings`, `MSG`, `LABEL`, `USED_UP_MSG` | non_combat | none | localization/tutorial |
| `name` | modeled | `RelicInstance.name_id` | public identity/display |
| `relicId` | modeled | `RelicInstance.relic_id` | relic identity |
| `relicStrings`, `DESCRIPTIONS`, `description`, `flavorText`, `tips` | modeled/render_only split | `RelicInstance.description_id`, payload if needed | description public; tips render-only |
| `energyBased` | modeled | `RelicInstance.energy_based` | energy relic behavior |
| `usedUp` | modeled | `RelicInstance.used_up` | disabled/used state |
| `grayscale` | modeled | `RelicInstance.grayscale` | used-up visible state |
| `cost` | run_level_materialized | `RelicInstance.cost` | shop/run value, preserved on instance |
| `counter` | modeled | `RelicInstance.counter` | relic counter |
| `tier` | run_level_materialized | `RelicInstance.tier` | reward/shop context; preserved |
| images, img paths, page/position/scale/color/pulse/animation/glow/flash/hitbox/rotation/floaty fields | render_only | none | rendering/UI |
| `isSeen` | non_combat | `RelicInstance.is_seen` | compendium/unlock state; retained for parity |
| `isDone` | run_level_materialized | `RelicInstance.is_done` | room pickup/update state |
| `isAnimating` | render_only | `RelicInstance.is_animating` if a pickup path needs it | normally UI |
| `isObtained` | run_level_materialized | `RelicInstance.is_obtained` | room pickup/update state |
| `landingSFX` | render_only | none | audio |
| `discarded` | modeled | `RelicInstance.discarded` | relic removal/discard state |
| `assetURL` | render_only | none | asset path |

## Field Ledger: `potions/AbstractPotion.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `uiStrings`, `TEXT` | non_combat | none | localization |
| `ID` | modeled | `PotionInstance.potion_id` | potion identity |
| `name` | modeled | `PotionInstance.name_id` | public identity/display |
| `description` | modeled | `PotionInstance.description_id` | public identity/display |
| `slot` | modeled | `PotionInstance.slot` | potion slot |
| `tips` | render_only | none | UI tips |
| texture/color rendering fields, position, scale, flash/sparkle timers, hitbox, angle, placeholder color | render_only | none | potion rendering/UI |
| `isObtained` | run_level_materialized | `PotionInstance.is_obtained` | pickup state |
| `p_effect` | modeled | `PotionInstance.effect` | potion effect kind |
| `color` | modeled | `PotionInstance.color` | potion color kind |
| `rarity` | run_level_materialized | `PotionInstance.rarity` | generation/shop context |
| `size` | modeled | `PotionInstance.size` | potion representation |
| `potency` | modeled | `PotionInstance.potency` | potion amount |
| `canUse` | modeled | `PotionInstance.can_use` | legal use state |
| `discarded` | modeled | `PotionInstance.discarded` | potion discarded |
| `isThrown` | modeled | `PotionInstance.is_thrown` | thrown/used state |
| `targetRequired` | modeled | `PotionInstance.target_required` | target legality |

## Field Ledger: `orbs/AbstractOrb.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `name` | modeled | `OrbInstance.name_id` | public identity/display |
| `description` | modeled | `OrbInstance.description_id` | public identity/display |
| `ID` | modeled | `OrbInstance.orb_id` | orb identity |
| `tips`, position/target/color/texture/bob/hitbox/angle/scale/font constants | render_only | none | rendering/UI |
| `evokeAmount` | modeled | `OrbInstance.evoke_amount` | evoke value |
| `passiveAmount` | modeled | `OrbInstance.passive_amount` | passive value |
| `baseEvokeAmount` | modeled | `OrbInstance.base_evoke_amount` | base evoke value |
| `basePassiveAmount` | modeled | `OrbInstance.base_passive_amount` | base passive value |
| `showEvokeValue` | modeled | `OrbInstance.show_evoke_value` | card hover can toggle this before render |
| `channelAnimTimer` | render_only | `OrbInstance.channel_anim_timer_bits` if parity requires | channel animation; currently retained as raw bits |

## Field Ledger: `stances/AbstractStance.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| `logger` | non_combat | none | logging |
| `name` | modeled | `StanceState.name_id` | public identity/display |
| `description` | modeled | `StanceState.description_id` | public identity/display |
| `ID` | modeled | `StanceState.stance_id` | stance identity |
| `tips`, color, image, angle | render_only | none | rendering/UI |
| `particleTimer`, `particleTimer2` | render_only | `StanceState.*particle_timer*_bits` if parity requires | VFX timers retained as raw bits for audit |

## Field Ledger: `screens/select/GridCardSelectScreen.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| localization/layout/static scroll fields | render_only | none | UI layout |
| `selectedCards` | modeled | `GridSelectState.selected_card_refs` | partial selection state |
| `targetGroup` | modeled | `GridSelectState.target_group_zone_ref` | candidate group |
| `hoveredCard` | render_only | `GridSelectState.hovered_card_ref` if input replay needs it | UI hover |
| `upgradePreviewCard` | modeled | `GridSelectState.upgrade_preview_card_ref` | upgrade/transform confirmation preview |
| `numCards` | modeled | `GridSelectState.num_cards` | required selection count |
| `cardSelectAmount` | modeled | `GridSelectState.card_select_amount` | selected count/progress |
| scroll/grab/controller/arrow/timer fields | render_only | none | UI only |
| `canCancel` | modeled | `GridSelectState.can_cancel` | cancellation legality |
| `forUpgrade` | modeled | `GridSelectState.for_upgrade` | selection semantics |
| `forTransform` | modeled | `GridSelectState.for_transform` | selection semantics |
| `forPurge` | modeled | `GridSelectState.for_purge` | selection semantics |
| `confirmScreenUp` | modeled | `GridSelectState.confirm_screen_up` | multi-step state |
| `isJustForConfirming` | modeled | `GridSelectState.is_just_for_confirming` | confirm-only state |
| `confirmButton`, `peekButton` | render_only | none | UI widgets |
| `tipMsg`, `lastTip` | modeled | `GridSelectState.tip_msg`, `last_tip` | selection screen public prompt |
| `ritualAnimTimer` | render_only | none | event animation |
| `prevDeckSize` | modeled | `GridSelectState.prev_deck_size` | grid screen state |
| `cancelWasOn` | modeled | `GridSelectState.cancel_was_on` | cancel restore state |
| `anyNumber` | modeled | `GridSelectState.any_number` | choice constraint |
| `forClarity` | modeled | `GridSelectState.for_clarity` | choice constraint |
| `cancelText` | modeled | `GridSelectState.cancel_text` | public cancel prompt |

## Field Ledger: `screens/select/HandCardSelectScreen.java`

| Field | Classification | Schema path | Notes |
| --- | --- | --- | --- |
| localization/layout/static hover/arrow fields | render_only | none | UI layout |
| `numCardsToSelect` | modeled | `HandSelectState.num_cards_to_select` | choice constraint |
| `selectedCards` | modeled | `HandSelectState.selected_card_refs` | partial selection state |
| `hoveredCard` | render_only | `HandSelectState.hovered_card_ref` if input replay needs it | UI hover |
| `upgradePreviewCard` | modeled | `HandSelectState.upgrade_preview_card_ref` | upgrade/transform confirmation preview |
| `selectionReason` | modeled | `HandSelectState.selection_reason` | choice context |
| `wereCardsRetrieved` | modeled | `HandSelectState.were_cards_retrieved` | retrieval state |
| `canPickZero` | modeled | `HandSelectState.can_pick_zero` | choice constraint |
| `upTo` | modeled | `HandSelectState.up_to` | choice constraint |
| `message` | modeled | `HandSelectState.message` | public prompt |
| `button`, `peekButton` | render_only | none | UI widgets |
| `anyNumber` | modeled | `HandSelectState.any_number` | choice constraint |
| `forTransform` | modeled | `HandSelectState.for_transform` | selection semantics |
| `forUpgrade` | modeled | `HandSelectState.for_upgrade` | selection semantics |
| `numSelected` | modeled | `HandSelectState.num_selected` | selected count/progress |
| `waitThenClose` | modeled | `HandSelectState.wait_then_close_if_mechanical` | delayed close affects transform timing |
| `waitToCloseTimer` | modeled | `HandSelectState.wait_to_close_timer_bits` | delayed close timer raw bits |
| `hand` | modeled | `HandSelectState.hand_zone_ref` | source hand group |

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
