# Java ↔ Rust Cross-Validation Report

Uses **PowerId-based reverse matching**: no manual hook name mapping needed.

## 1. Per-Power Hook Coverage

For each power: how many Java hooks vs Rust dispatch functions?

### ❌ No Rust Dispatch (14 powers)

| Power | Java Hooks | Rust Functions |
|-------|-----------|----------------|
| `Artifact` | `onSpecificTrigger` | (none) |
| `Buffer` | `onAttackedToChangeDamage` | (none) |
| `Confusion` | `onCardDraw` | (none) |
| `Demon Form` | `atStartOfTurnPostDraw` | (none) |
| `Flex` | `atEndOfTurn` | (none) |
| `Focus` | `reducePower` | (none) |
| `Invincible` | `onAttackedToChangeDamage`, `atStartOfTurn` | (none) |
| `Magnetism` | `atStartOfTurn` | (none) |
| `Mayhem` | `atStartOfTurn` | (none) |
| `No Draw` | `atEndOfTurn` | (none) |
| `Panache` | `onUseCard`, `atStartOfTurn` | (none) |
| `Poison` | `atStartOfTurn` | (none) |
| `Sadistic` | `onApplyPower` | (none) |
| `TheBomb` | `atEndOfTurn` | (none) |

### ⚠️ Partial Coverage (9 powers)

Java has more hooks than Rust has dispatch functions for this PowerId.

**`Double Tap`** → `DoubleTap` (1/2)
  - Java hooks: `onUseCard`, `atEndOfTurn`
  - Rust fns: `resolve_power_on_use_card`

**`DuplicationPower`** → `DuplicationPower` (1/2)
  - Java hooks: `onUseCard`, `atEndOfRound`
  - Rust fns: `resolve_power_on_use_card`

**`Flight`** → `Flight` (2/4)
  - Java hooks: `atStartOfTurn`, `atDamageFinalReceive`, `onAttacked`, `onRemove`
  - Rust fns: `resolve_power_on_attacked`, `resolve_power_on_calculate_damage_from_player`

**`Intangible`** → `Intangible` (1/2)
  - Java hooks: `atDamageFinalReceive`, `atEndOfTurn`
  - Rust fns: `resolve_power_at_end_of_turn`

**`Malleable`** → `Malleable` (2/3)
  - Java hooks: `atEndOfTurn`, `atEndOfRound`, `onAttacked`
  - Rust fns: `resolve_power_at_end_of_turn`, `resolve_power_on_attacked`

**`Rage`** → `Rage` (1/2)
  - Java hooks: `onUseCard`, `atEndOfTurn`
  - Rust fns: `resolve_power_at_turn_start`

**`Ritual`** → `Ritual` (1/2)
  - Java hooks: `atEndOfTurn`, `atEndOfRound`
  - Rust fns: `resolve_power_at_end_of_turn`

**`Slow`** → `Slow` (2/3)
  - Java hooks: `atEndOfRound`, `onAfterUseCard`, `atDamageReceive`
  - Rust fns: `resolve_power_at_end_of_turn`, `resolve_power_on_calculate_damage_from_player`

**`Weakened`** → `Weak` (1/2)
  - Java hooks: `atEndOfRound`, `atDamageGive`
  - Rust fns: `resolve_power_on_calculate_damage_to_enemy`


### ✅ Full Coverage (38 powers)

