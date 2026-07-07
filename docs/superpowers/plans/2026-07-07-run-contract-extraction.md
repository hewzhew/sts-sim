# Run Contract Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract a typed `RunContract` from `branch_tiny::Args` and write it into run capsule artifacts without changing runner behavior.

**Architecture:** `Args` remains the CLI adapter shape for now. `RunContract` becomes the stable runtime/capsule identity shape, with conversion from `Args` and compatibility reads from legacy `args` artifacts. This plan deliberately does not move modules, rewrite `run_loop`, add `branch_panel`, or remove child-process continuation.

**Tech Stack:** Rust, serde JSON artifacts, existing `branch_tiny` module tests, PowerShell verification.

---

## File Structure

- Modify: `src/bin/branch_tiny/run_contract.rs`
  - Add `RunContract` and nested contract structs.
  - Add `RunContract::from_args(args: Args)`.
  - Keep existing `RunObjective` and `satisfied` behavior.

- Modify: `src/bin/branch_tiny/run_capsule_format.rs`
  - Add `run_contract` to `manifest.json`.
  - Keep `args` as a legacy projection.
  - Add focused schema test.

- Modify: `src/bin/branch_tiny/frontier_checkpoint.rs`
  - Add optional `run_contract` to frontier checkpoints.
  - New writers populate it.
  - Legacy readers fall back to converting `args`.
  - Add focused compatibility tests.

- Modify: `src/bin/branch_tiny/run_chain_state.rs`
  - Read `wall_ms` from `manifest.run_contract.slice.slice_ms` first.
  - Fall back to `manifest.args.wall_ms` for legacy capsules.
  - Add focused compatibility tests.

---

### Task 1: Add `RunContract` Types And Conversion

**Files:**
- Modify: `src/bin/branch_tiny/run_contract.rs`

- [ ] **Step 1: Write conversion tests**

Add this test module content to the existing `#[cfg(test)]` area in `src/bin/branch_tiny/run_contract.rs`. If the file has no test module, add one at the end.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> super::super::Args {
        super::super::Args {
            seed: 123,
            ascension: 7,
            objective: RunObjective::ExhaustFrontier,
            generations: 11,
            max_branches: 3,
            auto_ops: 22,
            search_nodes: 101,
            search_ms: 202,
            rescue_search_nodes: 303,
            rescue_search_ms: 404,
            boss_search_nodes: 505,
            boss_search_ms: 606,
            wall_ms: Some(707),
            checkpoint_before_combat_portfolio: true,
            wall_capped_search_budget: true,
            wall_capped_boss_budget: true,
        }
    }

    #[test]
    fn run_contract_from_args_preserves_stable_runtime_fields() {
        let contract = RunContract::from_args(sample_args());

        assert_eq!(contract.game.seed, 123);
        assert_eq!(contract.game.ascension, 7);
        assert_eq!(contract.objective, RunObjective::ExhaustFrontier);
        assert_eq!(contract.branching.generations, 11);
        assert_eq!(contract.branching.max_branches, 3);
        assert_eq!(contract.automation.auto_ops, 22);
        assert_eq!(contract.combat_search.primary_nodes, 101);
        assert_eq!(contract.combat_search.primary_ms, 202);
        assert_eq!(contract.combat_search.rescue_nodes, 303);
        assert_eq!(contract.combat_search.rescue_ms, 404);
        assert_eq!(contract.combat_search.boss_nodes, 505);
        assert_eq!(contract.combat_search.boss_ms, 606);
        assert_eq!(contract.slice.slice_ms, Some(707));
        assert!(contract.features.checkpoint_before_combat_portfolio);
    }

    #[test]
    fn run_contract_does_not_encode_per_slice_wall_cap_flags() {
        let contract = RunContract::from_args(sample_args());
        let value = serde_json::to_value(contract).unwrap();

        assert!(value.get("wall_capped_search_budget").is_none());
        assert!(value.get("wall_capped_boss_budget").is_none());
    }
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
cargo test --bin branch_tiny run_contract_from_args_preserves_stable_runtime_fields
```

Expected: FAIL because `RunContract` is not defined.

- [ ] **Step 3: Implement contract structs**

In `src/bin/branch_tiny/run_contract.rs`, add these public-within-bin types after `RunObjective`:

First, update the existing `RunObjective` derive so the conversion test can compare it directly:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RunObjective {
    FirstVictory,
    FirstTerminal,
    ExhaustFrontier,
}
```

