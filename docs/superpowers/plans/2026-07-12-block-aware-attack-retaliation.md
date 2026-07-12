# Block-Aware Attack Retaliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace raw retaliation damage mislabeled as HP loss with engine-derived raw damage, sequential block consumption, and actual HP-loss facts.

**Architecture:** Extract the numeric player-damage stages from the live engine handler into a shared resolver. Combat search lazily clones combat state only after finding retaliation, applies retaliation actions to that scratch state in order, and projects the three quantities through action facts, ordering risk, and diagnostics.

**Tech Stack:** Rust 2021, existing combat engine power/relic hooks, serde diagnostics, Cargo unit and architecture tests.

## Global Constraints

- Work in the stable checkout on local `master`; do not create a worktree or dispatch subagents.
- Do not run `cargo clean`.
- Do not add `EnemyId::Spiker` checks, Shapes-specific target priority, or retaliation coefficients.
- Do not add a path ledger, change frontier/state identity, prune actions, or add a frozen trajectory assertion.
- The live engine and AI projection must share one numeric player-damage resolver.
- Clone projection state lazily and never mutate the live state while deriving action facts.
- Raw retaliation damage is diagnostic only; ordering risk counts block consumed plus actual HP loss exactly once.

---

### Task 1: Extract the engine-owned player damage resolver

**Files:**
- Modify: `src/engine/action_handlers/damage/core.rs`
- Modify: `src/engine/action_handlers/damage.rs`

**Interfaces:**
- Produces: `resolve_player_damage(info: &DamageInfo, state: &mut CombatState) -> PlayerDamageResolution`.
- Produces: `PlayerDamageResolution::{raw_damage, block_consumed, damage_before_hp_loss_hooks, hp_loss}`.
- Consumed by: live `handle_damage` and Task 2's scratch retaliation projection.

- [ ] **Step 1: Write the failing resolver parity tests**

Add a `#[cfg(test)]` module in `core.rs` with a focused test that sets player block to 2 and resolves a 3-damage Thorns action:

```rust
#[test]
fn player_damage_resolution_reports_block_and_hp_loss_without_subtracting_hp() {
    let mut projected = blank_test_combat();
    projected.entities.player.current_hp = 20;
    projected.entities.player.block = 2;
    let info = DamageInfo {
        source: 1,
        target: 0,
        base: 3,
        output: 3,
        damage_type: DamageType::Thorns,
        is_modified: false,
    };

    let resolution = resolve_player_damage(&info, &mut projected);

    assert_eq!(resolution.raw_damage, 3);
    assert_eq!(resolution.block_consumed, 2);
    assert_eq!(resolution.damage_before_hp_loss_hooks, 1);
    assert_eq!(resolution.hp_loss, 1);
    assert_eq!(projected.entities.player.block, 0);
    assert_eq!(projected.entities.player.current_hp, 20);
}
```

Add a second test that clones the same setup, calls `handle_damage`, and proves the live block delta and HP delta equal the resolver result. Add focused Buffer and Tungsten Rod assertions so stateful power consumption and `onLoseHpLast` remain on the shared path.

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --lib player_damage_resolution -- --nocapture
```

Expected: compilation fails because `resolve_player_damage` and `PlayerDamageResolution` do not exist.

- [ ] **Step 3: Implement the shared numeric resolver**

Add this public result type in `core.rs`:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlayerDamageResolution {
    pub raw_damage: i32,
    pub block_consumed: i32,
    pub damage_before_hp_loss_hooks: i32,
    pub hp_loss: i32,
}
```

Implement `resolve_player_damage` by moving the player-only numeric stages, in their existing order, out of `handle_damage`:

```rust
pub fn resolve_player_damage(
    info: &DamageInfo,
    state: &mut CombatState,
) -> PlayerDamageResolution {
    let (raw_damage, already_final_receive) = calculate_damage_action_output(state, info);
    let mut damage = raw_damage;
    if !already_final_receive {
        for power in &store::powers_snapshot_for(state, 0) {
            damage = crate::content::powers::resolve_power_at_damage_final_receive(
                power.power_type,
                damage,
                power.amount,
                info.damage_type,
            );
        }
    }
    let block_before = state.entities.player.block.max(0);
    damage = deduct_block(&mut state.entities.player.block, damage);
    let block_consumed = block_before.saturating_sub(state.entities.player.block.max(0));
    damage = crate::content::relics::hooks::on_attacked_to_change_damage(state, damage, info);
    for power in &store::powers_snapshot_for(state, 0) {
        damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
            power.power_type,
            state,
            info,
            damage,
            power.amount,
        );
    }
    let damage_before_hp_loss_hooks = damage.max(0);
    let hp_loss = crate::content::relics::hooks::on_lose_hp_last(
        state,
        damage_before_hp_loss_hooks,
    )
    .max(0);
    PlayerDamageResolution {
        raw_damage: raw_damage.max(0),
        block_consumed,
        damage_before_hp_loss_hooks,
        hp_loss,
    }
}
```