| Power | Java Hooks | Rust Functions |
|-------|-----------|----------------|
| `Anger` | `onUseCard` | `resolve_power_on_card_played` |
| `Angry` | `onAttacked` | `resolve_power_on_attacked` |
| `BeatOfDeath` | `onAfterUseCard` | `resolve_power_on_player_card_played` |
| `Berserk` | `atStartOfTurn` | `resolve_power_at_turn_start` |
| `Brutality` | `atStartOfTurnPostDraw` | `resolve_power_on_post_draw` |
| `Combust` | `atEndOfTurn` | `resolve_power_at_end_of_turn` |
| `Compulsive` | `onAttacked` | `resolve_power_on_attacked` |
| `Constricted` | `atEndOfTurn` | `resolve_power_at_end_of_turn` |
| `Corruption` | `onCardDraw`, `onUseCard` | `resolve_power_on_apply`, `resolve_power_on_card_draw`, `resolve_power_on_use_card` |
| `Curiosity` | `onUseCard` | `resolve_power_on_player_card_played` |
| `Curl Up` | `onAttacked` | `resolve_power_on_attacked` |
| `Dark Embrace` | `onExhaust` | `resolve_power_on_exhaust` |
| `Dexterity` | `reducePower` | `resolve_power_on_calculate_block` |
| `Entangled` | `atEndOfTurn` | `resolve_power_at_end_of_turn` |
| `Evolve` | `onCardDraw` | `resolve_power_on_card_drawn` |
| `Explosive` | `duringTurn` | `resolve_power_at_end_of_turn` |
| `Fading` | `duringTurn` | `resolve_power_at_end_of_turn` |
| `Feel No Pain` | `onExhaust` | `resolve_power_on_exhaust` |
| `Fire Breathing` | `onCardDraw` | `resolve_power_on_card_drawn` |
| `Flame Barrier` | `onAttacked`, `atStartOfTurn` | `resolve_power_at_turn_start`, `resolve_power_on_attacked` |
| `Frail` | `atEndOfRound` | `resolve_power_on_calculate_block` |
| `Hex` | `onUseCard` | `resolve_power_on_card_played` |
| `Juggernaut` | `onGainedBlock` | `resolve_power_on_block_gained` |
| `Metallicize` | `atEndOfTurnPreEndTurnCards` | `resolve_power_at_end_of_turn` |
| `Next Turn Block` | `atStartOfTurn` | `resolve_power_at_turn_start` |
| `Painful Stabs` | `onInflictDamage` | `resolve_power_on_attack` |
| `Plated Armor` | `wasHPLost`, `onRemove`, `atEndOfTurnPreEndTurnCards` | `resolve_power_at_end_of_turn`, `resolve_power_on_attacked`, `resolve_power_on_remove` |
| `Regeneration` | `atEndOfTurn` | `resolve_power_at_end_of_turn` |
| `Rupture` | `wasHPLost` | `resolve_power_on_hp_lost` |
| `Sharp Hide` | `onUseCard` | `resolve_power_on_card_played` |
| ... | ... | (8 more) |

## 2. Inferred Hook Name Mapping

Auto-discovered by finding shared PowerIds between Java hooks and Rust functions.
Use this to update the fallback mapping table, or feed to an LLM for refinement.

