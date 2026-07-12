# Attack Retaliation Diagnostics Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete JSON-visible attack-retaliation attribution and search-wide exposure counters without changing search ordering, pruning, merging, rollout, or frontier value.

**Architecture:** Reuse the retaliation fields already carried by `CardPlayEffectDiagnostics`. Project them into public ordering samples, aggregate them while the existing ordering collector observes candidate actions, and expose the counters in the additive diagnostics schema.

**Tech Stack:** Rust 2021, serde diagnostics structs, existing combat-search action-ordering collector, Cargo tests.

## Global Constraints

- Work in the stable checkout on local `master`; do not create a worktree or dispatch subagents.
- Do not run `cargo clean`.
- Do not add `EnemyId::Spiker` checks or enemy-specific weights.
- Do not change action ordering, rollout comparison, frontier comparison, pruning, or exact state identity.
- New counters describe candidate observations, not unique paths and not realized HP loss.
- Keep the frozen A3F42 rerun as ignored evidence, not a permanent trajectory test.

---

### Task 1: Project attribution and aggregate search exposure

**Files:**
- Modify: `src/ai/combat_search_v2/action_ordering/tests/diagnostics.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/collector.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/report.rs`
- Modify: `src/ai/combat_search_v2/types/diagnostics/action.rs`
- Modify: `src/ai/combat_search_v2/diagnostics_tags/tests/fixtures.rs`

**Interfaces:**
- Consumes: `CardPlayReactiveEffectDiagnostics::{attack_retaliation_trigger_count_hint, attack_retaliation_player_hp_loss_hint}` from the completed action-effects slice.
- Produces: JSON-visible sample attribution plus four fields on `CombatSearchV2DiagnosticsOrdering`: `attack_retaliation_actions`, `attack_retaliation_trigger_count_hint`, `attack_retaliation_player_hp_loss_hint`, and `max_attack_retaliation_player_hp_loss_hint`.

- [ ] **Step 1: Add a failing collector test**

Add this test beside the existing ordering diagnostic tests:

```rust
#[test]
fn ordering_collector_reports_attack_retaliation_attribution_and_exposure() {
    let mut combat = blank_test_combat();
    let mut target = test_monster(EnemyId::Spiker);
    target.id = 1;
    target.current_hp = 40;
    target.max_hp = 40;
    combat.entities.monsters = vec![target];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Thorns,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::TwinStrike, 11),
    ];
    let ordered = order_action_choices(
        &EngineState::CombatPlayerTurn,
        &combat,
        vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ],
    );
    let mut collector = ActionOrderingDiagnosticsCollector::default();

    collector.observe(&ordered.summary);
    let report = collector.finish();

    assert_eq!(report.attack_retaliation_actions, 2);
    assert_eq!(report.attack_retaliation_trigger_count_hint, 3);
    assert_eq!(report.attack_retaliation_player_hp_loss_hint, 9);
    assert_eq!(report.max_attack_retaliation_player_hp_loss_hint, 6);
    let twin = report
        .action_effect_samples
        .iter()
        .find(|sample| sample.action_key.contains("Twin Strike"))
        .expect("Twin Strike retaliation sample");
    assert_eq!(twin.reactive.attack_retaliation_trigger_count_hint, 2);
    assert_eq!(
        twin.reactive.attack_retaliation_player_hp_loss_hint,
        6
    );
}
```

- [ ] **Step 2: Add a failing neutral test**

Extend `ordering_collector_reports_role_counts_without_action_tree` with:

```rust
assert_eq!(report.attack_retaliation_actions, 0);
assert_eq!(report.attack_retaliation_trigger_count_hint, 0);
assert_eq!(report.attack_retaliation_player_hp_loss_hint, 0);
assert_eq!(report.max_attack_retaliation_player_hp_loss_hint, 0);
```

- [ ] **Step 3: Run RED**

Run:

```powershell
cargo test --lib ordering_collector_reports_attack_retaliation -- --nocapture
```

Expected: compilation fails because the public report fields are absent.

- [ ] **Step 4: Add internal collector counters**

Add these fields to `ActionOrderingDiagnosticsCollector`:

```rust
pub(super) attack_retaliation_actions: u64,
pub(super) attack_retaliation_trigger_count_hint: u64,
pub(super) attack_retaliation_player_hp_loss_hint: i64,
pub(super) max_attack_retaliation_player_hp_loss_hint: i32,
```

