# Potion Source Audit

This audit records the current Java-source parity pass for combat potion use.
The intent is not to make potion logic a strategy surface; potion behavior is
mechanical simulator truth and must stay anchored to the Java source under
`D:/rust/cardcrawl`.

## Source Files Checked

- `D:/rust/cardcrawl/potions/AbstractPotion.java`
- `D:/rust/cardcrawl/potions/*Potion.java`
- `D:/rust/cardcrawl/ui/panels/PotionPopUp.java`
- `D:/rust/cardcrawl/helpers/PotionHelper.java`
- `D:/rust/cardcrawl/actions/unique/DiscoveryAction.java`
- `D:/rust/cardcrawl/actions/defect/EssenceOfDarknessAction.java`
- `D:/rust/cardcrawl/actions/utility/RandomizeHandCostAction.java`
- Rust paths:
  - `src/content/potions/mod.rs`
  - `src/content/potions/potion_effects.rs`
  - `src/engine/action_handlers/cards.rs`
  - `src/engine/core.rs`
  - `src/bot/combat/legal_moves.rs`

## Shared Java Rules

- `PotionPopUp` applies `potion.use(target)`, then relic `onUsePotion`, then
  destroys the potion slot. Rust combat use now keeps this ordering: potion
  actions first, relic hooks second, slot clear after successful use.
- `AbstractPotion.canUse()` blocks most potion use outside active combat, after
  the turn has ended, when monsters are basically dead, and in `WeMeetAgain`.
  Some potion classes override this. `BloodPotion`, `FruitJuice`, and
  `EntropicBrew` can be manually used outside combat unless `WeMeetAgain` is
  active; `FairyPotion` cannot be manually used.
- `AbstractPotion.canDiscard()` is true except while the current room event is
  `WeMeetAgain`. Java `PotionPopUp` uses this for top-panel discard, so this is
  a run-level potion lifetime rule rather than combat strategy.
- `AbstractPotion.getPotency()` doubles potency when `SacredBark` is owned.
  Rust combat use resolves effective potency at use time.
- `PotionHelper.getPotions(class, getAll)` order is source-parity critical.
  Rust `potions_for_class()` preserves class-specific potion order followed by
  shared potion order.

## Fixed In This Pass

