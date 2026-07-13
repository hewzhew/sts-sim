# Combat Laboratory V1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a sequential, resumable offline combat laboratory that crosses controlled shuffle samples with explicit combat-search profiles and emits trustworthy raw and descriptive evidence without changing live policy.

**Architecture:** Add a library-owned `eval::combat_lab_v1` subsystem, make the smallest simulator seam needed to observe draw history during exact replay, and expose the subsystem through the maintained `combat_search_v2_driver`. The journal is authoritative, the manifest freezes the experiment contract, and summaries are pure deterministic derivations of manifest plus cells.

**Tech Stack:** Rust, serde/serde_json, clap, Blake2, the existing combat-start compiler, Combat Search V2, and the existing runtime source-identity/fingerprint types.

## Global Constraints

- Work in the existing stable checkout; do not create a worktree and do not run `cargo clean`.
- Use test-driven development for each behavioral step: add one focused failing test, run it and inspect the expected failure, implement the minimum behavior, then rerun it.
- Keep the laboratory offline. Nothing under route planning, run-control selection, card acquisition, shops, or campfires may import `combat_lab_v1`.
- Preserve the old `CombatStepResult` and witness V0 JSON shapes. Add opt-in V1 observation/replay APIs instead of widening hot-path structures.
- Run cells sequentially in sample-major order. Do not add threads, subprocess-per-cell execution, Cargo invocation, learning, fitting, or parameter promotion.
- Commit after every numbered task. Before the final commit, follow `AGENTS.md`: formatting, full library tests, and `architecture_runtime_boundaries`.

---

## File Structure

```text
src/eval/combat_lab_v1/
  mod.rs                 public exports
  contract.rs            spec, resolved contract, schedule and hashes
  scenario.rs            controlled-shuffle compilation and isolation checks
  replay.rs              exact replay evidence and cell classification
  artifact.rs            manifest/journal/checkpoint persistence and resume
  summary.rs             profile, pair and interaction summaries
  runner.rs              sequential sample-major orchestration
  tests.rs               subsystem tests
src/bin/combat_search_v2_driver/main.rs
fixtures/combat_lab/seed006_reptomancer_derived.start.json
fixtures/combat_lab/seed006_reptomancer_8x2.lab.json
docs/RUNBOOK.md
docs/architecture/supported-surfaces.md
src/bin/README.md
```

### Task 1: Compile a natural start with only the shuffle stream replaced

**Files:**

- Modify: `src/testing/combat_start_spec.rs`
- Modify: `src/testing/mod.rs` or the existing fixture re-export module only if the new function is not already reachable there
- Test: existing tests colocated in `src/testing/combat_start_spec.rs`

**Interfaces:**

- Consumes: `CombatStartSpec`, `RunState::new`, `StsRng::new`, and `build_natural_combat_start`.
- Produces: `compile_combat_start_spec_with_rng_overrides(&CombatStartSpec, u64, Option<u64>) -> Result<(EngineState, CombatState), String>` while preserving both old compiler functions.

- [ ] **Step 1: Write the failing isolation test**

Add a test named `shuffle_override_changes_only_shuffle_rng_before_natural_start`. It must:

1. build one small deterministic `CombatStartSpec`;
2. compile it once with the existing function and once with a new shuffle override;
3. compare the two starts after natural initialization;
4. assert equal monster identities, HP, initial intentions, engine boundary, player resources, and every RNG stream except `shuffle_rng`;
5. assert that the shuffle RNG state and at least one of opening hand/draw-pile order differ.

Also add `same_shuffle_override_reproduces_identical_start` and compare the full `(EngineState, CombatState)` result.

Run:

```powershell
cargo test --lib shuffle_override_changes_only_shuffle_rng_before_natural_start
```

Expected before implementation: compilation fails because the controlled compiler does not exist.

- [ ] **Step 2: Add the narrow compiler seam**

Refactor the existing compiler through one internal function and add this public API:

```rust
pub fn compile_combat_start_spec_with_rng_overrides(
    spec: &CombatStartSpec,
    seed: u64,
    shuffle_seed: Option<u64>,
) -> Result<(EngineState, CombatState), String>;
```

Immediately after `RunState::new(seed, ascension_level, false, player_class)` and before `build_natural_combat_start`, apply:

```rust
if let Some(shuffle_seed) = shuffle_seed {
    run_state.rng_pool.shuffle_rng = StsRng::new(shuffle_seed);
}
```

Keep these compatibility rules exact:

```rust
compile_combat_start_spec(spec)
    == compile_combat_start_spec_with_rng_overrides(spec, spec.seed, None)

compile_combat_start_spec_with_seed(spec, seed)
    == compile_combat_start_spec_with_rng_overrides(spec, seed, None)
```

Do not add a generic mutable `RngPool` callback; V1 is authorized to vary only shuffle RNG.

- [ ] **Step 3: Verify and commit**

Run:

```powershell
cargo test --lib combat_start_spec
git diff --check
git add src/testing/combat_start_spec.rs src/testing/mod.rs
git commit -m "feat: isolate combat start shuffle sampling"
```

Add only paths that actually changed.

### Task 2: Freeze the experiment contract and deterministic schedule

**Files:**

- Create: `src/eval/combat_lab_v1/mod.rs`
- Create: `src/eval/combat_lab_v1/contract.rs`
- Create: `src/eval/combat_lab_v1/tests.rs`
- Modify: `src/eval/mod.rs`
- Modify: `src/ai/combat_search_v2/types/labels.rs`
- Modify: `src/testing/combat_start_spec.rs`

**Interfaces:**