Change the player branch of `handle_damage` to call this resolver, pass `damage_before_hp_loss_hooks` to existing `on_attacked` hooks, and apply `hp_loss` to HP. Leave monster damage calculation unchanged. Re-export the type and function from `damage.rs`.

- [ ] **Step 4: Run GREEN and neighboring damage tests**

Run:

```powershell
cargo fmt --all
cargo test --lib player_damage_resolution -- --nocapture
cargo test --lib content::relics::tests -- --nocapture
```

Expected: resolver tests and existing relic damage-pipeline tests pass.

- [ ] **Step 5: Commit the engine boundary**

```powershell
git add -- src/engine/action_handlers/damage.rs src/engine/action_handlers/damage/core.rs
git commit -m "refactor: share player damage resolution"
```

---

### Task 2: Project sequential retaliation defense costs

**Files:**
- Modify: `src/ai/combat_search_v2/attack_retaliation.rs`
- Modify: `src/ai/combat_search_v2/action_effects/card_play_effects/observation.rs`
- Modify: `src/ai/combat_search_v2/action_effects/card_play_effects/attack_retaliation_observation.rs`
- Modify: `src/ai/combat_search_v2/action_effects/types.rs`
- Modify: `src/ai/combat_search_v2/action_effects/tests.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`

**Interfaces:**
- Produces: `AttackRetaliationEventProjectionV1::{trigger_count, raw_player_damage, player_block_loss, player_hp_loss}`.
- Extends: `ReactiveCardPlayEffectFacts` and its diagnostics with raw retaliation damage and retaliation block loss.
- Consumes: Task 1's `resolve_player_damage` on a lazily cloned `CombatState`.

- [ ] **Step 1: Write failing sequential-block tests**

Add a test beside `attack_retaliation_counts_explicit_damage_events_without_affecting_non_attacks`:

```rust
#[test]
fn attack_retaliation_consumes_current_block_sequentially_before_hp() {
    let mut combat = blank_test_combat();
    combat.entities.player.block = 5;
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 1;
    combat.entities.monsters = vec![spiker];
    insert_power(&mut combat, 1, PowerId::Thorns, 3);

    let twin = card_play_effect_facts(
        &combat,
        &CombatCard::new(CardId::TwinStrike, 10),
        Some(1),
    );

    assert_eq!(twin.reactive.attack_retaliation_trigger_count_hint, 2);
    assert_eq!(twin.reactive.attack_retaliation_raw_player_damage_hint, 6);
    assert_eq!(twin.reactive.attack_retaliation_player_block_loss_hint, 5);
    assert_eq!(twin.reactive.attack_retaliation_player_hp_loss_hint, 1);
    assert_eq!(twin.reactive.player_hp_loss, 1);
    assert_eq!(twin.reactive_risk_score(), 6);
    assert_eq!(combat.entities.player.block, 5);
}
```

Add a fully blocked Strike assertion: raw 3, block loss 3, HP loss 0, one trigger. Extend the existing priority test so a block-protected retaliation action still carries nonzero defense-resource risk.

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --lib attack_retaliation_consumes_current_block -- --nocapture
```

Expected: compilation fails because raw-damage and block-loss facts are absent.

- [ ] **Step 3: Implement the event projection and lazy scratch state**

Replace the integer-returning event helper with:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct AttackRetaliationEventProjectionV1 {
    pub(super) trigger_count: usize,
    pub(super) raw_player_damage: i32,
    pub(super) player_block_loss: i32,
    pub(super) player_hp_loss: i32,
}
```

Resolve target `on_attacked` actions once. For each positive `Action::Damage` targeting player, initialize `scratch.get_or_insert_with(|| combat.clone())`, call `resolve_player_damage`, and saturating-add its raw, block, and HP fields. For positive `Action::LoseHp`, apply `on_lose_hp_last`, count no block, and count the resulting HP loss. Count positive emitted events even when projected HP loss is zero.

Add `retaliation_projection_state: Option<CombatState>` to `CardPlayEffectAccumulator`. The observer passes it across every explicit damage action from the candidate card so multi-hit block and Buffer consumption are sequential.

- [ ] **Step 4: Extend reactive facts and ordering cost**

Add these fields to both internal reactive fact/diagnostic structs:

```rust
pub attack_retaliation_raw_player_damage_hint: i32,
pub attack_retaliation_player_block_loss_hint: i32,
```

Project the event fields in `attack_retaliation_observation.rs`. Keep `reactive.player_hp_loss` equal to projected HP loss only. Add `attack_retaliation_player_block_loss_hint` to `reactive_risk_score`; do not add raw damage.

- [ ] **Step 5: Run GREEN and action-ordering tests**

Run:

```powershell
cargo fmt --all
cargo test --lib attack_retaliation_consumes_current_block -- --nocapture
cargo test --lib ai::combat_search_v2::action_effects::tests -- --nocapture
cargo test --lib damage_progress_prefers_fewer_attack_retaliation_triggers -- --nocapture
```

Expected: sequential block, fully blocked, existing unblocked, and priority tests pass.

- [ ] **Step 6: Commit sequential projection**