| Potion / path | Java source behavior | Rust fix | Tests |
| --- | --- | --- | --- |
| Entropic Brew | Queues one `ObtainPotionAction(returnRandomPotion(true))` per potion slot while the potion is used. Concrete potion IDs are generated before queued obtains run; Fruit Juice is excluded. | `handle_use_potion` now handles Entropic Brew statefully with `potion_rng`, limited potion generation, queued concrete obtains, relic hooks, then slot clear. | `entropic_brew_generates_concrete_limited_potions_before_obtain_actions` |
| Attack / Skill / Power / Colorless Potion with Sacred Bark | `DiscoveryAction(type, amount)` / `DiscoveryAction(colorless, amount)` uses `amount`; selected card can create two stat-equivalent copies with Sacred Bark. | `SuspendForDiscovery` and `DiscoveryChoiceState` now carry `amount`; resolution creates that many copies, preserving Java hand-capacity split between hand and discard. | `sacred_bark_discovery_potion_adds_two_selected_copies_with_java_hand_capacity_split` |
| Distilled Chaos | Queues `PlayTopCardAction(getRandomMonster(... cardRandomRng), false)` once per potency. Random targets are rolled at potion-use time, before top cards execute. | `handle_use_potion` now rolls random targets immediately and queues targeted `PlayTopCard` actions. | `distilled_chaos_rolls_random_targets_when_potion_is_used` |
| Essence of Darkness | `EssenceOfDarknessAction` loops current orb slots, then channels `potency` Dark orbs per slot. Sacred Bark with 3 slots channels 6 Dark orbs. | `handle_use_potion` now expands channels as `orb_slots * potency` rather than only `potency`. | `essence_of_darkness_channels_for_each_orb_slot_and_sacred_bark_potency` |
| Liquid Memories | `BetterDiscardPileToHandAction(number, 0)` auto-moves when discard size is `<= number`; if hand fills, remaining discard cards stay in discard. Empty discard still consumes the potion and no-ops. | Immediate path now checks hand capacity before removing each discard card and leaves overflow cards in discard. | `liquid_memories_auto_move_does_not_drop_cards_when_hand_fills`; `engine_fizzles_liquid_memories_empty_discard_after_consuming_potion` |
| Snecko Oil | Queues draw, then `RandomizeHandCostAction`; that action skips `cost < 0`, rolls 0-3 per eligible card, and changes both `cost` and `costForTurn` when different. | `handle_randomize_hand_costs` now reads current combat cost, skips X/unplayable cost, and mutates combat plus turn cost. | `snecko_oil_randomize_updates_combat_cost_and_turn_cost_like_java` |
| Smoke Bomb | `SmokeBomb.canUse()` rejects if any monster has `BackAttack` or `EnemyType.BOSS`; it is not just a room-level boss flag. | `handle_use_potion` and `engine_local_moves` now block by room boss flag, visible boss monster type, and `BackAttack`. | `smoke_bomb_is_blocked_by_spire_shield_back_attack_power`; `smoke_bomb_is_blocked_by_boss_monster_type_even_without_room_flag`; `engine_local_moves_skip_smoke_bomb_when_visible_monster_is_boss` |
| Run observation `canUse` / `canDiscard` | Non-combat top-panel affordances are dynamic: only Blood/Fruit/Entropic override non-combat use, `FairyPotion` is passive, and `WeMeetAgain` blocks both use and discard. During combat, potion slots live in combat state, not stale run state. | `build_potion_observations` now reads combat slots when combat is active and uses source-backed non-combat affordance helpers for run-state slots. | `non_combat_potion_observation_uses_java_can_use_overrides`; `we_meet_again_blocks_potion_use_and_discard_observation`; `combat_potion_observation_uses_combat_slots_not_stale_run_slots` |

## Short-Term Clean Areas

- Metadata for all 42 potions exists in `PotionId` / `PotionDefinition`.
- Character-specific potion pool order matches Java `PotionHelper` ordering for
  Ironclad, Silent, Defect, Watcher, and all-class mode.
- Straight combat action potions are represented as mechanical actions:
  damage, block, energy, draw, powers, stance, orb slot increase, generated
  cards, hand/grid choices, and flee.
- `FairyPotion` is not emitted as a manual legal move and remains passive.
- `Sozu` blocks potion obtain paths through `obtain_specific_potion_if_allowed`.
- Full-run observation no longer treats `Potion::can_use` / `can_discard` as
  context-free truth: combat slots come from `CombatState`, and non-combat
  affordances account for Blood/Fruit/Entropic overrides and `WeMeetAgain`.

## Boundaries Still Not Closed

- Out-of-combat potion execution is not treated as closed here. The observation
  layer exposes Java-like use/discard affordances, but `ClientInput::UsePotion`
  and `ClientInput::DiscardPotion` outside combat still need a source-backed
  run-level handler before policy can actually take those actions.
- `EntropicBrew` outside combat uses Java's non-limited
  `returnRandomPotion()` path and effect timing around slot destruction. That
  should be implemented deliberately with run-level RNG/slot tests, not folded
  into combat `Action::UsePotion`.
- The passive death-prevention path for `FairyPotion` belongs to revive/death
  handling, not `Action::UsePotion`; it should remain audited with death hooks.
- Potion reward/drop generation is partly covered by relic/run audits, but it is
  not the same thing as combat potion use.
- UI-only effects, sounds, cursor movement, hitbox movement, and visual potion
  flags are intentionally not ported unless they host mechanical state.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `999 passed`.