- Consumes: Task 1's controlled start compiler plus all serialized Combat Search V2 policy enums.
- Produces: `ResolvedCombatLabSpecV1`, `derive_shuffle_seed_v1`, `load_and_resolve_combat_lab_spec_v1`, and `profile_config_v1` for every later task.

- [ ] **Step 1: Write failing contract tests**

Add tests for:

- `splitmix64_v1` schedule seed `42` producing:
  - sample 0: `13679457532755275413`
  - sample 1: `2949826092126892291`
  - sample 2: `5139283748462763858`
- duplicate profile IDs being rejected;
- an empty profile list being rejected;
- a profile-local resource budget being impossible because budgets exist only in the common block;
- canonical hashes being equal after JSON key reordering;
- a changed profile policy or common budget changing the corresponding hash;
- deserializing `SearchCoverageStatus` from snake-case artifact JSON.

Run:

```powershell
cargo test --lib combat_lab_v1::tests::schedule_is_frozen
```

Expected before implementation: the module and types do not exist.

- [ ] **Step 2: Define exact versioned input types**

Use `#[serde(deny_unknown_fields)]` on input contract structs. Define:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabSpecV1 {
    pub schema_version: u32,
    pub experiment_id: String,
    pub scenario_id: String,
    pub start_spec: PathBuf,
    pub schedule: CombatLabShuffleScheduleV1,
    pub profiles: Vec<CombatLabProfileSpecV1>,
    pub common_budget: CombatLabCommonBudgetV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabShuffleScheduleV1 {
    pub generator: CombatLabShuffleGeneratorV1,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabShuffleGeneratorV1 { SplitMix64V1 }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabProfileSpecV1 {
    pub id: String,
    pub label: String,
    pub information_scope: CombatLabInformationScopeV1,
    pub potion_policy: CombatSearchV2PotionPolicy,
    pub rollout_policy: CombatSearchV2RolloutPolicy,
    pub child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    pub turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    pub frontier_policy: CombatSearchV2FrontierPolicy,
    pub phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
    pub setup_bias_policy: CombatSearchV2SetupBiasPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabInformationScopeV1 { ExactStateOracle }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCommonBudgetV1 {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_ms: Option<u64>,
    pub stop_on_win_hp_loss_at_most: Option<u32>,
    pub min_win_candidates_before_stop: usize,
    pub max_potions_used: Option<u32>,
    pub rollout_max_evaluations: usize,
    pub rollout_max_actions: usize,
    pub rollout_beam_width: usize,
    pub turn_plan_probe_max_inner_nodes: Option<usize>,
    pub turn_plan_probe_max_end_states: Option<usize>,
    pub turn_plan_probe_per_bucket_limit: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolvedCombatLabProfileV1 {
    pub spec: CombatLabProfileSpecV1,
    pub profile_hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolvedCombatLabSpecV1 {
    pub schema_version: u32,
    pub experiment_id: String,
    pub scenario_id: String,
    pub lab_spec_path: PathBuf,
    pub start_spec_path: PathBuf,
    pub start_spec_snapshot: CombatStartSpec,
    pub schedule: CombatLabShuffleScheduleV1,
    pub profiles: Vec<ResolvedCombatLabProfileV1>,
    pub common_budget: CombatLabCommonBudgetV1,
    pub scenario_hash: String,
    pub budget_hash: String,
    pub experiment_hash: String,
}
```

Canonicalize the two provenance paths during resolution, but compute semantic hashes from versioned content rather than local path spelling. `scenario_hash` covers the parsed start snapshot; each profile hash covers its information scope and search policies; `budget_hash` covers the full common block; `experiment_hash` covers schema/generator versions, experiment/scenario IDs, scenario hash, ordered profile IDs/labels/hashes, schedule, and budget hash. It excludes provenance paths, source identity, creation time, and requested sample count.

Use canonical serde values plus the existing Blake2 dependency for hashes. Sort object keys recursively, serialize compactly, truncate the `Blake2b512` digest to the first 32 bytes consistently with the repository fingerprint convention, and hex-encode it. Do not hash `Debug` output. The profile configuration hash excludes the profile ID and human label; the experiment hash still includes the full ordered profile identities and labels.

Implement:

```rust
pub fn derive_shuffle_seed_v1(
    schedule: &CombatLabShuffleScheduleV1,
    sample_index: u64,
) -> u64;

pub fn load_and_resolve_combat_lab_spec_v1(
    lab_spec_path: &Path,
) -> Result<ResolvedCombatLabSpecV1, String>;

pub fn profile_config_v1(
    experiment_id: &str,
    profile: &CombatLabProfileSpecV1,
    budget: &CombatLabCommonBudgetV1,
) -> CombatSearchV2Config;
```

Freeze `SplitMix64V1` as `state = schedule.seed + GOLDEN_GAMMA * (sample_index + 1)` with wrapping `u64` arithmetic, followed by the standard SplitMix64 xor/shift/multiply finalizer using constants `0xBF58476D1CE4E5B9` and `0x94D049BB133111EB`. Do not retain mutable generator state; each index is independently derivable.

`profile_config_v1` starts from `CombatSearchV2Config::default()`, overwrites every serialized policy and every common budget field, derives `input_label` from experiment/profile IDs, sets both prior fields to `None`, and never reads global defaults for a field represented in the contract. The common block includes the turn-plan probe caps and early-stop fields because they change comparison cost or termination semantics.

Add `Deserialize` to `SearchCoverageStatus`; do not otherwise change the enum. Add `Serialize` to `CombatStartSpec`, `StartSpecCardSpec`, `StartSpecCardEntry`, `StartSpecRelicSpec`, and `StartSpecRelicEntry` so the resolved manifest can retain the parsed source snapshot without lossy conversion. Put `#[serde(deny_unknown_fields)]` on the two detailed start-entry structs and add a test that an unsupported nested card or relic field fails preflight instead of being silently discarded.

- [ ] **Step 3: Verify and commit**

Run:

```powershell
cargo test --lib combat_lab_v1::tests
cargo test --lib search_coverage_status
git diff --check
git add src/eval/combat_lab_v1 src/eval/mod.rs src/ai/combat_search_v2/types/labels.rs src/testing/combat_start_spec.rs
git commit -m "feat: define combat laboratory contract"
```

### Task 3: Add opt-in draw observation and exact replay V1

**Files:**

- Modify: `src/sim/combat.rs`
- Modify: `src/runtime/combat/state.rs`
- Modify: `src/ai/combat_search_v2/witness_guidance.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Test: colocated test modules in those files

**Interfaces:**

- Consumes: existing `apply_combat_input_to_stable`, `DomainEvent::CardDrawn`, and witness V0 types.
- Produces: `CombatObservedStepResultV1`, `apply_combat_input_to_stable_observed_v1`, `CombatSearchV2WitnessReplayV1`, and `replay_combat_search_witness_line_v1` without changing V0 artifacts.

- [ ] **Step 1: Prove observation without changing V0**

Add tests named:

- `observed_step_returns_cards_drawn_during_that_action`;
- `unobserved_step_still_clears_draw_events`;
- `witness_replay_v1_records_draw_history`;
- `witness_replay_v0_json_shape_is_unchanged`.

The first test must execute a deterministic draw action and compare ordered `DomainCardSnapshot` values. The V0 compatibility test serializes the existing V0 witness types and asserts that no `drawn_cards` field appears.

Run:

```powershell
cargo test --lib observed_step_returns_cards_drawn_during_that_action
```

Expected before implementation: the observed API is missing.

- [ ] **Step 2: Add an observed wrapper, not a hot-path field**

Define:

```rust
#[derive(Clone, Debug)]
pub struct CombatObservedStepResultV1 {
    pub step: CombatStepResult,
    pub drawn_cards: Vec<DomainCardSnapshot>,
}

pub fn apply_combat_input_to_stable_observed_v1(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> CombatObservedStepResultV1;
```

Add `CombatState::take_card_draw_observation_events_v1() -> Vec<DomainCardSnapshot>` beside the existing clear method. It must remove only `DomainEvent::CardDrawn` entries, preserve all other emitted events, and return cards in emission order.

Refactor the stepping loop once so both public APIs share it through an internal `observe_draws: bool` path. Clear any pre-existing draw observations from the cloned input position before the first engine tick, so opening-hand events cannot be misattributed to the first chosen action. At every return boundary, the observed path takes ordered draws while the ordinary path only clears them; the ordinary path must not clone draw snapshots or allocate a draw vector. The old `apply_combat_input_to_stable` returns the same `CombatStepResult` shape and behavior. An already-expired deadline returns an empty draw list.

- [ ] **Step 3: Add witness replay V1**

Keep `replay_combat_search_witness_line_v0` unchanged. Add serializable V1 evidence:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatSearchV2WitnessReplayV1 {
    pub terminal: CombatTerminal,
    pub replayed_actions: usize,
    pub steps: Vec<CombatSearchV2WitnessReplayStepV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatSearchV2WitnessReplayStepV1 {
    pub action_index: usize,
    pub action: ClientInput,
    pub drawn_cards: Vec<DomainCardSnapshot>,
    pub terminal: CombatTerminal,
    pub player_hp: i32,
}

pub fn replay_combat_search_witness_line_v1(
    start: &CombatPosition,
    line: &CombatSearchV2WitnessLine,
    max_engine_steps_per_action: usize,
) -> Result<CombatSearchV2WitnessReplayV1, String>;
```

Replay must reject truncated/timed-out steps, illegal divergence, or a final terminal different from the witness terminal. Re-export the V1 API from `ai::combat_search_v2`.

- [ ] **Step 4: Verify and commit**

Run:

```powershell
cargo test --lib observed_step
cargo test --lib witness_replay
git diff --check
git add src/sim/combat.rs src/runtime/combat/state.rs src/ai/combat_search_v2/witness_guidance.rs src/ai/combat_search_v2/mod.rs
git commit -m "feat: capture combat replay draw evidence"
```

### Task 4: Compile isolated samples and classify one matrix cell

**Files:**

- Create: `src/eval/combat_lab_v1/scenario.rs`
- Create: `src/eval/combat_lab_v1/replay.rs`
- Modify: `src/eval/combat_lab_v1/mod.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Modify: `src/ai/combat_search_v2/outcome_score.rs`
- Modify: `src/ai/combat_search_v2/trajectory_report.rs`
- Modify: `src/ai/combat_search_v2/types/summary.rs`
- Modify: `src/ai/combat_search_v2/types/report/core.rs`
- Modify: `src/eval/run_control/combat_case_retained_candidates.rs` (test-only trajectory literal compatibility)
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs` (test-only trajectory literal compatibility)

**Interfaces:**

- Consumes: Tasks 1-3's resolved contract, controlled start compiler, exact search report, and replay V1.
- Produces: `CombatLabScenarioCompilerV1`, `CombatLabCompiledSampleV1`, stable cell keys, `CombatLabCellRecordV1`, and the serialized exact outcome-order key.

- [ ] **Step 1: Write failing scenario and cell tests**

Add tests that prove:

1. one sample is compiled once and two profiles receive equal clones;
2. mutating/running profile A cannot change profile B's start;
3. the sample invariant fingerprint changes only in the shuffle component relative to the base;
4. node/time/frontier-limited unresolved searches classify as `coverage_limited`, never `resolved_loss`;
5. a replayed win classifies as `resolved_win` and carries HP/action/draw data;
6. replay mismatch classifies as an invariant execution error that tells the runner to halt;
7. cell keys are stable and include experiment hash, sample index, shuffle seed, profile ID/hash, and budget hash.
8. the serialized outcome-order key compares trajectories in the same order as the internal `CombatOutcomeScore`.

Run:

```powershell
cargo test --lib combat_lab_v1::tests::sample_is_shared_across_profiles
```

- [ ] **Step 2: Implement compiled sample evidence**

Define:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCompiledSampleV1 {
    pub sample_index: u64,
    pub shuffle_seed: u64,
    pub start: CombatPosition,
    pub state_fingerprint: StateFingerprintV1,
    pub non_shuffle_rng_hash: String,
    pub shuffle_rng_hash: String,
    pub monster_snapshot_hash: String,
}

pub struct CombatLabScenarioCompilerV1 {
    resolved: ResolvedCombatLabSpecV1,
    baseline: CombatPosition,
    baseline_non_shuffle_rng_hash: String,
    baseline_monster_snapshot_hash: String,
}

pub fn preflight_combat_lab_scenario_v1(
    resolved: &ResolvedCombatLabSpecV1,
) -> Result<CombatLabScenarioCompilerV1, String>;

impl CombatLabScenarioCompilerV1 {
    pub fn compile_sample(
        &self,
        sample_index: u64,
    ) -> Result<CombatLabCompiledSampleV1, String>;
}
```

Preflight compiles the no-override baseline exactly once. Each sample compilation derives its shuffle seed and builds only the overridden start, then compares monster identities, HP and plans plus all non-shuffle RNG fields with the retained baseline. Return an error naming the first unequal invariant field. `start` is the stable sample position cloned for profiles; the baseline is never offered to a profile.

- [ ] **Step 3: Implement raw cell records**

Define these result classes separately from search labels:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabOutcomeClassV1 {
    ResolvedWin,
    ResolvedLoss,
    CoverageLimited,
    ExecutionError,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabCellErrorStageV1 {
    SampleConstruction,
    Search,
    ExactReplay,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCellErrorV1 {
    pub stage: CombatLabCellErrorStageV1,
    pub code: String,
    pub message: String,
    pub halt_experiment: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCellRecordV1 {
    pub schema_version: u32,
    pub cell_key: String,
    pub experiment_hash: String,
    pub sample_index: u64,
    pub shuffle_seed: u64,
    pub profile_id: String,
    pub profile_hash: String,
    pub budget_hash: String,
    pub initial_state_fingerprint: StateFingerprintV1,
    pub non_shuffle_rng_hash: String,
    pub shuffle_rng_hash: String,
    pub search_terminal: Option<SearchTerminalLabel>,
    pub coverage_status: Option<SearchCoverageStatus>,
    pub outcome_class: CombatLabOutcomeClassV1,
    pub outcome_order_key: Option<CombatSearchV2OutcomeOrderKeyReport>,
    pub replay_validated: bool,
    pub start_hp: i32,
    pub final_hp: Option<i32>,
    pub hp_loss: Option<i32>,
    pub turns: Option<u32>,
    pub actions: Option<usize>,
    pub cards_played: Option<u32>,
    pub potions_used: Option<u32>,
    pub draw_history: Vec<DomainCardSnapshot>,
    pub action_history: Vec<ClientInput>,
    pub expanded_nodes: u64,
    pub generated_nodes: u64,
    pub nodes_to_first_win: Option<u64>,
    pub node_budget_exhausted: bool,
    pub deadline_exhausted: bool,
    pub error: Option<CombatLabCellErrorV1>,
}

pub fn combat_lab_cell_key_v1(
    experiment_hash: &str,
    sample_index: u64,
    shuffle_seed: u64,
    profile_id: &str,
    profile_hash: &str,
    budget_hash: &str,
) -> String;
```

Use the existing complete-trajectory report ordering and search report; do not invent another combat evaluator. Select the same complete trajectory that the report presents as best. Exact-replay it with V1 before accepting resolved metrics.

To make that ordering reproducible from the raw journal, expose the exact key already computed by `CombatOutcomeScore`:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CombatSearchV2OutcomeOrderKeyReport {
    pub terminal_rank: i32,
    pub run_hygiene: i32,
    pub persistent_adjusted_hp: i32,
    pub final_hp: i32,
    pub persistent_run_value: i32,
    pub potion_conservation: i32,
    pub faster_turns: i32,
    pub fewer_cards_played: i32,
    pub enemy_progress: i32,
    pub shorter_line: i32,
}
```

Attach it to `CombatSearchV2TrajectoryReport`, and prove it compares identically to `CombatOutcomeScore` in an AI-unit test. Bump `COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION` because the serialized report shape changes. The laboratory stores this key only for exact-replayed resolved trajectories; pair summaries compare these keys directly.

Classification order is strict. Search fields are `Some` whenever search returned a report; they are `None` only for a recorded sample-construction/invariant error that happened before search:

1. replay/invariant failure -> `ExecutionError`;
2. coverage-limited status or unresolved/no complete trajectory -> `CoverageLimited`;
3. exact-replayed terminal win -> `ResolvedWin`;
4. exact-replayed terminal loss under non-limited coverage -> `ResolvedLoss`.

- [ ] **Step 4: Verify and commit**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::sample
cargo test --lib combat_lab_v1::tests::cell
git diff --check
git add src/eval/combat_lab_v1 src/ai/combat_search_v2/outcome_score.rs src/ai/combat_search_v2/trajectory_report.rs src/ai/combat_search_v2/types/summary.rs src/ai/combat_search_v2/types/report/core.rs
git commit -m "feat: execute combat laboratory cells"
```

### Task 5: Make artifacts durable and resume-safe

**Files:**

- Create: `src/eval/combat_lab_v1/artifact.rs`
- Modify: `src/eval/combat_lab_v1/mod.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

**Interfaces:**

- Consumes: Task 2's resolved experiment and Task 4's cell records.
- Produces: `CombatLabManifestV1`, `CombatLabCheckpointV1`, and `CombatLabArtifactStoreV1` as the sole manifest/journal/checkpoint/summary owner.

- [ ] **Step 1: Write failing persistence tests**

Use a unique directory below `std::env::temp_dir()` and remove it at test end. Tests must cover:

- a new run writes `manifest.json` before any cell;
- appending then reopening preserves cells and never duplicates a cell key;
- a partial final JSONL line without a newline is ignored and that exact cell remains pending;
- malformed JSON in any newline-terminated journal entry is an error;
- checkpoint digest disagreement is repaired from the valid journal, not trusted over it;
- a profile/budget/scenario/code-identity mismatch reports the differing field and refuses resume;
- increasing requested samples keeps old cell keys and returns only new/missing work;
- decreasing requested samples deletes nothing;
- atomic JSON replacement works when the destination already exists on Windows.

Run:

```powershell
cargo test --lib combat_lab_v1::tests::resume_does_not_duplicate_cells
```

- [ ] **Step 2: Define the manifest, checkpoint, and store API**

Define:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabManifestV1 {
    pub schema_version: u32,
    pub experiment_hash: String,
    pub resolved_spec: ResolvedCombatLabSpecV1,
    pub source_identity: SourceIdentity,
    pub environment: CombatLabEnvironmentV1,
    pub created_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatLabEnvironmentV1 {
    pub package_version: String,
    pub target_os: String,
    pub target_arch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCheckpointV1 {
    pub schema_version: u32,
    pub journal_digest: String,
    pub completed_cell_keys: BTreeSet<String>,
    pub next_sample_hint: u64,
}

pub struct CombatLabArtifactStoreV1 {
    root: PathBuf,
    manifest: CombatLabManifestV1,
    cells: Vec<CombatLabCellRecordV1>,
    completed_cell_keys: BTreeSet<String>,
    valid_journal_bytes: Vec<u8>,
}

impl CombatLabArtifactStoreV1 {
    pub fn create_or_resume(
        output_dir: &Path,
        expected_manifest: CombatLabManifestV1,
    ) -> Result<Self, String>;

    pub fn manifest(&self) -> &CombatLabManifestV1;
    pub fn cells(&self) -> &[CombatLabCellRecordV1];
    pub fn contains_cell(&self, cell_key: &str) -> bool;
    pub fn append_cell(&mut self, cell: &CombatLabCellRecordV1) -> Result<(), String>;
    pub fn checkpoint_sample_boundary(&self, next_sample_hint: u64) -> Result<(), String>;
    pub fn write_summary<T: Serialize>(&self, summary: &T) -> Result<(), String>;
}
```

Map the store to exactly four root-relative files: `manifest.json`, `cells.jsonl`, `checkpoint.json`, and `summary.json`. Do not introduce per-cell files, an alternate checkpoint authority, or a second journal.

Manifest construction is separate and explicit:

```rust
impl CombatLabManifestV1 {
    pub fn from_resolved_v1(
        resolved_spec: ResolvedCombatLabSpecV1,
        source_identity: SourceIdentity,
        created_at_unix_ms: u64,
    ) -> Self;
}
```

`from_resolved_v1` fills `environment` from the running package/target constants; callers do not supply or override it.

Write manifest/checkpoint/summary through a uniquely named sibling temporary file and call `File::sync_all` before replacement. On Unix replace with same-directory `fs::rename`. On Windows add the target-specific direct dependency `windows-sys = { version = "0.61", features = ["Win32_Storage_FileSystem"] }` and use `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`; do not remove the destination first and create a crash window. Open `cells.jsonl` with append mode, write one compact JSON record plus `\n`, call `sync_data`, and return success only afterward. Compute checkpoint digest from the exact valid journal bytes. The journal remains authoritative.

Resume compares all immutable fields, including Git commit, dirty flag, package version, target OS, and target architecture. Fill environment fields from `env!("CARGO_PKG_VERSION")`, `std::env::consts::OS`, and `std::env::consts::ARCH`. `created_at_unix_ms` and the requested target sample count are not part of identity comparison. Obtain the timestamp from `SystemTime::now().duration_since(UNIX_EPOCH)`; do not add a date/time dependency.

- [ ] **Step 3: Verify and commit**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::artifact
cargo test --lib combat_lab_v1::tests::resume
git diff --check
git add src/eval/combat_lab_v1 Cargo.toml Cargo.lock
git commit -m "feat: persist resumable combat laboratory artifacts"
```

### Task 6: Derive deterministic descriptive summaries

**Files:**

- Create: `src/eval/combat_lab_v1/summary.rs`
- Modify: `src/eval/combat_lab_v1/mod.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`

**Interfaces:**

- Consumes: Task 5's manifest and Task 4's raw cell journal records only.
- Produces: `summarize_combat_lab_v1(&CombatLabManifestV1, &[CombatLabCellRecordV1], u64) -> Result<CombatLabSummaryV1, String>` and deterministic profile/pair/interaction reports.

- [ ] **Step 1: Write failing aggregation tests**

Build raw cell fixtures directly as typed structs. Cover:

- counts and both win-rate denominators;
- win HP-loss mean, population standard deviation, median, nearest-rank p90;
- resolved terminal-HP mean, population standard deviation, median, nearest-rank p10 with losses at zero HP;
- coverage-limited and execution-error cells excluded from resolved distributions;
- pair tables using only shared sample indices and reporting incomplete pairs;
- first action and first draw divergence indices;
- balanced `2 samples x 2 profiles` terminal-HP decomposition with known sums of squares;
- omission of decomposition with a precise reason on an unbalanced resolved matrix;
- byte-identical compact JSON for two regenerations from equal manifest/cells/target.

Run:

```powershell
cargo test --lib combat_lab_v1::tests::summary_separates_coverage_from_loss
```

- [ ] **Step 2: Define the summary surface**

Implement:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabSummaryV1 {
    pub schema_version: u32,
    pub experiment_hash: String,
    pub requested_samples: u64,
    pub completed_cells: usize,
    pub profiles: Vec<CombatLabProfileSummaryV1>,
    pub pairs: Vec<CombatLabPairSummaryV1>,
    pub interaction: Option<CombatLabInteractionSummaryV1>,
    pub interaction_omitted_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabProfileSummaryV1 {
    pub profile_id: String,
    pub requested_cells: u64,
    pub completed_cells: usize,
    pub resolved_cells: usize,
    pub wins: usize,
    pub losses: usize,
    pub coverage_limited: usize,
    pub errors: usize,
    pub win_rate_all_non_error: Option<f64>,
    pub win_rate_all_non_error_denominator: usize,
    pub win_rate_resolved: Option<f64>,
    pub win_rate_resolved_denominator: usize,
    pub hp_loss_mean: Option<f64>,
    pub hp_loss_stddev_population: Option<f64>,
    pub hp_loss_median: Option<f64>,
    pub hp_loss_p90_nearest_rank: Option<i32>,
    pub terminal_hp_mean: Option<f64>,
    pub terminal_hp_stddev_population: Option<f64>,
    pub terminal_hp_median: Option<f64>,
    pub terminal_hp_p10_nearest_rank: Option<i32>,
    pub turns: CombatLabNumericSummaryV1,
    pub potions_used: CombatLabNumericSummaryV1,
    pub expanded_nodes: CombatLabNumericSummaryV1,
    pub deadline_exhaustion_rate: Option<f64>,
    pub node_budget_exhaustion_rate: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabNumericSummaryV1 {
    pub count: usize,
    pub mean: Option<f64>,
    pub stddev_population: Option<f64>,
    pub median: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabPairSummaryV1 {
    pub left_profile_id: String,
    pub right_profile_id: String,
    pub shared_samples: usize,
    pub incomplete_pair_samples: usize,
    pub both_win: usize,
    pub left_only_win: usize,
    pub right_only_win: usize,
    pub both_loss: usize,
    pub unresolved_or_error: usize,
    pub comparable_resolved_samples: usize,
    pub final_hp_delta_left_minus_right: CombatLabNumericSummaryV1,
    pub hp_loss_delta_left_minus_right: CombatLabNumericSummaryV1,
    pub left_strictly_better: usize,
    pub right_strictly_better: usize,
    pub tied: usize,
    pub divergences: Vec<CombatLabPairDivergenceV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabPairDivergenceV1 {
    pub sample_index: u64,
    pub first_action_divergence: Option<usize>,
    pub first_draw_divergence: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabInteractionSummaryV1 {
    pub eligible_samples: usize,
    pub profile_count: usize,
    pub total_sum_squares: f64,
    pub shuffle_sum_squares: f64,
    pub profile_sum_squares: f64,
    pub interaction_sum_squares: f64,
    pub shuffle_share: Option<f64>,
    pub profile_share: Option<f64>,
    pub interaction_share: Option<f64>,
}

pub fn summarize_combat_lab_v1(
    manifest: &CombatLabManifestV1,
    cells: &[CombatLabCellRecordV1],
    requested_samples: u64,
) -> Result<CombatLabSummaryV1, String>;
```

Profile summaries expose explicit fields `hp_loss_mean`, `hp_loss_stddev_population`, `hp_loss_median`, `hp_loss_p90_nearest_rank`, `terminal_hp_mean`, `terminal_hp_stddev_population`, `terminal_hp_median`, and `terminal_hp_p10_nearest_rank`; do not hide tail meaning behind generic lower/upper-tail names.

For sorted `n > 0` values, median is the middle value or arithmetic mean of the two middle values; population standard deviation divides squared deviations by `n`; nearest-rank percentile uses one-based rank `ceil(p * n)` clamped to `1..=n`. Return `None` for every empty distribution and expose its zero denominator.

Sort profiles by manifest order, pairs lexicographically by manifest profile order, and cells by `(sample_index, profile_id)` before aggregation. Do not include generation timestamps in the summary. Reject cells from another experiment hash.

For the optional two-way descriptive decomposition, use only samples having a resolved terminal HP for every profile. With `N` eligible samples, `P` profiles, cell value `y[i,j]`, grand mean `g`, sample mean `s[i]`, and profile mean `p[j]`, compute `SS_shuffle = P * sum((s[i]-g)^2)`, `SS_profile = N * sum((p[j]-g)^2)`, `SS_interaction = sum((y[i,j]-s[i]-p[j]+g)^2)`, and `SS_total = sum((y[i,j]-g)^2)`. Shares divide by `SS_total`; when total variation is zero, return `None` shares rather than NaN. If fewer than two profiles, fewer than two balanced samples, or any balanced set cannot be formed, omit it with the exact condition.

- [ ] **Step 3: Verify and commit**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::summary
cargo test --lib combat_lab_v1::tests::interaction
git diff --check
git add src/eval/combat_lab_v1
git commit -m "feat: summarize combat laboratory evidence"
```

### Task 7: Orchestrate the matrix and add the maintained CLI mode

**Files:**

- Create: `src/eval/combat_lab_v1/runner.rs`
- Modify: `src/eval/combat_lab_v1/mod.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Modify: `src/bin/combat_search_v2_driver/main.rs`

**Interfaces:**

- Consumes: Tasks 2, 4, 5, and 6 through their public library APIs plus `run_combat_search_v2`.
- Produces: `run_combat_lab_v1(&CombatLabRunRequestV1) -> Result<CombatLabRunReportV1, String>` and the maintained `--lab-spec/--lab-output/--lab-samples` CLI mode.

- [ ] **Step 1: Write failing runner tests**

Add a cheap `2 samples x 2 profiles` integration-style library test with tiny budgets. It must invoke the runner twice:

1. first with target 1, producing exactly two distinct cells and a summary;
2. then with target 2, preserving those bytes/keys and appending exactly two cells;
3. third with target 2, appending nothing and regenerating byte-identical `summary.json`.

Add a failure test where the cell executor returns a replay invariant error; assert the error cell is flushed and no later cell runs. Use a private executor trait/function parameter in tests rather than requiring a naturally occurring corrupt replay.

Add driver parser tests for:

- `--lab-spec FILE --lab-output DIR --lab-samples 8` parsing successfully;
- missing `--lab-output` or `--lab-samples` being rejected for lab mode;
- lab mode rejecting benchmark/start/snapshot and compare/guidance/search override flags.
- an output directory outside the repository's ignored `artifacts/runs` root being rejected before files are written.

Run:

```powershell
cargo test --lib combat_lab_v1::tests::runner_resumes_sample_major
cargo test --bin combat_search_v2_driver lab_mode
```

- [ ] **Step 2: Implement sequential sample-major orchestration**

Define:

```rust
pub struct CombatLabRunRequestV1 {
    pub lab_spec_path: PathBuf,
    pub output_dir: PathBuf,
    pub requested_samples: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatLabRunReportV1 {
    pub output_dir: PathBuf,
    pub requested_samples: u64,
    pub cells_present: usize,
    pub cells_appended: usize,
    pub summary: CombatLabSummaryV1,
}

pub fn run_combat_lab_v1(
    request: &CombatLabRunRequestV1,
) -> Result<CombatLabRunReportV1, String>;

trait CombatLabCellExecutorV1 {
    fn execute_cell(
        &self,
        resolved: &ResolvedCombatLabSpecV1,
        sample: &CombatLabCompiledSampleV1,
        profile: &ResolvedCombatLabProfileV1,
    ) -> CombatLabCellRecordV1;
}
```

Production uses a zero-sized exact-search executor. Keep `run_combat_lab_v1_with_executor` private and generic over this trait so tests can deterministically simulate an invariant error without changing the public surface.

Execution order must be:

```text
resolve spec, compile the baseline, and compile/cache sample 0 as preflight
create-or-resume artifact store
for sample_index in 0..requested_samples
    derive all expected cell keys
    if every key exists: continue
    use the cached sample 0 or compile the later sample exactly once; if construction/isolation fails, append one
    execution-error record for the first pending profile with search fields absent, then halt
    for profile in manifest order
        skip an existing cell
        clone sample start
        search, exact-replay, classify
        append and flush the cell
        halt after a replay/invariant execution error
    atomically checkpoint the completed sample boundary
regenerate summary from manifest + journal
atomically write summary
```

Preflight validates profile IDs/hashes and budgets, compiles the no-override baseline once, and compiles sample 0 once before creating the artifact store. Retain that sample for the loop so preflight does not duplicate compilation. For each profile the normal executor calls `run_combat_search_v2(&sample.start.engine, &sample.start.combat, profile_config_v1(&resolved.experiment_id, &profile.spec, &resolved.common_budget))`, converts the selected complete trajectory into a full witness line, and passes it through replay V1 before constructing the cell. It never shells out to Cargo or recursively calls the binary.

- [ ] **Step 3: Add CLI dispatch without duplicating orchestration**

Extend the existing clap input group with `lab_spec`:

```rust
#[arg(long)]
lab_spec: Option<PathBuf>,

#[arg(long, requires = "lab_spec")]
lab_output: Option<PathBuf>,

#[arg(long, requires = "lab_spec")]
lab_samples: Option<u64>,
```

After parsing and validating mutual exclusions, dispatch lab mode before loading any ordinary combat input:

```rust
if let Some(lab_spec_path) = args.lab_spec.as_ref() {
    let report = run_combat_lab_v1(&CombatLabRunRequestV1 {
        lab_spec_path: lab_spec_path.clone(),
        output_dir: args.lab_output.clone().expect("validated by clap"),
        requested_samples: args.lab_samples.expect("validated by clap"),
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    return Ok(());
}
```

Reject `requested_samples == 0`. Lab mode must reject every existing option that would override a profile, budget, compare mode, guidance mode, output file, validation mode, or explanation mode. Keep `lab_output` as a directory distinct from the existing `--output` report file.

Canonicalize the repository root and require the resolved output directory to be a descendant of `<repo>/artifacts/runs`. Capture source identity before creating the directory. This honors the artifact boundary and prevents the experiment's own journal from changing `git_dirty` between creation and resume.

- [ ] **Step 4: Verify and commit**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::runner
cargo test --bin combat_search_v2_driver
git diff --check
git add src/eval/combat_lab_v1 src/bin/combat_search_v2_driver/main.rs
git commit -m "feat: run combat laboratory matrices"
```

### Task 8: Add the seed006-derived pilot, operator docs, and final verification

**Files:**

- Create: `fixtures/combat_lab/seed006_reptomancer_derived.start.json`
- Create: `fixtures/combat_lab/seed006_reptomancer_8x2.lab.json`
- Modify: `docs/RUNBOOK.md`
- Modify: `docs/architecture/supported-surfaces.md`
- Modify: `src/bin/README.md`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Test: `tests/architecture_runtime_boundaries.rs` only if a new boundary assertion is needed to protect the offline-only dependency direction

**Interfaces:**

- Consumes: Task 7's CLI and artifact contract.
- Produces: the maintained seed006-derived start/lab fixtures, supported-surface documentation, optional offline-boundary regression check, and the local `8 x 2` pilot artifact.

- [ ] **Step 1: Add the exact pilot fixtures**

The start fixture must encode:

- name/scenario label: `seed006_derived_reptomancer`;
- Ironclad, ascension 0, elite `Reptomancer`, base seed `20260713006`;
- HP `88/110`;
- deck: Strike x2, Defend x4, Bash+1, Berserk, Clothesline+1, Feed, BattleTrance+1, Armaments+1, ShrugItOff+1, MasterOfStrategy+1, Inflame+1, HeavyBlade;
- relics: BlackBlood, FrozenEgg, OddMushroom, ToxicEgg, RunicPyramid, Courier;
- potion: LiquidBronze.

The lab fixture must use `split_mix64_v1` and two `exact_state_oracle` profiles under one budget:

- `lazy_on_pop`: adaptive rollout, lazy child rollout, semantic potion budget 1, turn plan disabled, round-robin frontier, default phase/setup policies;
- `immediate`: identical except immediate child rollout.

Freeze the pilot common limits at `max_nodes=200000`, `max_actions_per_line=200`, `max_engine_steps_per_action=250`, `wall_ms=3000`, `stop_on_win_hp_loss_at_most=null`, `min_win_candidates_before_stop=1`, `max_potions_used=1`, `rollout_max_evaluations=384`, `rollout_max_actions=80`, `rollout_beam_width=3`, and all three turn-plan probe caps `null`.

Add a fixture-loading test that resolves this real lab spec and asserts its derived label, profile scopes, profile count, and common budget. This validates spelling and supported card/relic/potion IDs without creating a long-lived artifact from an uncommitted source identity.

- [ ] **Step 2: Document the supported surface**

Document:

- the `8 x 2` invocation and how to resume/extend it by raising `--lab-samples`;
- the four artifact files and journal authority;
- the outcome/coverage distinction;
- the sequential resource model and absence of automatic feedback;
- the derived-seed limitation;
- historical profile behavior: readable artifacts remain valid, but rerunning a removed profile requires its recorded commit.

Update the existing Combat Search V2 driver row in supported surfaces; do not describe the lab as a new binary or live run-control component.

If the existing architecture test cannot detect a future forbidden live dependency, add a narrow assertion that files under run-control/route/acquisition do not reference `combat_lab_v1`. Do not add broad source-text assertions for incidental implementation names.

- [ ] **Step 3: Run focused and completion verification**

Verify the real fixture and the entire completion boundary before the final maintained-source commit:

```powershell
cargo test --lib combat_lab_v1::tests::seed006_derived_fixture_resolves
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
cargo test --bin combat_search_v2_driver
git diff --check
```

- [ ] **Step 4: Commit the maintained surface and run the accepted pilot**

Commit fixtures, docs, and the optional architecture assertion only after Step 3 is green. This ordering satisfies the repository completion gate and is also required because resume freezes Git commit and dirty state.

```powershell
git add fixtures/combat_lab docs/RUNBOOK.md docs/architecture/supported-surfaces.md src/bin/README.md src/eval/combat_lab_v1/tests.rs
git add tests/architecture_runtime_boundaries.rs
git commit -m "docs: add combat laboratory pilot workflow"
git status --short
```

Run the second `git add` only if the architecture test changed. Require an empty status before starting the durable experiment.

Start with one sample to inspect the artifact contract:

```powershell
cargo run --bin combat_search_v2_driver -- --lab-spec fixtures/combat_lab/seed006_reptomancer_8x2.lab.json --lab-output artifacts/runs/combat-lab-seed006-pilot --lab-samples 1
```

Inspect all four artifact files. Confirm the manifest says `seed006_derived` and `exact_state_oracle`, records a clean source identity at the Task 8 commit, and does not imply exact campaign history or human-visible information. Then extend the same directory to eight samples:

```powershell
cargo run --bin combat_search_v2_driver -- --lab-spec fixtures/combat_lab/seed006_reptomancer_8x2.lab.json --lab-output artifacts/runs/combat-lab-seed006-pilot --lab-samples 8
```

Verify with read-only commands:

```powershell
(Get-Content artifacts/runs/combat-lab-seed006-pilot/cells.jsonl).Count
Get-Content artifacts/runs/combat-lab-seed006-pilot/summary.json
git status --short
```

Acceptance evidence is sixteen unique journal cells unless an exact-replay/invariant error correctly halts the experiment. Coverage-limited cells still count as recorded evidence. Do not turn the observed seed006 outcome or HP into a regression assertion.

The ignored `artifacts/runs` output remains local evidence and is not committed.

## Final self-review checklist

- Every implemented type and artifact schema is explicitly versioned V1.
- Sample count is an execution bound and absent from immutable experiment identity.
- One sample position is compiled once and cloned across profiles.
- Only `shuffle_rng` changes at natural-start construction.
- Exact-state profiles are labeled as oracles.
- Coverage limits, losses, and infrastructure errors remain distinct.
- Exact replay validates every resolved metric and captures ordered draw/action histories.
- Journal data is authoritative; checkpoint and summary are rebuildable.
- Resume never substitutes profile implementations or silently accepts a new Git identity.
- Summary fields state denominators and percentile conventions.
- No live policy imports or reads laboratory output.
- The seed006 fixture is explicitly derived, and no permanent win/HP behavioral assertion is added.
