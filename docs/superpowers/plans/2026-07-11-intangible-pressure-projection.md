# Intangible Pressure Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make combat-search survival pressure use the simulator's per-hit Intangible cap without changing raw monster intent reporting.

**Architecture:** Keep global monster previews raw. Add one search-local helper in `pressure_value.rs` that reads exact `IntangiblePlayer` state and converts each visible attack to its effective hit-count damage before the existing HP-plus-block survival calculation.

**Tech Stack:** Rust, existing `CombatState`, `PowerId`, monster move projections, and library tests.

## Global Constraints

- Do not special-case Apparition, Time Eater, Runic Pyramid, or the seed.
- Do not change simulator damage resolution or global monster intent previews.
- Do not change run-control, owner HP policy, or public report schema.
- Add one mechanical regression test only.
- Do not generalize to Buffer or unrelated defensive powers.

---

### Task 1: Search-local Intangible pressure projection

**Files:**
- Modify: `src/ai/combat_search_v2/pressure_value.rs`

**Interfaces:**
- Consumes: `CombatState`, `PowerId::IntangiblePlayer`, and `project_monster_move_preview_in_combat`.
- Produces: unchanged `visible_incoming_damage(&CombatState) -> i32` and `combat_pressure_value(&CombatState) -> CombatPressureValueV1` signatures with corrected Intangible semantics.

- [ ] **Step 1: Write the failing mechanical test**

Extend the existing `pressure_value.rs` test module with a Time Eater
Reverberate fixture. Record raw pressure first, install one exact player
Intangible power, then assert the three-hit attack projects to three damage:

```rust
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::runtime::combat::{Power, PowerPayload};
use crate::test_support::planned_monster;

#[test]
fn pressure_value_caps_each_visible_attack_hit_with_player_intangible() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![planned_monster(EnemyId::TimeEater, 2)];
    let raw = combat_pressure_value(&combat);
    assert_eq!(raw.visible_incoming_damage, 21);

    let player_id = combat.entities.player.id;
    store::set_powers_for(
        &mut combat,
        player_id,
        vec![Power {
            power_type: PowerId::IntangiblePlayer,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let intangible = combat_pressure_value(&combat);
    assert_eq!(intangible.visible_incoming_damage, 3);
    assert_eq!(intangible.survival_margin, 80 - 3);
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib pressure_value_caps_each_visible_attack_hit_with_player_intangible -- --nocapture
```

Expected: FAIL because current search pressure remains the raw 21 damage after
the Intangible power is installed.

- [ ] **Step 3: Implement the minimal search-local projection**

Replace the raw-total mapping in `visible_incoming_damage` with an exact player
power check and per-monster preview mapping:

```rust
use crate::content::powers::PowerId;
use crate::sim::combat_projection::project_monster_move_preview_in_combat;

pub(super) fn visible_incoming_damage(combat: &CombatState) -> i32 {
    let player_intangible = combat.get_power(
        combat.entities.player.id,
        PowerId::IntangiblePlayer,
    ) > 0;
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            let preview = project_monster_move_preview_in_combat(combat, monster);
            if player_intangible && preview.total_damage.is_some() {
                i32::from(preview.hits)
            } else {
                preview.total_damage.unwrap_or(0)
            }
        })
        .sum()
}
```

Remove the now-unused `monster_preview_total_damage_in_combat` import from the
combat-search module root if the compiler reports it unused.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run the same focused command. Expected: one matching test passes with zero
failures.

- [ ] **Step 5: Commit the focused implementation**

```powershell
git add src/ai/combat_search_v2/pressure_value.rs src/ai/combat_search_v2/mod.rs
git commit -m "fix: project intangible combat pressure"
```

Only add `mod.rs` if its import changed.

### Task 2: Repository and frozen-case verification

**Files:**
- No additional source files.
- Generate: `target/bounded-mainline-20260711002/time-eater-intangible-pressure-review.json`

**Interfaces:**
- Consumes: the corrected search pressure projection from Task 1 and the saved Time Eater CombatCase.
- Produces: verification evidence only; no new policy or report fields.

- [ ] **Step 1: Run the full library suite**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib
```

Expected: all library tests pass with zero failures.

- [ ] **Step 2: Run architecture and hygiene checks**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --test architecture_runtime_boundaries
cargo fmt --all -- --check
git diff --check
```

Expected: seven architecture tests pass; formatting and whitespace checks exit
successfully.

- [ ] **Step 3: Rerun the frozen case with the owner-equivalent budget**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo run --quiet --bin combat_case_review -- `
  --case 'D:\rust\sts_simulator\target\bounded-mainline-20260711002\combat_cases\seed20260711002_g38_b0038_a3f48_timeeater.json' `
  --ladder --fast-nodes 1 --fast-ms 1 `
  --slow-nodes 800000 --slow-ms 10000 --compact `
  --write-review 'D:\rust\sts_simulator\target\bounded-mainline-20260711002\time-eater-intangible-pressure-review.json'
```

Expected: the review completes and writes the artifact. Record whether a
verified terminal win appears and whether the closest progress now survives
beyond the previous turn-one 431-HP loss. Do not add another policy exception
if the fight still fails.

- [ ] **Step 4: Merge locally and clean the worktree**

Fast-forward the verified implementation branch into `master`, rerun the
focused test on merged `master`, remove the project-owned worktree, and delete
the merged branch.
