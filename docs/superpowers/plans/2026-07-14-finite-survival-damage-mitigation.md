# Finite-Survival Damage-Mitigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make adaptive combat rollout recognize the semantic `Fading + Shifting` finite-survival mechanic and reuse the existing phase-aware rollout that solves the retained Transient case under the normal immediate-child budget.

**Architecture:** The private enemy-mechanics profile derives a name-independent combined fact from powers on living monsters and serializes it for audit. Adaptive rollout consumes only that fact to select `PhaseAwareNoPotion`; run-control lanes, terminal ownership, exact replay, and combat execution remain unchanged.

**Tech Stack:** Rust 2021, existing combat-search V2 profiles and rollout plugins, serde reports, Cargo library tests, retained combat captures.

## Global Constraints

- Work in `D:\rust\sts_simulator` on the existing feature branch; do not create a worktree.
- Never run `cargo clean`.
- Detect positive `Fading` plus the presence of `Shifting`; do not branch on `EnemyId::Transient`.
- Facts are read-only diagnostics and dispatch inputs: no pruning, terminal claims, state mutation, or run-control lane overrides.
- `Fading` alone and `Shifting` alone must retain conservative adaptive rollout.
- Adding serialized report fields increments `CombatSearchV2Report` schema version from 12 to 13.
- Keep the regression contract at the mechanism/dispatch boundary; use the retained capture only for acceptance, not to lock an action sequence in a unit test.
- Use focused red/green tests, then run the full library and `architecture_runtime_boundaries` suites.

---

## File Map

- Modify `src/ai/combat_search_v2/enemy_mechanics_profile.rs`: derive and report the combined finite-survival damage-mitigation facts.
- Modify `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`: lock combined and isolated-power behavior with a non-Transient enemy.
- Modify `src/ai/combat_search_v2/types/report/frontier.rs`: serialize the two new facts.
- Modify `src/ai/combat_search_v2/types/report/core.rs`: increment report schema version to 13.
- Modify `src/ai/combat_search_v2/search/tests.rs`: assert the concrete report schema version.
- Modify `src/ai/combat_search_v2/rollout_cache/policy.rs`: select phase-aware rollout for the combined mechanism.
- Modify `src/ai/combat_search_v2/rollout_cache/mod.rs`: lock adaptive dispatch and negative cases.
- Modify `src/ai/combat_search_v2/rollout_cache/report.rs`: describe the new dispatch in artifact notes.
- Modify `src/content/powers/core/shifting.rs`: replace the stale end-of-turn MVP comment with the actual `Shackled` ownership statement.

### Task 1: Expose the Combined Mechanism Fact

**Files:**
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile.rs`
- Modify: `src/ai/combat_search_v2/types/report/frontier.rs`
- Modify: `src/ai/combat_search_v2/types/report/core.rs`
- Modify: `src/ai/combat_search_v2/search/tests.rs`

**Interfaces:**
- Produces: `EnemyMechanicsProfileV1::finite_survival_damage_mitigation_target_count: usize`.
- Produces: `EnemyMechanicsProfileV1::finite_survival_damage_mitigation_min_owner_turns: Option<u32>`.
- Serializes both fields through `CombatSearchV2EnemyMechanicsReport`.

- [ ] **Step 1: Write the failing combined-mechanism profile test**

Add a non-Transient monster with `Fading(5)` and `Shifting(-1)` and assert both the internal and report values:

```rust
#[test]
fn finite_survival_damage_mitigation_profile_reads_powers_not_enemy_name() {
    let mut combat = blank_test_combat();
    let mut owner = test_monster(EnemyId::Cultist);
    owner.id = 7;
    combat.entities.monsters = vec![owner];
    combat.entities.power_db.insert(
        7,
        vec![
            Power {
                power_type: PowerId::Fading,
                instance_id: None,
                amount: 5,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            },
            Power {
                power_type: PowerId::Shifting,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            },
        ],
    );

    let profile = enemy_mechanics_profile(&combat);
    let report = enemy_mechanics_profile_report(profile);

    assert_eq!(profile.finite_survival_damage_mitigation_target_count, 1);
    assert_eq!(
        profile.finite_survival_damage_mitigation_min_owner_turns,
        Some(5)
    );
    assert_eq!(report.finite_survival_damage_mitigation_target_count, 1);
    assert_eq!(
        report.finite_survival_damage_mitigation_min_owner_turns,
        Some(5)
    );
}
```

- [ ] **Step 2: Write the failing isolation test**

Construct one Cultist with only `Fading(5)` and one with only `Shifting(-1)`; assert that each profile reports count zero and no minimum.

- [ ] **Step 3: Run the focused test and verify RED**

Run:

```powershell
cargo test --lib finite_survival_damage_mitigation_profile -- --nocapture
```

Expected: compilation fails because the new profile/report fields do not exist.

- [ ] **Step 4: Implement the minimal profile fields and derivation**

Add the two fields to `EnemyMechanicsProfileV1`.  Before the enemy-id match, derive the combined fact from the living monster's powers:

```rust
let fading_turns = store::power_amount(combat, monster.id, PowerId::Fading);
if fading_turns > 0 && store::has_power(combat, monster.id, PowerId::Shifting) {
    profile.finite_survival_damage_mitigation_target_count += 1;
    let fading_turns = fading_turns as u32;
    profile.finite_survival_damage_mitigation_min_owner_turns = Some(
        profile
            .finite_survival_damage_mitigation_min_owner_turns
            .map_or(fading_turns, |old| old.min(fading_turns)),
    );
}
```

Map both values into `CombatSearchV2EnemyMechanicsReport` and add matching public serialized fields.

- [ ] **Step 5: Increment and lock the report schema**

Change:

```rust
pub const COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION: u32 = 13;
```

Add `assert_eq!(COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION, 13);` beside the existing report-schema assertions.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```powershell
cargo test --lib finite_survival_damage_mitigation_profile -- --nocapture
cargo test --lib search_report_declares_privileged_policy_evidence_boundary -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 7: Commit the mechanism fact**