| Java Hook | Best Rust Match | Confidence | Shared Powers |
|-----------|----------------|------------|---------------|
| `atDamageFinalReceive` | `resolve_power_at_end_of_turn` | 1/2 | Intangible |
| `atDamageFinalReceive` | `resolve_power_on_calculate_damage_from_player` | 1/2 | Flight |
| `atDamageGive` | `resolve_power_on_calculate_damage_to_enemy` | 3/3 | Strength, Vigor, Weak |
| `atDamageGive` | `resolve_power_on_attack_to_change_damage` | 1/3 | Strength |
| `atDamageReceive` | `resolve_power_on_calculate_damage_from_player` | 2/2 | Slow, Vulnerable |
| `atDamageReceive` | `resolve_power_on_attacked_to_change_damage` | 1/2 | Vulnerable |
| `atEndOfRound` | `resolve_power_at_end_of_turn` | 3/7 | Malleable, Ritual, Slow |
| `atEndOfRound` | `resolve_power_on_calculate_damage_from_player` | 2/7 | Slow, Vulnerable |
| `atEndOfTurn` | `resolve_power_at_end_of_turn` | 7/12 | Combust, Constricted, Entangle, Intangible (+3) |
| `atEndOfTurn` | `resolve_power_at_turn_start` | 1/12 | Rage |
| `atEndOfTurnPreEndTurnCards` | `resolve_power_at_end_of_turn` | 2/2 | Metallicize, PlatedArmor |
| `atEndOfTurnPreEndTurnCards` | `resolve_power_on_remove` | 1/2 | PlatedArmor |
| `atStartOfTurn` | `resolve_power_at_turn_start` | 3/9 | Berserk, FlameBarrier, NextTurnBlock |
| `atStartOfTurn` | `resolve_power_on_attacked` | 2/9 | FlameBarrier, Flight |
| `atStartOfTurnPostDraw` | `resolve_power_on_post_draw` | 1/2 | Brutality |
| `duringTurn` | `resolve_power_at_end_of_turn` | 2/2 | Explosive, Fading |
| `onAfterUseCard` | `resolve_power_on_player_card_played` | 2/3 | BeatOfDeath, TimeWarp |
| `onAfterUseCard` | `resolve_power_at_end_of_turn` | 1/3 | Slow |
| `onAttacked` | `resolve_power_on_attacked` | 8/8 | Angry, CurlUp, FlameBarrier, Flight (+4) |
| `onAttacked` | `resolve_power_at_end_of_turn` | 2/8 | Malleable, Shifting |
| `onCardDraw` | `resolve_power_on_card_drawn` | 2/4 | Evolve, FireBreathing |
| `onCardDraw` | `resolve_power_on_apply` | 1/4 | Corruption |
| `onDeath` | `resolve_power_on_death` | 2/2 | SporeCloud, Stasis |
| `onExhaust` | `resolve_power_on_exhaust` | 2/2 | DarkEmbrace, FeelNoPain |
| `onGainedBlock` | `resolve_power_on_block_gained` | 1/1 | Juggernaut |
| `onInflictDamage` | `resolve_power_on_attack` | 1/1 | PainfulStabs |
| `onRemove` | `resolve_power_on_attacked` | 2/2 | Flight, PlatedArmor |
| `onRemove` | `resolve_power_at_end_of_turn` | 1/2 | PlatedArmor |
| `onUseCard` | `resolve_power_on_use_card` | 4/10 | Corruption, DoubleTap, DuplicationPower, Vigor |
| `onUseCard` | `resolve_power_on_card_played` | 3/10 | Anger, Hex, SharpHide |
| `reducePower` | `resolve_power_on_attack_to_change_damage` | 1/3 | Strength |
| `reducePower` | `resolve_power_on_calculate_damage_to_enemy` | 1/3 | Strength |
| `wasHPLost` | `resolve_power_at_end_of_turn` | 1/2 | PlatedArmor |
| `wasHPLost` | `resolve_power_on_remove` | 1/2 | PlatedArmor |

## 3. Fallback Mapping Health

Status of the hardcoded `JAVA_HOOK_TO_RUST_FN` mapping table.

### ⚠️ Stale Entries (17) — likely renamed

| Java Hook | Mapped To (missing!) | Possible Replacements |
|-----------|---------------------|----------------------|
| `onPlayCard` | `resolve_power_on_play_card` | (none found) |
| `atDamageFinalGive` | `resolve_power_at_damage_final_give` | (none found) |
| `atDamageFinalReceive` | `resolve_power_at_damage_final_receive` | **`resolve_power_at_end_of_turn`** (inferred), (none found) |
| `onApplyPower` | `resolve_power_on_apply_power` | (none found) |
| `onAfterCardPlayed` | `resolve_power_on_after_card_played` | (none found) |
| `onHeal` | `resolve_power_on_heal` | (none found) |
| `onLoseHp` | `resolve_power_on_lose_hp` | (none found) |
| `onBlockBroken` | `resolve_power_on_block_broken` | (none found) |
| `onPlayerEndTurn` | `resolve_power_on_player_end_turn` | (none found) |
| `onPlayerGainBlock` | `resolve_power_on_player_gain_block` | (none found) |
| `onPlayerGainedBlock` | `resolve_power_on_player_gained_block` | (none found) |
| `onSpecificTrigger` | `resolve_power_on_specific_trigger` | (none found) |
| `onChannel` | `resolve_power_on_channel` | (none found) |
| `onEvokeOrb` | `resolve_power_on_evoke_orb` | (none found) |
| `onChangeStance` | `resolve_power_on_change_stance` | (none found) |
| `onInitialApplication` | `resolve_power_on_initial_application` | (none found) |
| `atBattleStart` | `resolve_power_at_battle_start` | (none found) |

### ✅ Valid Entries (18)