Then add the contract structs:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RunContract {
    pub(super) game: GameRunContract,
    pub(super) objective: RunObjective,
    pub(super) branching: BranchingContract,
    pub(super) automation: AutomationContract,
    pub(super) combat_search: CombatSearchContract,
    pub(super) slice: SliceContract,
    pub(super) features: RuntimeFeatureContract,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct GameRunContract {
    pub(super) seed: u64,
    pub(super) ascension: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct BranchingContract {
    pub(super) generations: usize,
    pub(super) max_branches: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct AutomationContract {
    pub(super) auto_ops: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct CombatSearchContract {
    pub(super) primary_nodes: usize,
    pub(super) primary_ms: u64,
    pub(super) rescue_nodes: usize,
    pub(super) rescue_ms: u64,
    pub(super) boss_nodes: usize,
    pub(super) boss_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct SliceContract {
    pub(super) slice_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RuntimeFeatureContract {
    pub(super) checkpoint_before_combat_portfolio: bool,
}
```

Add the conversion impl in the same file:

```rust
impl RunContract {
    pub(super) fn from_args(args: super::Args) -> Self {
        Self {
            game: GameRunContract {
                seed: args.seed,
                ascension: args.ascension,
            },
            objective: args.objective,
            branching: BranchingContract {
                generations: args.generations,
                max_branches: args.max_branches,
            },
            automation: AutomationContract {
                auto_ops: args.auto_ops,
            },
            combat_search: CombatSearchContract {
                primary_nodes: args.search_nodes,
                primary_ms: args.search_ms,
                rescue_nodes: args.rescue_search_nodes,
                rescue_ms: args.rescue_search_ms,
                boss_nodes: args.boss_search_nodes,
                boss_ms: args.boss_search_ms,
            },
            slice: SliceContract {
                slice_ms: args.wall_ms,
            },
            features: RuntimeFeatureContract {
                checkpoint_before_combat_portfolio: args.checkpoint_before_combat_portfolio,
            },
        }
    }
}
```

- [ ] **Step 4: Run focused tests**

Run:

```powershell
cargo test --bin branch_tiny run_contract
```

Expected: PASS for both new tests.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src\bin\branch_tiny\run_contract.rs
git commit -m "Add branch tiny run contract"
```

---

### Task 2: Write `run_contract` To Capsule Manifest

**Files:**
- Modify: `src/bin/branch_tiny/run_capsule_format.rs`

- [ ] **Step 1: Write manifest schema test**

Add this test inside the existing `#[cfg(test)] mod tests` in `src/bin/branch_tiny/run_capsule_format.rs`:

```rust
fn sample_args() -> Args {
    Args {
        seed: 99,
        ascension: 3,
        objective: super::super::run_contract::RunObjective::FirstTerminal,
        generations: 8,
        max_branches: 2,
        auto_ops: 13,
        search_nodes: 100,
        search_ms: 200,
        rescue_search_nodes: 300,
        rescue_search_ms: 400,
        boss_search_nodes: 500,
        boss_search_ms: 600,
        wall_ms: Some(700),
        checkpoint_before_combat_portfolio: true,
        wall_capped_search_budget: true,
        wall_capped_boss_budget: true,
    }
}

#[test]
fn manifest_writes_run_contract_and_legacy_args_projection() {
    let value = manifest_value(
        sample_args(),
        "running",
        None,
        10,
        20,
        &Some("abc123".to_string()),
    );

    assert_eq!(value["run_contract"]["game"]["seed"], 99);
    assert_eq!(value["run_contract"]["game"]["ascension"], 3);
    assert_eq!(value["run_contract"]["slice"]["slice_ms"], 700);
    assert_eq!(value["run_contract"]["combat_search"]["boss_ms"], 600);
    assert_eq!(value["args"]["wall_ms"], 700);
    assert_eq!(value["args_schema"], "legacy_args_projection_v1");
    assert!(value["run_contract"]["wall_capped_search_budget"].is_null());
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
cargo test --bin branch_tiny manifest_writes_run_contract_and_legacy_args_projection
```

Expected: FAIL because `manifest_value` does not write `run_contract` or `args_schema`.

- [ ] **Step 3: Update manifest projection**

Modify `manifest_value` in `src/bin/branch_tiny/run_capsule_format.rs`.

Add this import near the existing imports:

```rust
use super::run_contract::RunContract;
```

Change the JSON object to include `run_contract` and `args_schema`:

```rust
json!({
    "schema": "branch_tiny_run_capsule",
    "seed": args.seed,
    "ascension": args.ascension,
    "status": status,
    "reason": reason,
    "created_at_epoch_ms": created_at_ms,
    "updated_at_epoch_ms": updated_at_ms,
    "git_commit": git_commit,
    "run_contract": RunContract::from_args(args),
    "args_schema": "legacy_args_projection_v1",
    "args": args,
})
```

- [ ] **Step 4: Run focused tests**

Run:

```powershell
cargo test --bin branch_tiny manifest_
```

Expected: PASS, including the new manifest test.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src\bin\branch_tiny\run_capsule_format.rs
git commit -m "Write run contract to capsule manifest"
```

---

### Task 3: Write `run_contract` To Frontier Checkpoints And Preserve Legacy Reads

**Files:**
- Modify: `src/bin/branch_tiny/frontier_checkpoint.rs`

- [ ] **Step 1: Write legacy checkpoint compatibility test**

Add this test module at the end of `src/bin/branch_tiny/frontier_checkpoint.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_checkpoint_json() -> String {
        serde_json::json!({
            "schema": "branch_tiny_frontier_checkpoint",
            "args": {
                "seed": 44,
                "ascension": 2,
                "objective": "first_victory",
                "generations": 6,
                "max_branches": 4,
                "auto_ops": 9,
                "search_nodes": 10,
                "search_ms": 20,
                "rescue_search_nodes": 30,
                "rescue_search_ms": 40,
                "boss_search_nodes": 50,
                "boss_search_ms": 60,
                "wall_ms": 70
            },
            "generation": 1,
            "next_branch_id": 2,
            "frontier": []
        })
        .to_string()
    }

    #[test]
    fn legacy_checkpoint_without_run_contract_loads_contract_from_args() {
        let path = std::env::temp_dir().join("branch_tiny_legacy_frontier_checkpoint.json");
        fs::write(&path, legacy_checkpoint_json()).unwrap();

        let checkpoint = load(&path).unwrap();
        let contract = checkpoint.run_contract();

        assert_eq!(contract.game.seed, 44);
        assert_eq!(contract.game.ascension, 2);
        assert_eq!(contract.branching.generations, 6);
        assert_eq!(contract.slice.slice_ms, Some(70));

        let _ = fs::remove_file(path);
    }
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
cargo test --bin branch_tiny legacy_checkpoint_without_run_contract_loads_contract_from_args
```

Expected: FAIL because `FrontierCheckpoint::run_contract` does not exist.

- [ ] **Step 3: Add optional contract to checkpoint schema**

In `src/bin/branch_tiny/frontier_checkpoint.rs`, add this import:

```rust
use super::run_contract::RunContract;
```

Change `FrontierCheckpoint` to:

```rust
#[derive(Deserialize, Serialize)]
pub(super) struct FrontierCheckpoint {
    schema: String,
    pub(super) args: Args,
    #[serde(default)]
    run_contract: Option<RunContract>,
    pub(super) generation: usize,
    next_branch_id: usize,
    frontier: Vec<BranchCheckpoint>,
}
```

In `save`, populate the new field:

```rust
run_contract: Some(RunContract::from_args(args)),
```

Add this impl method:

```rust
impl FrontierCheckpoint {
    pub(super) fn run_contract(&self) -> RunContract {
        self.run_contract
            .unwrap_or_else(|| RunContract::from_args(self.args))
    }

    pub(super) fn into_frontier(self) -> Result<(VecDeque<Branch>, usize), String> {
        let mut frontier = VecDeque::new();
        for branch in self.frontier {
            frontier.push_back(branch.into_branch()?);
        }
        Ok((frontier, self.next_branch_id))
    }
}
```

If `into_frontier` already exists, merge the new `run_contract` method into the existing `impl FrontierCheckpoint` block instead of duplicating `into_frontier`.

- [ ] **Step 4: Add writer projection test**

Add this second test in the same test module:

```rust
#[test]
fn checkpoint_writer_includes_run_contract() {
    let args = Args {
        seed: 45,
        ascension: 1,
        objective: super::super::run_contract::RunObjective::FirstVictory,
        generations: 2,
        max_branches: 1,
        auto_ops: 3,
        search_nodes: 4,
        search_ms: 5,
        rescue_search_nodes: 6,
        rescue_search_ms: 7,
        boss_search_nodes: 8,
        boss_search_ms: 9,
        wall_ms: Some(10),
        checkpoint_before_combat_portfolio: false,
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    };
    let path = std::env::temp_dir().join("branch_tiny_frontier_checkpoint_contract.json");
    let frontier = VecDeque::new();

    save(&path, args, 0, 1, &frontier).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(value["run_contract"]["game"]["seed"], 45);
    assert_eq!(value["run_contract"]["slice"]["slice_ms"], 10);
    assert_eq!(value["args"]["wall_ms"], 10);

    let _ = fs::remove_file(path);
}
```

- [ ] **Step 5: Run focused tests**

Run:

```powershell
cargo test --bin branch_tiny frontier_checkpoint
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```powershell
git add src\bin\branch_tiny\frontier_checkpoint.rs
git commit -m "Store run contract in frontier checkpoints"
```

---

### Task 4: Read Slice Budget From Modern Manifest First

**Files:**
- Modify: `src/bin/branch_tiny/run_chain_state.rs`

- [ ] **Step 1: Write manifest read compatibility tests**

Add this test module at the end of `src/bin/branch_tiny/run_chain_state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir_name: &str, value: serde_json::Value) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(dir_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn manifest_wall_ms_prefers_run_contract_slice_ms() {
        let dir = write_manifest(
            "branch_tiny_manifest_run_contract_wall",
            serde_json::json!({
                "run_contract": {
                    "slice": { "slice_ms": 1234 }
                },
                "args": { "wall_ms": 9999 }
            }),
        );

        assert_eq!(manifest_wall_ms(&dir).unwrap(), Some(1234));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn manifest_wall_ms_falls_back_to_legacy_args() {
        let dir = write_manifest(
            "branch_tiny_manifest_legacy_wall",
            serde_json::json!({
                "args": { "wall_ms": 4321 }
            }),
        );

        assert_eq!(manifest_wall_ms(&dir).unwrap(), Some(4321));

        let _ = fs::remove_dir_all(dir);
    }
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```powershell
cargo test --bin branch_tiny manifest_wall_ms_prefers_run_contract_slice_ms
```

Expected: FAIL because `manifest_wall_ms` still reads only `args.wall_ms`.

- [ ] **Step 3: Update manifest wall read order**

Change `manifest_wall_ms` to:

```rust
pub(super) fn manifest_wall_ms(capsule: &Path) -> Result<Option<u64>, String> {
    let manifest = capsule.join("manifest.json");
    if !manifest.exists() {
        return Ok(None);
    }
    let value = read_json(&manifest)?;
    Ok(value
        .get("run_contract")
        .and_then(|contract| contract.get("slice"))
        .and_then(|slice| slice.get("slice_ms"))
        .and_then(Value::as_u64)
        .or_else(|| {
            value
                .get("args")
                .and_then(|args| args.get("wall_ms"))
                .and_then(Value::as_u64)
        }))
}
```

- [ ] **Step 4: Run focused tests**

Run:

```powershell
cargo test --bin branch_tiny manifest_wall_ms_
```

Expected: PASS for both manifest wall tests.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src\bin\branch_tiny\run_chain_state.rs
git commit -m "Read continuation slice budget from run contract"
```

---

### Task 5: Verify No Behavior Drift

**Files:**
- No new source file changes expected.

- [ ] **Step 1: Run formatting**

Run:

```powershell
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, review the diff, and amend only formatting changes into the relevant previous commit.

- [ ] **Step 2: Run branch_tiny tests**

Run:

```powershell
cargo test --bin branch_tiny
```

Expected: PASS.

- [ ] **Step 3: Run all target check**

Run:

```powershell
cargo check --all-targets
```

Expected: PASS.

- [ ] **Step 4: Run whitespace check**

Run:

```powershell
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Smoke one capsule manifest**

Run:

```powershell
$root = "target\run_contract_smoke"
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
cargo run --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --generations 1 --wall-ms 5000 --run-capsule $root
Get-Content "$root\manifest.json" | Select-String "run_contract"
```

Expected: command exits successfully and prints a line containing `run_contract`.

- [ ] **Step 6: Commit smoke/doc-free verification marker only if files changed**

Run:

```powershell
git status --short
```

Expected: clean. If verification caused tracked changes, inspect them and commit only intentional changes.

---

## Self-Review

- Spec coverage: this plan covers the first implementation cut from the durable panel design: add `RunContract`, convert from `Args`, write `run_contract` into manifest/frontier artifacts, preserve legacy `args`, and keep behavior unchanged.
- Out of scope by design: moving files into `src/runtime`, rewriting `run_loop`, adding `RunSliceResult`, adding `ArtifactStore`, adding `branch_panel`, and deleting child-process continuation.
- Type consistency: `slice_ms` is the `RunContract` field corresponding to legacy `wall_ms`; per-slice wall cap flags stay out of `RunContract`.
- Test policy: tests protect typed interface and artifact migration contracts, not prose or exact terminal output.