```powershell
git add -- src/ai/combat_search_v2/enemy_mechanics_profile.rs src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs src/ai/combat_search_v2/types/report/frontier.rs src/ai/combat_search_v2/types/report/core.rs src/ai/combat_search_v2/search/tests.rs
git commit -m "feat: expose finite survival damage mitigation"
```

### Task 2: Route Adaptive Rollout Through Phase-Aware Selection

**Files:**
- Modify: `src/ai/combat_search_v2/rollout_cache/mod.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/policy.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/report.rs`
- Modify: `src/content/powers/core/shifting.rs`

**Interfaces:**
- Consumes: `finite_survival_damage_mitigation_target_count` from Task 1.
- Preserves: Guardian and Bronze Automaton phase-aware dispatch and conservative fallback for all untracked mechanics.

- [ ] **Step 1: Write failing adaptive-dispatch tests**

In `rollout_cache/mod.rs`, import `Power` and `PowerPayload`, then build three otherwise identical Cultist combats: combined powers, Fading only, and Shifting only.  Assert:

```rust
assert_eq!(
    adaptive_no_potion_rollout_plugin(&test_search_node(combined)),
    CombatSearchRolloutPluginId::PhaseAwareNoPotion
);
assert_eq!(
    adaptive_no_potion_rollout_plugin(&test_search_node(fading_only)),
    CombatSearchRolloutPluginId::ConservativeNoPotion
);
assert_eq!(
    adaptive_no_potion_rollout_plugin(&test_search_node(shifting_only)),
    CombatSearchRolloutPluginId::ConservativeNoPotion
);
```

- [ ] **Step 2: Run the dispatch test and verify RED**

Run:

```powershell
cargo test --lib adaptive_no_potion_rollout_uses_phase_aware_for_finite_survival_damage_mitigation -- --nocapture
```

Expected: combined case returns `ConservativeNoPotion` instead of `PhaseAwareNoPotion`.

- [ ] **Step 3: Implement the minimal adaptive condition**

Extend the existing policy gate:

```rust
if profile
    .enemy_mechanics
    .finite_survival_damage_mitigation_target_count
    > 0
    || profile.enemy_mechanics.guardian_open_count > 0
    || profile.enemy_mechanics.guardian_defensive_count > 0
    || profile.enemy_mechanics.bronze_automaton_count > 0
    || profile.enemy_mechanics.bronze_orb_count > 0
```

Update the artifact note to name typed Guardian, Bronze Automaton, and finite-survival damage-mitigation mechanics.  Replace the stale `Shifting::at_end_of_turn` MVP comment with:

```rust
// Shifting applies paired Shackled stacks on damage; Shackled owns the
// end-of-turn Strength restoration and removes itself.
```

- [ ] **Step 4: Run dispatch and profile tests and verify GREEN**

Run:

```powershell
cargo test --lib adaptive_no_potion_rollout_uses_phase_aware -- --nocapture
cargo test --lib enemy_mechanics_profile::tests -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 5: Commit adaptive dispatch**

```powershell
git add -- src/ai/combat_search_v2/rollout_cache/mod.rs src/ai/combat_search_v2/rollout_cache/policy.rs src/ai/combat_search_v2/rollout_cache/report.rs src/content/powers/core/shifting.rs
git commit -m "fix: adapt rollout to finite survival fights"
```

### Task 3: Verify the Exact Case and Repository Boundaries

**Files:**
- Create ignored evidence: `artifacts/runs/seed006-current-from-a2f32-v2-20260714/transient-adaptive-immediate-after-fix-100k-1s.json`
- Modify only if verification exposes a defect in Tasks 1-2.

**Interfaces:**
- Consumes the exact retained capture `seed006-a3f39-transient.capture.json`.
- Produces replay-verified acceptance evidence without adding a brittle full-line test.

- [ ] **Step 1: Run focused ownership suites**

```powershell
cargo test --lib ai::combat_search_v2::enemy_mechanics_profile::tests -- --nocapture
cargo test --lib ai::combat_search_v2::rollout_cache::tests -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 2: Run the full repository-required suites**

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both commands exit zero with no failed tests.

- [ ] **Step 3: Run the one-second exact acceptance case**

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- `
  --combat-snapshot "artifacts/runs/seed006-current-from-a2f32-v2-20260714/seed006-a3f39-transient.capture.json" `
  --max-nodes 100000 --wall-ms 1000 `
  --potion-policy semantic --max-potions-used 2 `
  --rollout-policy enemy_mechanics_adaptive_no_potion `
  --child-rollout-policy immediate `
  --output "artifacts/runs/seed006-current-from-a2f32-v2-20260714/transient-adaptive-immediate-after-fix-100k-1s.json"
```

Parse the report and require `outcome.complete_win_found == true`, non-null `stats.nodes_to_first_win`, `best_win_trajectory.potions_used == 0`, and a replay-verified non-estimated best win.

- [ ] **Step 4: Check formatting and workspace scope**

```powershell
cargo fmt --all -- --check
git diff --check
git status --short
```

Expected: formatting and diff checks pass; only intentional source/document changes are present.

- [ ] **Step 5: Commit any final formatting-only change**

If `cargo fmt --all` changes an intentional file, inspect it, rerun the focused tests, and commit only that formatting change.  Otherwise create no empty commit.