| Java Hook | Rust Function | Inferred Match? |
|-----------|--------------|------------------|
| `atDamageGive` | `resolve_power_on_calculate_damage_to_enemy` | ✅ confirmed (3 shared) |
| `atDamageReceive` | `resolve_power_on_calculate_damage_from_player` | ✅ confirmed (2 shared) |
| `atEndOfRound` | `resolve_power_at_end_of_turn` | ✅ confirmed (3 shared) |
| `atEndOfTurn` | `resolve_power_at_end_of_turn` | ✅ confirmed (7 shared) |
| `atEndOfTurnPreEndTurnCards` | `resolve_power_at_end_of_turn` | ✅ confirmed (2 shared) |
| `atStartOfTurn` | `resolve_power_at_turn_start` | ✅ confirmed (3 shared) |
| `atStartOfTurnPostDraw` | `resolve_power_on_post_draw` | ✅ confirmed (1 shared) |
| `duringTurn` | `resolve_power_at_end_of_turn` | ✅ confirmed (2 shared) |
| `onAfterUseCard` | `resolve_power_on_player_card_played` | ✅ confirmed (2 shared) |
| `onAttacked` | `resolve_power_on_attacked` | ✅ confirmed (8 shared) |
| `onAttackedToChangeDamage` | `resolve_power_on_attacked_to_change_damage` | ⚪ no Java powers use this hook |
| `onCardDraw` | `resolve_power_on_card_draw` | ✅ confirmed (1 shared) |
| `onDeath` | `resolve_power_on_death` | ✅ confirmed (2 shared) |
| `onExhaust` | `resolve_power_on_exhaust` | ✅ confirmed (2 shared) |
| `onGainedBlock` | `resolve_power_on_block_gained` | ✅ confirmed (1 shared) |
| `onInflictDamage` | `resolve_power_on_attack` | ✅ confirmed (1 shared) |
| `onRemove` | `resolve_power_on_remove` | ✅ confirmed (1 shared) |
| `onUseCard` | `resolve_power_on_use_card` | ✅ confirmed (4 shared) |

## 4. Private Fields → extra_data Coverage

Java powers with private fields that need `extra_data` or equivalent in Rust.

### ⚠️ Missing extra_data (12)

| Java Power | Rust Name | Fields Needed | Status |
|-----------|-----------|---------------|--------|
| `Curl Up` | `CurlUp` | `triggered` | NO extra_data usage — POTENTIAL BUG |
| `DevaForm` | `NOT IN RUST` | `energyGainAmount` | Not in Rust enum |
| `Echo Form` | `NOT IN RUST` | `cardsDoubledThisTurn` | Not in Rust enum |
| `Flight` | `Flight` | `storedAmount`, `calculateDamageTakenAmount` | NO extra_data usage — POTENTIAL BUG |
| `GrowthPower` | `NOT IN RUST` | `skipFirst` | Not in Rust enum |
| `Invincible` | `Invincible` | `maxAmt` | NO extra_data usage — POTENTIAL BUG |
| `Panache` | `Panache` | `damage` | NO extra_data usage — POTENTIAL BUG |
| `Rebound` | `NOT IN RUST` | `justEvoked` | Not in Rust enum |
| `RechargingCore` | `NOT IN RUST` | `turnTimer` | Not in Rust enum |
| `Ritual` | `Ritual` | `skipFirst`, `onPlayer` | NO extra_data usage — POTENTIAL BUG |
| `TheBomb` | `TheBomb` | `damage` | NO extra_data usage — POTENTIAL BUG |
| `TimeMazePower` | `NOT IN RUST` | `maxAmount` | Not in Rust enum |

### ✅ Handled (2)

| Java Power | Rust Name | Fields | Status |
|-----------|-----------|--------|--------|
| `Combust` | `Combust` | `hpLoss` | Uses extra_data ✅ |
| `Malleable` | `Malleable` | `basePower` | Uses extra_data ✅ |

## 5. Custom stackPower Coverage

| Java Power | Rust Name | Modifies | Status |
|-----------|-----------|----------|--------|
| `Collect` | `?` | [] | ⚠️ Has cap/remove logic — verify |
| `Dexterity` | `Dexterity` | [] | ⚠️ Has cap/remove logic — verify |
| `Energized` | `Energized` | [] | ⚠️ Has cap/remove logic — verify |
| `EnergizedBlue` | `?` | [] | ⚠️ Has cap/remove logic — verify |
| `Focus` | `Focus` | [] | ⚠️ Has cap/remove logic — verify |
| `LikeWaterPower` | `?` | [] | ⚠️ Has cap/remove logic — verify |
| `Malleable` | `Malleable` | ['basePower'] | ❌ Custom field modifications NOT handled |
| `Panache` | `Panache` | ['damage'] | ❌ Custom field modifications NOT handled |
| `Plated Armor` | `PlatedArmor` | [] | ⚠️ Has cap/remove logic — verify |
| `Shackled` | `?` | [] | ⚠️ Has cap/remove logic — verify |
| `Strength` | `Strength` | [] | ⚠️ Has cap/remove logic — verify |

