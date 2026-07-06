# Frozen Case Panel V0a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first review-only Frozen Case Panel for three fixed combat cases and two explicit search lanes.

**Architecture:** Freeze selected historical combat cases into tracked fixtures, then run a tiny panel wrapper over `combat_case_review`. Rust owns combat search and lane execution; the panel wrapper only orchestrates cases, extracts stable rows, and writes `panel_rows.jsonl` plus `panel_table.md`.

**Tech Stack:** Rust `combat_case_review`, `serde_json`, Python 3 for the thin panel wrapper, tracked JSON combat fixtures.

---

## Boundaries

V0a implements only:

```text
3 frozen combat cases
2 explicit lanes
panel_rows.jsonl
panel_table.md
```

V0a does not implement:

```text
Fresh Run Panel
HTML output
automatic interpretation
automatic next-step recommendation
runner integration
root action intervention
case auto-discovery
```

## Frozen Cases

Copy these exact source files into tracked fixtures before implementing the panel. If any source path is missing, stop and recapture the case; do not silently substitute another target artifact.

```powershell
Copy-Item -LiteralPath 'target\gap-panel-after-falling\1552225675\combat_cases\seed1552225675_g42_b0042_a3f48_cultist_cultist_awakenedone.json' -Destination 'fixtures\combat_cases\frozen_v0a_awakened_one_1552225675_a3f48.json'
Copy-Item -LiteralPath 'target\gap-panel-after-falling\1552225671\combat_cases\seed1552225671_g25_b0025_a2f32_thecollector.json' -Destination 'fixtures\combat_cases\frozen_v0a_collector_1552225671_a2f32.json'
Copy-Item -LiteralPath 'target\gap-panel-after-vampires-v0\1552225671\combat_cases\seed1552225671_g22_b0022_a2f29_gremlintsundere_gremlinwarrior_gremlinleader.json' -Destination 'fixtures\combat_cases\frozen_v0a_gremlin_leader_1552225671_a2f29.json'
```

Expected case facts after copy:

```text
frozen_v0a_awakened_one_1552225675_a3f48.json:
  seed=1552225675
  act=3 floor=48
  hp=105/128
  enemies=Cultist,Cultist,AwakenedOne

frozen_v0a_collector_1552225671_a2f32.json:
  seed=1552225671
  act=2 floor=32
  hp=80/80
  enemies=TheCollector

frozen_v0a_gremlin_leader_1552225671_a2f29.json:
  seed=1552225671
  act=2 floor=29
  hp=52/80
  enemies=GremlinTsundere,GremlinWarrior,GremlinLeader
```

Validate the copied fixtures:

```powershell
cargo run --quiet --bin combat_case_review -- --case fixtures\combat_cases\frozen_v0a_awakened_one_1552225675_a3f48.json --ladder --slow-nodes 800000 --slow-ms 8000 --diagnostic-potion-max 3 --compact --write-review target\frozen-case-panel-v0a\validate_awakened.json
cargo run --quiet --bin combat_case_review -- --case fixtures\combat_cases\frozen_v0a_collector_1552225671_a2f32.json --ladder --slow-nodes 800000 --slow-ms 8000 --diagnostic-potion-max 3 --compact --write-review target\frozen-case-panel-v0a\validate_collector.json
cargo run --quiet --bin combat_case_review -- --case fixtures\combat_cases\frozen_v0a_gremlin_leader_1552225671_a2f29.json --ladder --slow-nodes 800000 --slow-ms 8000 --diagnostic-potion-max 3 --compact --write-review target\frozen-case-panel-v0a\validate_gremlin_leader.json
```

Expected: each command exits 0 and writes a valid review JSON.

## Lane Configs

### baseline

`baseline` is a named search config, not the current default.

```text
lane=baseline
source_label=slow_potion_diagnostic
max_nodes=800000
wall_ms=8000
turn_plan_policy=DiagnosticOnly
potion_policy=All
max_potions_used=3
rollout_policy=EnemyMechanicsAdaptiveNoPotion
child_rollout_policy=LazyOnPop
setup_bias_policy=Default
phase_guard_policy=Default
```