Inside the existing `for sample in &summary.action_effect_samples` loop, before remembering the sample, add:

```rust
let retaliation = sample.effects.reactive;
if retaliation.attack_retaliation_player_hp_loss_hint > 0 {
    self.attack_retaliation_actions = self.attack_retaliation_actions.saturating_add(1);
    self.attack_retaliation_trigger_count_hint = self
        .attack_retaliation_trigger_count_hint
        .saturating_add(retaliation.attack_retaliation_trigger_count_hint as u64);
    self.attack_retaliation_player_hp_loss_hint = self
        .attack_retaliation_player_hp_loss_hint
        .saturating_add(i64::from(
            retaliation.attack_retaliation_player_hp_loss_hint,
        ));
    self.max_attack_retaliation_player_hp_loss_hint = self
        .max_attack_retaliation_player_hp_loss_hint
        .max(retaliation.attack_retaliation_player_hp_loss_hint);
}
```

- [ ] **Step 5: Add public schema fields and projection**

Add these fields to `CombatSearchV2DiagnosticsOrdering`:

```rust
pub attack_retaliation_actions: u64,
pub attack_retaliation_trigger_count_hint: u64,
pub attack_retaliation_player_hp_loss_hint: i64,
pub max_attack_retaliation_player_hp_loss_hint: i32,
```

Add these fields to `CombatSearchV2DiagnosticsActionEffectReactive`:

```rust
pub attack_retaliation_trigger_count_hint: usize,
pub attack_retaliation_player_hp_loss_hint: i32,
```

Project collector totals in `ActionOrderingDiagnosticsCollector::finish` and project the two reactive fields in `action_effect_samples`. Add zero values for the four ordering counters to `diagnostics_tags/tests/fixtures.rs`.

Append this note to the ordering report notes:

```rust
"attack retaliation totals count candidate observations, not unique paths or realized HP loss",
```

- [ ] **Step 6: Run GREEN and neighboring diagnostics tests**

Run:

```powershell
cargo fmt --all
cargo test --lib ordering_collector_reports_attack_retaliation -- --nocapture
cargo test --lib ai::combat_search_v2::action_ordering::tests::diagnostics -- --nocapture
cargo test --lib ai::combat_search_v2::diagnostics_tags::tests -- --nocapture
```

Expected: all focused diagnostics tests pass and no search behavior tests require changes.

- [ ] **Step 7: Commit diagnostic completion**

```powershell
git add -- src/ai/combat_search_v2/action_ordering src/ai/combat_search_v2/types/diagnostics/action.rs src/ai/combat_search_v2/diagnostics_tags/tests/fixtures.rs
git commit -m "feat: report attack retaliation search exposure"
```

---

### Task 2: Verify schema and frozen evidence

**Files:**
- Inspect: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/accepted_high_loss_combat/seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json`
- Create ignored evidence: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/diagnostics/a3f42_lazy_all_8s_attack_retaliation_diagnostics.json`

**Interfaces:**
- Consumes: completed diagnostic schema from Task 1.
- Produces: search-wide exposure evidence used to decide whether a future path ledger is justified; no behavioral change.

- [ ] **Step 1: Run repository verification**

```powershell
cargo fmt --all -- --check
git diff --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: all library and all seven architecture-boundary tests pass.

- [ ] **Step 2: Rerun frozen A3F42**

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\accepted_high_loss_combat\seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json" --max-nodes 800000 --wall-ms 8000 --potion-policy all --max-potions-used 3 --child-rollout-policy lazy-on-pop --output "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\diagnostics\a3f42_lazy_all_8s_attack_retaliation_diagnostics.json"
```

Expected: output contains nonzero ordering-level retaliation counters and action samples with explicit attribution. Because this slice is diagnostic-only, the selected trajectory is allowed and expected to remain unchanged.

- [ ] **Step 3: Apply the evidence gate**

Record:

- final HP and action-key equality against `a3f42_lazy_all_8s_attack_retaliation.json`;
- search-wide retaliation candidate count, trigger count, total projected loss, and maximum projected loss;
- whether samples contain both single-hit and multi-hit attribution.

Do not implement the deferred path ledger unless the report identifies comparable path families or a documented coverage collapse. If it does not, close this goal at the diagnostic boundary.

- [ ] **Step 4: Confirm clean local state**

```powershell
git status --short --branch
git log -8 --oneline
```

Expected: tracked working tree is clean, evidence JSON remains ignored, and all work is committed locally without pushing.
