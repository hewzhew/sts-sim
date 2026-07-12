# Nonpositive Card Energy Payment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent legal negative-cost non-X hand plays from increasing player energy while preserving Blue Candle, Medical Kit, positive-cost, and X-cost behavior.

**Architecture:** Normalize payment at the existing hand-play boundary in `handle_play_card_from_hand`; legality and card evaluation remain unchanged. Exercise the behavior through `tick_until_stable_turn` so the tests cover the real input, action queue, relic hooks, HP loss, and exhaust destinations.

**Tech Stack:** Rust, Cargo library tests, existing combat engine test support, `combat_search_v2_driver` diagnostics.

## Global Constraints

- Work in `D:\rust\sts_simulator`; do not create a worktree or dispatch subagents.
- Do not modify `TurnRuntime::spend_energy`, card legality, relic hooks, X-cost actions, combat search, run-control, or owner policy.
- Add no seed-outcome unit test; the saved A3F35 capture is diagnostic evidence only.
- Use test-first red/green development and make one production-code change.

---

### Task 1: Normalize non-X hand-play payment

**Files:**
- Modify: `src/engine/core.rs` (test module)
- Modify: `src/engine/action_handlers/cards/play_queue.rs:307-324`

**Interfaces:**
- Consumes: `ClientInput::PlayCard`, `tick_until_stable_turn`, `CombatCard`, `RelicState`, and the existing `handle_play_card_from_hand` payment path.
- Produces: the invariant that a non-X hand play passes a nonnegative amount to `TurnRuntime::spend_energy`.

- [ ] **Step 1: Add two failing end-to-end mechanism tests**

Add these tests to the existing `tests` module in `src/engine/core.rs`:

```rust
#[test]
fn nonpositive_cost_relic_play_blue_candle_spends_zero_energy_like_java() {
    let mut combat_state = blank_test_combat();
    combat_state.entities.monsters = vec![planned_monster(EnemyId::JawWorm, 1)];
    combat_state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::BlueCandle));
    combat_state.turn.energy = 4;
    combat_state.zones.hand = vec![CombatCard::new(CardId::Writhe, 90_001)];
    let mut engine_state = EngineState::CombatPlayerTurn;

    let alive = super::tick_until_stable_turn(
        &mut engine_state,
        &mut combat_state,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(alive);
    assert_eq!(combat_state.turn.energy, 4);
    assert_eq!(combat_state.entities.player.current_hp, 79);
    assert!(combat_state
        .zones
        .exhaust_pile
        .iter()
        .any(|card| card.id == CardId::Writhe));
}

#[test]
fn nonpositive_cost_relic_play_medical_kit_spends_zero_energy_like_java() {
    let mut combat_state = blank_test_combat();
    combat_state.entities.monsters = vec![planned_monster(EnemyId::JawWorm, 1)];
    combat_state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MedicalKit));
    combat_state.turn.energy = 4;
    combat_state.zones.hand = vec![CombatCard::new(CardId::Burn, 90_002)];
    let mut engine_state = EngineState::CombatPlayerTurn;

    let alive = super::tick_until_stable_turn(
        &mut engine_state,
        &mut combat_state,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(alive);
    assert_eq!(combat_state.turn.energy, 4);
    assert_eq!(combat_state.entities.player.current_hp, 80);
    assert!(combat_state
        .zones
        .exhaust_pile
        .iter()
        .any(|card| card.id == CardId::Burn));
}
```

- [ ] **Step 2: Run the tests and verify the red state**

Run:

```powershell
cargo test --lib nonpositive_cost_relic_play -- --nocapture
```

Expected: both tests compile and fail at the energy assertion because the current implementation produces `6` from an initial `4`. HP loss and exhaust assertions are not the cause of the failure.

- [ ] **Step 3: Make the minimal payment correction**

In `handle_play_card_from_hand`, change only the non-X branch of `energy_on_use`:

```rust
let energy_on_use = if is_x_cost {
    state.turn.energy as i32
} else {
    effective_cost.max(0)
};
```

Leave the affordability check, X-cost capture, and later action-specific X-cost spending unchanged.

- [ ] **Step 4: Verify the green state and adjacent cost semantics**

Run:

```powershell
cargo test --lib nonpositive_cost_relic_play -- --nocapture
cargo test --lib upgraded_base_cost_is_used_when_spending_energy -- --nocapture
cargo test --lib ironclad_debuff_draw_xcost_and_wound_definitions_match_java_sources -- --nocapture
```

Expected: all commands exit successfully; the new test command reports two passing tests, the upgraded positive-cost test retains `3 -> 1`, and the X-cost test retains generic-path capture followed by action-specific spending.

- [ ] **Step 5: Inspect the exact patch**

Run:

```powershell
git diff --check
git diff -- src/engine/core.rs src/engine/action_handlers/cards/play_queue.rs
```

Expected: no whitespace errors; the production diff contains only `effective_cost.max(0)` and the test diff contains only the two mechanism tests.

---

### Task 2: Verify globally and reassess A3F35

**Files:**
- Inspect: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json`
- Create: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_immediate_800k_8s_after_nonpositive_cost_fix.json`
- Create: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_lazy_800k_8s_after_nonpositive_cost_fix.json`

**Interfaces:**
- Consumes: the corrected combat engine and the existing restorable A3F35 capture.
- Produces: fresh, non-exploit search evidence for deciding whether Exploder/Spiker tactical work is still needed.

- [ ] **Step 1: Run repository completion suites**

Run:

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both commands exit successfully with zero failed tests.

- [ ] **Step 2: Run the immediate A3F35 diagnostic**

Run:

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json --max-nodes 800000 --wall-ms 8000 --child-rollout-policy immediate --output artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_immediate_800k_8s_after_nonpositive_cost_fix.json
```

Expected: the output JSON is written successfully. Record completion, final HP, HP loss, first action, nodes, and wall time; do not require a particular combat outcome.

- [ ] **Step 3: Run the lazy A3F35 diagnostic**

Run:

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json --max-nodes 800000 --wall-ms 8000 --child-rollout-policy lazy-on-pop --output artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_lazy_800k_8s_after_nonpositive_cost_fix.json
```

Expected: the output JSON is written successfully. Compare it with the immediate result and verify that no reported line gains energy by playing Writhe or another negative-cost non-X card.

- [ ] **Step 4: Commit the verified fix**

Run:

```powershell
git add -- src/engine/core.rs src/engine/action_handlers/cards/play_queue.rs
git commit -m "fix: prevent negative card costs from granting energy"
```

Do not add diagnostic JSON unless it is already tracked by the repository's artifact policy. Expected: one local code commit after all required tests and diagnostics complete.