### key_setup_bias

`key_setup_bias` differs from `baseline` only by setup bias.

```text
lane=key_setup_bias
source_label=key_setup_bias
max_nodes=800000
wall_ms=8000
turn_plan_policy=DiagnosticOnly
potion_policy=All
max_potions_used=3
rollout_policy=EnemyMechanicsAdaptiveNoPotion
child_rollout_policy=LazyOnPop
setup_bias_policy=KeyCardOnline
phase_guard_policy=Default
```

Rules for `key_setup_bias`:

```text
It may only affect action ordering.
It must not change legal actions.
It must not prune actions.
It must not force a root action.
It must not contain Awakened One / Demon Form / Collector / Gremlin Leader special cases.
It must run on all three frozen cases, even when no key setup card is present.
```

## Implementation Shape

### Task 1: Freeze Case Fixtures

**Files:**
- Create: `fixtures/combat_cases/frozen_v0a_awakened_one_1552225675_a3f48.json`
- Create: `fixtures/combat_cases/frozen_v0a_collector_1552225671_a2f32.json`
- Create: `fixtures/combat_cases/frozen_v0a_gremlin_leader_1552225671_a2f29.json`
- Modify: `fixtures/combat_cases/README.md`

- [ ] **Step 1: Copy the exact source cases**

Run the three `Copy-Item` commands from the Frozen Cases section.

- [ ] **Step 2: Verify the copied case schemas**

Run:

```powershell
@'
import json
from pathlib import Path
for path in [
    Path("fixtures/combat_cases/frozen_v0a_awakened_one_1552225675_a3f48.json"),
    Path("fixtures/combat_cases/frozen_v0a_collector_1552225671_a2f32.json"),
    Path("fixtures/combat_cases/frozen_v0a_gremlin_leader_1552225671_a2f29.json"),
]:
    data = json.loads(path.read_text())
    print(path.name, data["schema"], data["run"]["act"], data["run"]["floor"], ",".join(data["combat"]["enemies"]))
'@ | python -
```

Expected:

```text
frozen_v0a_awakened_one_1552225675_a3f48.json combat_case 3 48 Cultist,Cultist,AwakenedOne
frozen_v0a_collector_1552225671_a2f32.json combat_case 2 32 TheCollector
frozen_v0a_gremlin_leader_1552225671_a2f29.json combat_case 2 29 GremlinTsundere,GremlinWarrior,GremlinLeader
```

- [ ] **Step 3: Update fixture README**

Add a short "Frozen Case Panel V0a" section to `fixtures/combat_cases/README.md` listing the three tracked fixture paths and saying they are review-only combat boundaries, not campaign-policy verdicts.

### Task 2: Add a Review-Only Panel Lane Runner

**Files:**
- Create: `src/bin/combat_case_review/frozen_panel_lanes.rs`
- Modify: `src/bin/combat_case_review.rs`
- Modify: `src/bin/combat_case_review/options.rs`

- [ ] **Step 1: Add a gated artifact**

Add a new optional artifact to `CombatCaseReviewArtifacts` and `CombatCaseReview` in `case_payload.rs`:

```rust
frozen_panel_lanes: Option<FrozenPanelLaneReview>
```

Name the serialized JSON field exactly `frozen_panel_lanes`.

- [ ] **Step 2: Implement two lane searches**

Create `frozen_panel_lanes.rs` with one public function:

```rust
pub(super) fn run_frozen_panel_lanes(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<FrozenPanelLaneReview>
```

The function returns `None` unless `options.frozen_panel_lanes` is true.

It runs exactly two searches:

```text
baseline:
  CombatSearchV2Config {
    max_nodes: options.slow_nodes,
    wall_time: Some(Duration::from_millis(options.slow_ms)),
    turn_plan_policy: DiagnosticOnly,
    potion_policy: All,
    max_potions_used: Some(options.diagnostic_potion_max),
    rollout_policy: EnemyMechanicsAdaptiveNoPotion unless --disable-rollout,
    child_rollout_policy: options.child_rollout_policy(),
    setup_bias_policy: Default,
    phase_guard_policy: Default,
    ..
  }

key_setup_bias:
  same config, but setup_bias_policy: KeyCardOnline
```

