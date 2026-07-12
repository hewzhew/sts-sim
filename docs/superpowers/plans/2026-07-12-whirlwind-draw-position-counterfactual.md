# Whirlwind Draw-Position Counterfactual Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce ignored, replay-checked evidence showing whether one unupgraded Whirlwind at representative frozen draw-pile positions can make seed `20260712002`'s A2F32 Collector combat winnable.

**Architecture:** Add one ignored/manual test module to the existing `combat_case_review` binary, use public combat-search and witness-replay APIs against cloned `CombatCase` values, and write a compact JSON report under the existing seed capsule. Remove the temporary module after the run so no production CLI, schema, or permanent seed test remains.

**Tech Stack:** Rust, `serde_json`, Combat Search V2, `CombatCase`, optimized Cargo `fast-run` test profile.

## Global Constraints

- Keep the original combat case and run capsule byte-for-byte unchanged.
- Add exactly one unupgraded `CardId::Whirlwind`; the control remains the original 21-card combat and each mutation has 22 cards.
- Draw-pile index `0` is next draw; representative insert indices are `0`, `original_len / 2`, and `original_len`.
- Use `CombatSearchV2PotionPolicy::All`, `max_potions_used = Some(3)`, adaptive no-potion rollout, lazy-on-pop child rollout, and round-robin frontier.
- Screen baseline and each representative position at 200,000 nodes / 2,000 ms; rerun the strongest Whirlwind position at 800,000 nodes / 8,000 ms.
- An exact win claim requires `replay_combat_search_witness_line_v0` to finish at `CombatTerminal::Win` with matched terminal, final HP, and enemy HP.
- Write durable evidence only under `artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics`.
- Remove all temporary tracked code after evidence generation; do not add a permanent CLI, schema, card-injection API, or seed assertion.

---

### Task 1: Add the temporary position-portfolio probe

**Files:**
- Modify temporarily: `src/bin/combat_case_review.rs`
- Create temporarily: `src/bin/combat_case_review/whirlwind_draw_position_probe_tests.rs`

**Interfaces:**
- Consumes: `load_combat_case(&Path) -> Result<CombatCase, String>`, `run_combat_search_v2`, `CombatState::next_card_uuid`, `CombatCard::new`, `replay_combat_search_witness_line_v0`.
- Produces: ignored JSON evidence at `artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json`.

- [ ] **Step 1: Wire the ignored manual test module**

Append this test-only module declaration to `src/bin/combat_case_review.rs`:

```rust
#[cfg(test)]
#[path = "combat_case_review/whirlwind_draw_position_probe_tests.rs"]
mod whirlwind_draw_position_probe_tests;
```

- [ ] **Step 2: Create the temporary probe test**

Create `src/bin/combat_case_review/whirlwind_draw_position_probe_tests.rs` with these responsibilities and exact helper signatures:

```rust
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};
use sts_simulator::ai::combat_search_v2::{
    replay_combat_search_witness_line_v0, run_combat_search_v2,
    CombatSearchV2ActionPreview, CombatSearchV2Config, CombatSearchV2PotionPolicy,
    CombatSearchV2Report, CombatSearchV2WitnessLine,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::eval::combat_case::{load_combat_case, CombatCase};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::sim::combat::CombatTerminal;

const CASE_PATH: &str = "artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/combat_cases/seed20260712002_g26_b0026_a2f32_thecollector.json";
const OUTPUT_PATH: &str = "artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json";

fn search(case: &CombatCase, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Report {
    run_combat_search_v2(
        &case.position.engine,
        &case.position.combat,
        CombatSearchV2Config {
            max_nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            potion_policy: CombatSearchV2PotionPolicy::All,
            max_potions_used: Some(3),
            input_label: Some("whirlwind_draw_position_counterfactual".to_string()),
            ..CombatSearchV2Config::default()
        },
    )
}

fn with_whirlwind_at(base: &CombatCase, insert_index: usize) -> CombatCase {
    let mut case = base.clone();
    let uuid = case.position.combat.next_card_uuid();
    let whirlwind = CombatCard::new(CardId::Whirlwind, uuid);
    case.position
        .combat
        .meta
        .master_deck_snapshot
        .push(whirlwind.clone());
    case.position
        .combat
        .zones
        .draw_pile
        .insert(insert_index, whirlwind);
    case
}

fn trajectory_progress(
    trajectory: &sts_simulator::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> (i32, i32, i32) {
    let living = trajectory
        .enemy_final_state
        .iter()
        .filter(|enemy| enemy.alive)
        .count() as i32;
    let enemy_hp = trajectory
        .enemy_final_state
        .iter()
        .filter(|enemy| enemy.alive)
        .map(|enemy| enemy.hp.max(0) + enemy.block.max(0))
        .sum::<i32>();
    (living, enemy_hp, trajectory.final_hp)
}

fn report_rank(report: &CombatSearchV2Report) -> (i32, i32, i32, i32) {
    let win = i32::from(report.best_win_trajectory.is_some());
    let best = report
        .best_win_trajectory
        .as_ref()
        .into_iter()
        .chain(report.best_complete_trajectory.as_ref())
        .chain(report.best_frontier_trajectory.as_ref())
        .map(trajectory_progress)
        .min_by_key(|(living, enemy_hp, final_hp)| (*living, *enemy_hp, -*final_hp));
    let (living, enemy_hp, final_hp) = best.unwrap_or((i32::MAX, i32::MAX, 0));
    (win, -living, -enemy_hp, final_hp)
}

fn report_json(label: &str, insert_index: Option<usize>, report: &CombatSearchV2Report) -> Value {
    let progress = report
        .best_win_trajectory
        .as_ref()
        .or(report.best_complete_trajectory.as_ref())
        .or(report.best_frontier_trajectory.as_ref());
    json!({
        "label": label,
        "insert_index": insert_index,
        "complete_win": report.best_win_trajectory.is_some(),
        "progress": progress.map(|trajectory| json!({
            "terminal": trajectory.terminal,
            "final_hp": trajectory.final_hp,
            "hp_loss": trajectory.hp_loss,
            "turns": trajectory.turns,
            "potions_used": trajectory.potions_used,
            "living_enemy_count": trajectory_progress(trajectory).0,
            "total_enemy_hp": trajectory_progress(trajectory).1,
            "action_count": trajectory.actions.len(),
        })),
        "stats": &report.stats,
    })
}

fn witness_from_win(
    trajectory: &sts_simulator::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> CombatSearchV2WitnessLine {
    CombatSearchV2WitnessLine {
        source: "whirlwind_draw_position_counterfactual",
        terminal: trajectory.terminal,
        final_hp: trajectory.final_hp,
        total_enemy_hp: trajectory_progress(trajectory).1,
        action_count: Some(trajectory.actions.len()),
        actions: trajectory
            .actions
            .iter()
            .map(|action| CombatSearchV2ActionPreview {
                action_key: action.action_key.clone(),
                input: action.input.clone(),
            })
            .collect(),
    }
}
```

Add the ignored test body below those helpers:

```rust
#[test]
#[ignore = "manual frozen Whirlwind draw-position counterfactual"]
fn temporary_whirlwind_draw_position_portfolio() {
    let base = load_combat_case(Path::new(CASE_PATH)).expect("Collector case should load");
    let original_len = base.position.combat.zones.draw_pile.len();
    let baseline = search(&base, 200_000, 2_000);
    assert!(baseline.best_win_trajectory.is_none(), "baseline unexpectedly won");

    let positions = [
        ("next_draw", 0usize),
        ("middle_draw", original_len / 2),
        ("last_draw", original_len),
    ];
    let mut screened = Vec::new();
    for (label, insert_index) in positions {
        let case = with_whirlwind_at(&base, insert_index);
        let report = search(&case, 200_000, 2_000);
        screened.push((label, insert_index, report));
    }
    let selected_index = screened
        .iter()
        .enumerate()
        .max_by_key(|(_, (_, _, report))| report_rank(report))
        .map(|(index, _)| index)
        .expect("three Whirlwind positions should be screened");
    let (selected_label, selected_insert_index, _) = &screened[selected_index];
    let selected_case = with_whirlwind_at(&base, *selected_insert_index);
    let full = search(&selected_case, 800_000, 8_000);
    let replay = full.best_win_trajectory.as_ref().map(|trajectory| {
        replay_combat_search_witness_line_v0(&selected_case.position, &witness_from_win(trajectory))
    });
    if let Some(replay) = &replay {
        assert_eq!(replay.terminal, CombatTerminal::Win);
        assert!(replay.matched_witness_terminal);
        assert!(replay.matched_witness_final_hp);
        assert!(replay.matched_witness_enemy_hp);
        assert!(!replay.truncated && !replay.timed_out);
    }

    let short_runs = std::iter::once(report_json("baseline", None, &baseline))
        .chain(screened.iter().map(|(label, index, report)| {
            report_json(label, Some(*index), report)
        }))
        .collect::<Vec<_>>();
    let exact_short_position_wins = screened
        .iter()
        .filter(|(_, _, report)| report.best_win_trajectory.is_some())
        .count();
    let evidence = json!({
        "schema": "whirlwind_draw_position_counterfactual_v0",
        "contract": "diagnostic_only_frozen_combat_add_one_unupgraded_whirlwind_no_runner_policy_change",
        "case_path": CASE_PATH,
        "original_draw_count": original_len,
        "mutated_draw_count": original_len + 1,
        "short_budget": { "max_nodes": 200000, "wall_ms": 2000 },
        "full_budget": { "max_nodes": 800000, "wall_ms": 8000 },
        "short_runs": short_runs,
        "selected_position": {
            "label": selected_label,
            "insert_index": selected_insert_index,
        },
        "full_run": report_json(selected_label, Some(*selected_insert_index), &full),
        "exact_replay": replay,
        "interpretation": if full.best_win_trajectory.is_some() {
            if exact_short_position_wins >= 2 { "robust_multi_position_win" }
            else { "position_sensitive_win" }
        } else {
            "no_win_found"
        },
    });
    std::fs::write(
        OUTPUT_PATH,
        serde_json::to_string_pretty(&evidence).expect("evidence should serialize"),
    )
    .expect("evidence should write");
    eprintln!("wrote {OUTPUT_PATH}");
}
```

