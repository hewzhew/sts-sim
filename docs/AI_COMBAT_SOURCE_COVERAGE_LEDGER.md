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