- [ ] **Step 3: Add CLI switch**

Add:

```rust
#[arg(long)]
frozen_panel_lanes: bool,
```

to `Args`, and pass it through `ReviewOptions`.

- [ ] **Step 4: Verify it is review-only**

Run:

```powershell
cargo check --bin combat_case_review
```

Expected: pass. No `branch_tiny` runner files should be modified by this task.

### Task 3: Add the Thin Panel Wrapper

**Files:**
- Create: `tools/frozen_case_panel.py`
- Test: `tests/test_frozen_case_panel.py`

- [ ] **Step 1: Write parser tests first**

Create tests covering:

```text
extract baseline row from frozen_panel_lanes.lanes[0]
extract key_setup_bias row from frozen_panel_lanes.lanes[1]
classify half_dead + player dead as phase_complete_but_player_died
classify missing terminal/deadline facts as incomplete_or_unknown
write markdown table with one row per case/lane
```

Run:

```powershell
python -m pytest tests\test_frozen_case_panel.py
```

Expected before implementation: fail because `tools.frozen_case_panel` does not exist.

- [ ] **Step 2: Implement row extraction**

`tools/frozen_case_panel.py` should:

```text
read a fixed case list from code for V0a
run combat_case_review once per case with --frozen-panel-lanes
write each raw review under target/frozen-case-panel-v0a/reviews/
write target/frozen-case-panel-v0a/panel_rows.jsonl
write target/frozen-case-panel-v0a/panel_table.md
```

The fixed case list is exactly:

```text
fixtures/combat_cases/frozen_v0a_awakened_one_1552225675_a3f48.json
fixtures/combat_cases/frozen_v0a_collector_1552225671_a2f32.json
fixtures/combat_cases/frozen_v0a_gremlin_leader_1552225671_a2f29.json
```

- [ ] **Step 3: Keep output structural**

Each `panel_rows.jsonl` row must include:

```text
case_id
case_path
case_origin_seed
captured_at_commit
reviewed_at_commit
lane
search_config_summary
complete_win
outcome_tier
final_hp
turns
potions_used
first_action_key
first_action_role
key_card_played
key_card_first_play_step
living_enemy_count
total_enemy_hp
half_dead_enemy_count
phase_pending_enemy_player_died
nodes_expanded
elapsed_ms
deadline_hit
tool_status
```

If a field is not available from the raw review, set it to `null` rather than inventing a value.

- [ ] **Step 4: Verify wrapper**

Run:

```powershell
python -m pytest tests\test_frozen_case_panel.py
python tools\frozen_case_panel.py
```

Expected:

```text
target/frozen-case-panel-v0a/panel_rows.jsonl
target/frozen-case-panel-v0a/panel_table.md
```

exist, and `panel_rows.jsonl` contains exactly 6 rows.

### Task 4: Final Verification and Commit

**Files:**
- Modify only the files from Tasks 1-3.

- [ ] **Step 1: Run verification**

```powershell
cargo fmt --check
cargo check --bin combat_case_review
python -m pytest tests\test_frozen_case_panel.py
python -m py_compile tools\frozen_case_panel.py
git diff --check
```

- [ ] **Step 2: Confirm runner isolation**

Run:

```powershell
git diff --name-only
```

Expected: no `src/bin/branch_tiny*` files.

- [ ] **Step 3: Commit**

```powershell
git add fixtures/combat_cases src/bin/combat_case_review.rs src/bin/combat_case_review tools/frozen_case_panel.py tests/test_frozen_case_panel.py
git commit -m "Add frozen combat case panel"
```

## Review Notes

This plan deliberately uses tracked fixture files instead of `target/` paths for panel input. `target/` artifacts are only source material for the initial freeze.

The panel wrapper must not decide whether a lane is good. It only records rows. Human review and later external review decide whether `key_setup_bias` is worth promoting.