- [ ] **Step 3: Format and compile the temporary probe without running it**

Run:

```powershell
cargo fmt --all
cargo test --profile fast-run --bin combat_case_review temporary_whirlwind_draw_position_portfolio --no-run
```

Expected: optimized test binary compiles successfully; no search has run yet.

---

### Task 2: Execute, validate, and interpret the experiment

**Files:**
- Read: `artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json`
- Verify unchanged: `artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/combat_cases/seed20260712002_g26_b0026_a2f32_thecollector.json`

**Interfaces:**
- Consumes: `temporary_whirlwind_draw_position_portfolio` from Task 1.
- Produces: a replay-gated interpretation label and durable ignored JSON evidence.

- [ ] **Step 1: Record the original combat-case SHA-256**

Run:

```powershell
Get-FileHash -Algorithm SHA256 artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/combat_cases/seed20260712002_g26_b0026_a2f32_thecollector.json
```

Expected: one SHA-256 hash to compare after the experiment.

- [ ] **Step 2: Run the ignored optimized probe serially**

Run:

```powershell
cargo test --profile fast-run --bin combat_case_review temporary_whirlwind_draw_position_portfolio -- --ignored --nocapture --test-threads=1
```

Expected: one ignored test runs for roughly 16 seconds, passes, and prints the evidence path.

- [ ] **Step 3: Validate the evidence and replay gate**

Run:

```powershell
$probe = Get-Content -Raw artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json | ConvertFrom-Json
$probe | Select-Object schema, interpretation, selected_position | Format-List
$probe.short_runs | Select-Object label, insert_index, complete_win, progress, stats | Format-List
$probe.full_run | Format-List
$probe.exact_replay | Format-List terminal, final_hp, total_enemy_hp, living_enemy_count, replayed_actions, matched_witness_terminal, matched_witness_final_hp, matched_witness_enemy_hp, truncated, timed_out
```

Expected: JSON parses. If `full_run.complete_win` is true, `exact_replay.terminal` is `win`, all three match flags are true, and neither truncation flag is set. Otherwise `interpretation` is `no_win_found` and `exact_replay` is null.

- [ ] **Step 4: Recheck the original case hash**

Run the same `Get-FileHash` command from Step 1.

Expected: identical SHA-256 value, proving the source combat case was not mutated.

---

### Task 3: Remove temporary code and restore the tracked workspace

**Files:**
- Modify: `src/bin/combat_case_review.rs`
- Delete: `src/bin/combat_case_review/whirlwind_draw_position_probe_tests.rs`
- Preserve ignored: `artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json`

**Interfaces:**
- Consumes: completed evidence from Task 2.
- Produces: clean tracked worktree with only the already committed design and plan retained.

- [ ] **Step 1: Remove the temporary module declaration and test file**

Use `apply_patch` to delete exactly:

```rust
#[cfg(test)]
#[path = "combat_case_review/whirlwind_draw_position_probe_tests.rs"]
mod whirlwind_draw_position_probe_tests;
```

Then delete `src/bin/combat_case_review/whirlwind_draw_position_probe_tests.rs` with `apply_patch`.

- [ ] **Step 2: Verify formatting, evidence, and tracked cleanliness**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
Test-Path artifacts/runs/bounded-mainline-seed-20260712002-reliability-probe/diagnostics/whirlwind_draw_position_portfolio.json
git status --short --branch
```

Expected: formatting and diff checks succeed, evidence exists, and no tracked source changes remain. The branch is ahead only by the committed design and implementation-plan documents.

- [ ] **Step 3: Report without changing production policy**

Report the three short position results, the selected full result, replay status, and one of these bounded conclusions:

- `robust_multi_position_win`: strong evidence that skipping Whirlwind materially hurt this run;
- `position_sensitive_win`: Whirlwind can rescue a favorable representative order, but evidence is insufficient for a general reward-policy change;
- `no_win_found`: the Whirlwind-only hypothesis weakened; investigate a wider construction or search-coverage boundary next.

Do not commit the ignored evidence and do not modify card-reward behavior in this plan.