## 6. Java Turn Lifecycle

Exact hook execution order per turn, traced from Java source.
Powers that rely on timing (e.g., `atEndOfRound` AFTER monsters act) are flagged.

### Phase: PLAYER_END_TURN (scope: player)

1. `atEndOfTurnPreEndTurnCards` → player_powers — player.applyEndOfTurnTriggers() → p.atEndOfTurnPreEndTurnCards(isPlayer)
2. `atEndOfTurn` → player_powers — player.applyEndOfTurnTriggers() → p.atEndOfTurn(isPlayer)

   **Powers active here**: `NoDraw`.atEndOfTurn, `Metallicize`.atEndOfTurnPreEndTurnCards, `Regen`.atEndOfTurn, `TheBomb`.atEndOfTurn, `Flex`.atEndOfTurn, `PlatedArmor`.atEndOfTurnPreEndTurnCards, `Combust`.atEndOfTurn, `DoubleTap`.atEndOfTurn, `Rage`.atEndOfTurn, `Intangible`.atEndOfTurn (+4 more)

### Phase: END_OF_TURN_ACTIONS (scope: player)

3. `onPlayerEndTurn` → relics — applyEndOfTurnRelics() → r.onPlayerEndTurn()
4. `atEndOfTurnPreEndTurnCards` → player_powers — applyEndOfTurnPreCardPowers() → p.atEndOfTurnPreEndTurnCards(true)
5. `triggerEndOfTurnOrbs` → orbs — TriggerEndOfTurnOrbsAction
6. `triggerOnEndOfTurnForPlayingCard` → hand_cards — hand cards trigger

   **Powers active here**: `Metallicize`.atEndOfTurnPreEndTurnCards, `PlatedArmor`.atEndOfTurnPreEndTurnCards

### Phase: DISCARD_AND_END (scope: system)

7. `DiscardAtEndOfTurnAction` → system — Discard remaining hand cards
8. `EndTurnAction` → system — Formally ends player turn

### Phase: MONSTER_TURNS (scope: monsters)

9. `takeTurn` → each_monster — m.takeTurn() — execute monster move
10. `duringTurn` → each_monster_powers — m.applyTurnPowers() → p.duringTurn()

   **Powers active here**: `Explosive`.duringTurn, `Fading`.duringTurn

### Phase: END_OF_ROUND (scope: all)

11. `atEndOfTurnPreEndTurnCards` → monster_powers — m.applyEndOfTurnTriggers() → p.atEndOfTurnPreEndTurnCards(false)
12. `atEndOfTurn` → monster_powers — m.applyEndOfTurnTriggers() → p.atEndOfTurn(false)
13. `atEndOfRound` → player_powers — MonsterGroup.applyEndOfTurnPowers() → player p.atEndOfRound()
14. `atEndOfRound` → monster_powers — MonsterGroup.applyEndOfTurnPowers() → each monster p.atEndOfRound()

   **Powers active here**: `NoDraw`.atEndOfTurn, `Slow`.atEndOfRound, `Weak`.atEndOfRound, `Metallicize`.atEndOfTurnPreEndTurnCards, `Vulnerable`.atEndOfRound, `Regen`.atEndOfTurn, `TheBomb`.atEndOfTurn, `Frail`.atEndOfRound, `Flex`.atEndOfTurn, `PlatedArmor`.atEndOfTurnPreEndTurnCards (+11 more)

### Phase: NEW_TURN_SETUP (scope: monsters)

15. `loseBlock` → monsters — MonsterGroup.applyPreTurnLogic() — monster block reset
16. `atStartOfTurn` → monster_powers — m.applyStartOfTurnPowers() → p.atStartOfTurn()

   **Powers active here**: `Poison`.atStartOfTurn, `Magnetism`.atStartOfTurn, `Mayhem`.atStartOfTurn, `Panache`.atStartOfTurn, `NextTurnBlock`.atStartOfTurn, `Berserk`.atStartOfTurn, `FlameBarrier`.atStartOfTurn, `Flight`.atStartOfTurn, `Invincible`.atStartOfTurn