```powershell
git add -- src/ai/combat_search_v2/attack_retaliation.rs src/ai/combat_search_v2/action_effects src/ai/combat_search_v2/action_priority/tests.rs
git commit -m "feat: project retaliation defense costs"
```

---

### Task 3: Correct public facts and diagnostics

**Files:**
- Modify: `src/ai/combat_search_v2/action_facts/types.rs`
- Modify: `src/ai/combat_search_v2/action_facts/target.rs`
- Modify: `src/ai/combat_search_v2/action_facts/mechanics.rs`
- Modify: `src/ai/combat_search_v2/action_facts/tests.rs`
- Modify: `src/ai/combat_search_v2/types/diagnostics/action.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/collector.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/report.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/tests/diagnostics.rs`
- Modify: `src/ai/combat_search_v2/diagnostics_tags/tests/fixtures.rs`

**Interfaces:**
- Replaces target field: `player_hp_loss_per_damage_event` with raw, next-event block loss, and next-event HP loss.
- Extends action mechanics and diagnostic samples with raw retaliation damage and block loss.
- Extends ordering aggregates with total raw retaliation damage and total retaliation block loss.

- [ ] **Step 1: Write failing public-fact and collector tests**

Update `facts_report_attack_retaliation_on_target` to give the player 2 block and assert:

```rust
assert_eq!(retaliation.raw_player_damage_per_damage_event, 3);
assert_eq!(retaliation.projected_player_block_loss_for_next_damage_event, 2);
assert_eq!(retaliation.projected_player_hp_loss_for_next_damage_event, 1);
```

Update the ordering diagnostics test to give the player 5 block across Strike and Twin Strike candidate observations and assert each candidate uses an independent scratch projection. Assert raw, block-loss, and HP-loss totals explicitly, including that fully blocked actions still increment `attack_retaliation_actions`.

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --lib facts_report_attack_retaliation_on_target -- --nocapture
cargo test --lib ordering_collector_reports_attack_retaliation -- --nocapture
```

Expected: compilation fails on the new public schema fields.

- [ ] **Step 3: Propagate truthful public fields**

Rename the target fact field and add the two next-event projections in `AttackRetaliationTargetFactV1`, `CombatSearchV2AttackRetaliationTargetFacts`, and `target.rs`. Extend `CombatSearchV2ActionReactiveMechanicsFacts`, `CombatSearchV2DiagnosticsActionEffectReactive`, and every internal-to-public projection with:

```rust
attack_retaliation_raw_player_damage_hint
attack_retaliation_player_block_loss_hint
```

Remove every reference to the misleading target field rather than retaining a compatibility alias.

- [ ] **Step 4: Correct collector semantics**

Add collector/report totals:

```rust
pub attack_retaliation_raw_player_damage_hint: i64,
pub attack_retaliation_player_block_loss_hint: i64,
```

Count `attack_retaliation_actions` when raw damage is positive, not only when HP loss is positive. Aggregate triggers, raw damage, block loss, and HP loss independently. Update diagnostics notes to say that these are defense-aware candidate projections, not realized trajectory loss. Add zero-valued fields to tag fixtures.

- [ ] **Step 5: Run GREEN and schema neighbors**

Run:

```powershell
cargo fmt --all
cargo test --lib facts_report_attack_retaliation_on_target -- --nocapture
cargo test --lib ordering_collector_reports_attack_retaliation -- --nocapture
cargo test --lib ai::combat_search_v2::diagnostics_tags::tests -- --nocapture
rg -n "player_hp_loss_per_damage_event" src
```

Expected: tests pass and the final search returns no references to the misleading name.

- [ ] **Step 6: Commit the corrected schema**

```powershell
git add -- src/ai/combat_search_v2/action_facts src/ai/combat_search_v2/action_ordering src/ai/combat_search_v2/types/diagnostics/action.rs src/ai/combat_search_v2/diagnostics_tags/tests/fixtures.rs
git commit -m "feat: report defense-aware retaliation diagnostics"
```

---

### Task 4: Verify repository behavior and frozen evidence

**Files:**
- Create ignored evidence: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/diagnostics/a3f42_lazy_all_8s_block_aware_retaliation.json`

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: regression evidence and an updated frozen diagnostic; no permanent seed assertion and no push.

- [ ] **Step 1: Run repository verification**

```powershell
cargo fmt --all -- --check
git diff --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: all library and all seven architecture-boundary tests pass.

- [ ] **Step 2: Rerun frozen A3F42 diagnostics**

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\accepted_high_loss_combat\seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json" --max-nodes 800000 --wall-ms 8000 --potion-policy all --max-potions-used 3 --child-rollout-policy lazy-on-pop --output "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\diagnostics\a3f42_lazy_all_8s_block_aware_retaliation.json"
```

Expected: ordering diagnostics contain separate raw, block-loss, and HP-loss fields. Record trajectory equality or change as evidence, not as a pass criterion.

- [ ] **Step 3: Inspect final state**

```powershell
git status --short --branch
git log -10 --oneline
```

Expected: tracked work is committed locally, ignored evidence does not dirty the tree, and the branch is not pushed.
