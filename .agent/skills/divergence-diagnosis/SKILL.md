---
name: divergence-diagnosis
description: Methodology and patterns for diagnosing sts_sim divergences from shadow verification. Consult when investigating why Rust simulation results differ from Java CommunicationMod logs.
---

# Divergence Diagnosis Skill

## Tool: `sim_diag.py`

Location: `c:\Dev\rust\bottled_ai_fresh\sim_diag.py`

| Command | Purpose |
|---------|---------|
| `summary` | Quick accuracy numbers |
| `report` | Full report, filterable by `--field`, `--combat`, `--file` |
| `inspect <step>` | Deep state dump for one step |
| `context <step>` | Raw JSONL context (`--hand`, `--piles`, `--enemies`, `--all`) |

Always use `--out file.txt` or `> file.txt 2>$null` to avoid garbled terminal output.

---

## Root Cause Pattern Library

### Pattern 1: Cascading Δ=+N or Δ=-N

**Symptom**: Consecutive steps all have the same delta (e.g., Δ=+1 across 10 steps)

**Diagnosis**: One event is missing, and all subsequent steps inherit the error.

**Common Causes**:
- Missing `on_use_card` power hook (Pain, Hex, Beat of Death)
- Missing `on_attacked` hook (Thorns, Flame Barrier, Caltrops)
- Card not applying self-damage (Hemokinesis, Offering, Bloodletting)

**How to Find**: Look at the **first** divergent step. Check player & enemy powers for effects that trigger per-card-play or per-attack.

---

### Pattern 2: enemy_hp Δ=-1 (Rust deals 1 MORE damage)

**Symptom**: Single-step enemy HP off by 1

**Common Causes**:
- Vulnerable rounding: Java `floor(damage * 1.5)`, check if Rust rounds differently
- Flight halving: Java `floor(damage / 2)` per hit, Flight stacks decrement per hit
- Strength applied incorrectly when negative (Weak state)

**How to Verify**: Check `calculate_card_damage()` output vs manual calculation using before-state powers.

---

### Pattern 3: enemy_block Δ=-20 (Guardian)

**Symptom**: Guardian shows 0 block but Java shows 20

**Root Cause**: Guardian Mode Shift threshold reached → should enter Defensive Mode → gain 20 block. Rust triggers mode switch but doesn't apply the block.

---

### Pattern 4: enemy_powers.Mode Shift Δ=+N (Guardian)

**Symptom**: Mode Shift power not removed after mode switch

**Root Cause**: When threshold is exceeded, Java removes Mode Shift power entirely. Rust keeps it.

---

### Pattern 5: Card override missing hooks

**Symptom**: player_hp or enemy_hp wrong when using overridden cards (Perfected Strike, Feed, Dropkick, Blizzard)

**Root Cause**: Card overrides in `card_overrides.rs` bypass the normal `DealDamage` command handler. If the override doesn't call `fire_on_attacked_hooks()`, reflect damage (Thorns etc.) won't fire.

**Fix Template**: After `take_damage_from_player()`, add:
```rust
fire_on_attacked_hooks(state, target_enemy, actual);
```

---

### Pattern 6: player_block wrong (Second Wind, etc.)

**Symptom**: Block gained is different from expected

**Check**: Does the card exhaust/count cards correctly? Second Wind exhausts non-Attack cards and gains block per card. Verify what Rust considers "non-Attack" vs Java.

---

### Pattern 7: Curl Up timing (AoE vs single-target)

**Symptom**: Curl Up consumed on wrong step

**Root Cause**: Curl Up triggers on "first attack damage received". For AoE attacks, Java processes enemies in order; the trigger timing can differ from Rust.

---

### Pattern 8: Power not decaying / decaying too fast

**Symptom**: `player_powers.X` expected 0 but actual N, or vice versa

**Common Causes**:
- `just_applied` flag not hydrated from JSONL → debuff decays one turn early
- Power type mismatch: buff vs debuff → different decay behavior
- Missing `atTurnStart` hook for powers that decrement (DuplicationPower/Echo Form)

---

## Diagnostic Checklist

When investigating a divergent step:

1. ✅ **What card?** — Check if it has a card override (bypasses JSON commands)
2. ✅ **Card type?** — Attack/Skill/Power affects hook interactions (Corruption, Burst, etc.)
3. ✅ **Player powers?** — Pain, Hex, Corruption, IntangiblePlayer, FlameBarrier
4. ✅ **Enemy powers?** — Thorns, CurlUp, Flight, ModeShift, Sharp Hide, Artifact
5. ✅ **Is this the FIRST divergence?** — If no, trace back to the cascade source
6. ✅ **Is the .pyd fresh?** — Check file timestamp, verify it's in `bottled_ai_fresh/` not just site-packages
7. ✅ **DealDamage vs DealDamageAll vs MultiHit?** — Each handler has different hook coverage

---

## Improvement Log

- **2026-03-05**: Created skill. Initial gaps identified.
- **2026-03-05**: `.pyd` stale copy pitfall — use `/build-deploy` workflow.
- **2026-03-05**: `LoseHP` vs `DamageAction` distinction from Pain fix.
- **2026-03-05**: Shuriken relic counter hydration fixed (13 cascading divergences).
- **2026-03-05**: Created `timing_known.rs` whitelist module for ActionQueue timing artifacts.
- **2026-03-05**: DuplicationPower alias fix — `PowerId::from_str` didn't match CommunicationMod ID.
- **2026-03-05**: Second Wind card_override — JSON can't express per-card block.
- **2026-03-05**: **100% accuracy achieved** (1707/1707 steps, 0 divergences).

---

## Timing Whitelist Module

**Location**: `src/testing/timing_known.rs`

When a divergence is a Java ActionQueue timing artifact (Rust is correct, Java snapshot is intermediate), add a `TimingRule` to `timing_rules()`.

**Current rules**: `curl_up_deferred_removal`, `curl_up_deferred_block`, `malleable_deferred`, `deferred_death_trigger`

**Design**: Each rule has `id`, `reason`, and a `matches` closure that checks the divergence field + before/expected/actual snapshots. Rules are conservative — when in doubt, report rather than filter.

### PowerId Alias Pitfall

CommunicationMod sends IDs like `"DuplicationPower"`, but `PowerId::from_str()` may only match `"Duplication"`. Always add Java-style aliases in `from_str()`.

### JSON Command Limitation: Per-Card Effects

Cards like Second Wind, Fiend Fire that iterate hand and apply effects per card need `card_overrides.rs` overrides. JSON's sequential commands (`ExhaustCards → GainBlock`) give flat block, not per-card.

## Platform Notes

- Windows uses `C:\tmp` not `/tmp` for temporary scripts.
- Always pipe sim_diag output to file: `> C:\tmp\out.txt 2>$null`
- `cargo build` → `.dll` → copy as `.pyd`. Use `/build-deploy` workflow.