### Phase: PLAYER_NEW_TURN (scope: player)

17. `loseBlock` → player — Player block reset (unless Barricade/Blur/Calipers)
18. `atStartOfTurn` → relics — player.applyStartOfTurnRelics()
19. `applyStartOfTurnPreDrawCards` → cards — innate cards etc.
20. `applyStartOfTurnCards` → cards — start-of-turn card triggers
21. `atStartOfTurn` → player_powers — player.applyStartOfTurnPowers() → p.atStartOfTurn()
22. `startOfTurnOrbs` → orbs — player.applyStartOfTurnOrbs()

   **Powers active here**: `Poison`.atStartOfTurn, `Magnetism`.atStartOfTurn, `Mayhem`.atStartOfTurn, `Panache`.atStartOfTurn, `NextTurnBlock`.atStartOfTurn, `Berserk`.atStartOfTurn, `FlameBarrier`.atStartOfTurn, `Flight`.atStartOfTurn, `Invincible`.atStartOfTurn

### Phase: POST_DRAW (scope: player)

23. `DrawCardAction` → system — Draw gameHandSize cards
24. `atStartOfTurnPostDraw` → relics — player.applyStartOfTurnPostDrawRelics()
25. `atStartOfTurnPostDraw` → player_powers — player.applyStartOfTurnPostDrawPowers() → p.atStartOfTurnPostDraw()

   **Powers active here**: `Brutality`.atStartOfTurnPostDraw, `DemonForm`.atStartOfTurnPostDraw

### ⚠️ Timing-Sensitive Powers

Powers where Rust may fire hooks at the wrong phase:

| Power | Warning |
|-------|--------|
| `Slow` | Only has `atEndOfRound` (Phase 5, after monsters). If Rust fires this at player end-of-turn, timing is WRONG. |
| `Weak` | Only has `atEndOfRound` (Phase 5, after monsters). If Rust fires this at player end-of-turn, timing is WRONG. |
| `Vulnerable` | Only has `atEndOfRound` (Phase 5, after monsters). If Rust fires this at player end-of-turn, timing is WRONG. |
| `Frail` | Only has `atEndOfRound` (Phase 5, after monsters). If Rust fires this at player end-of-turn, timing is WRONG. |
| `Malleable` | Has BOTH `atEndOfTurn` (Phase 1, player turn) AND `atEndOfRound` (Phase 5, after monsters). These are DIFFERENT timing windows. |
| `Ritual` | Has BOTH `atEndOfTurn` (Phase 1, player turn) AND `atEndOfRound` (Phase 5, after monsters). These are DIFFERENT timing windows. |
| `DuplicationPower` | Only has `atEndOfRound` (Phase 5, after monsters). If Rust fires this at player end-of-turn, timing is WRONG. |

## 7. Rust Implementation Summary

- **PowerId variants**: 74
- **Power dispatch functions**: 21
- **Card play dispatch arms**: 135
- **Powers using extra_data**: 2

### Dispatch functions:
- `resolve_power_at_end_of_turn`: 16 arms
- `resolve_power_at_turn_start`: 8 arms
- `resolve_power_on_apply`: 1 arms
- `resolve_power_on_attack`: 1 arms
- `resolve_power_on_attack_to_change_damage`: 1 arms
- `resolve_power_on_attacked`: 9 arms
- `resolve_power_on_attacked_to_change_damage`: 1 arms
- `resolve_power_on_block_gained`: 1 arms
- `resolve_power_on_calculate_block`: 2 arms
- `resolve_power_on_calculate_damage_from_player`: 3 arms
- `resolve_power_on_calculate_damage_to_enemy`: 5 arms
- `resolve_power_on_card_draw`: 1 arms
- `resolve_power_on_card_drawn`: 2 arms
- `resolve_power_on_card_played`: 4 arms
- `resolve_power_on_death`: 3 arms
- `resolve_power_on_exhaust`: 2 arms
- `resolve_power_on_hp_lost`: 4 arms
- `resolve_power_on_player_card_played`: 3 arms
- `resolve_power_on_post_draw`: 1 arms
- `resolve_power_on_remove`: 1 arms
- `resolve_power_on_use_card`: 5 arms
