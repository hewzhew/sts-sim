# Route Reliability Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make route selection reliability-first by accumulating visible HP pressure along one real continuation and separating campfire recovery from shop liquidity.

**Architecture:** `route_window_facts` becomes the single owner of visible path-family enumeration. `route_planner_v1` derives aggregate evidence plus per-suffix viability from that family, evaluates each suffix independently, and exposes the selected suffix's risk evidence through the existing typed trace and map packet.

**Tech Stack:** Rust 2021, Serde/serde_json, existing route planner and run-control decision packet types, Cargo unit and architecture tests.

## Global Constraints

- Do not add `route_planner_v2` or a second scene-local map traversal.
- Do not add encounter-name, seed, Snake Plant, or Book of Stabbing special cases.
- Keep room-loss estimates explicitly uncalibrated; do not introduce combat search or learned loss models.
- A shop is liquidity, not guaranteed recovery, and must not stop HP accumulation.
- A campfire ends only the current danger projection; it does not force the campfire owner to rest.
- Candidate reward and risk terms must come from one observed suffix; only flexibility may use family-level alternatives.
- Do not lock exact heuristic totals in tests or run a full seed as a unit regression.
- Preserve typed candidate ordering and run-control's action-only boundary.

---

### Task 1: Share candidate-scoped visible path families

**Files:**
- Modify: `src/ai/route_window_facts/mod.rs`
- Modify: `src/ai/route_planner_v1/features/path_summary.rs`

**Interfaces:**
- Produces: `RouteWindowPath`, `RouteWindowPathFamily`, and `build_route_path_family_from_target(run_state, x, y, config)`.
- Produces: `summarize_route_path_family(family)` while preserving `summarize_route_from` for current non-planner consumers.

- [ ] **Step 1: Write failing route-window tests**

Add a test for the wished-for candidate API:

```rust
let family = build_route_path_family_from_target(
    &run_state,
    0,
    0,
    RouteWindowFactsConfig { horizon_nodes: 3, path_budget: 16 },
);
assert_eq!(family.paths.len(), 1);
assert_eq!(
    family.paths[0].nodes.iter().map(|node| node.room_type).collect::<Vec<_>>(),
    vec![
        Some(RoomType::MonsterRoom),
        Some(RoomType::ShopRoom),
        Some(RoomType::MonsterRoomElite),
    ]
);
assert_eq!(family.coverage.kind, RouteWindowCoverageKind::CompleteWithinHorizon);
```

Add a second assertion that `path_budget = 1` on a two-suffix graph reports
`PartialPathBudget`.

- [ ] **Step 2: Run and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_window_facts::tests -- --nocapture
```

Expected: compilation fails because the candidate path-family API does not exist.

- [ ] **Step 3: Implement the shared family**

Promote the existing private path and add:

```rust
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowPath {
    pub nodes: Vec<RouteWindowNode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowPathFamily {
    pub coverage: RouteWindowCoverage,
    pub paths: Vec<RouteWindowPath>,
}

pub fn build_route_path_family_from_target(
    run_state: &RunState,
    x: i32,
    y: i32,
    config: RouteWindowFactsConfig,
) -> RouteWindowPathFamily;
```

Make `build_route_window_facts` derive its facts from the same internal family
builder. Add `summarize_route_path_family` and make the existing summary builder
delegate to a horizon-15 candidate family.

- [ ] **Step 4: Run and verify GREEN**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_window_facts::tests -- --nocapture
cargo test --lib ai::route_planner_v1::tests -- --nocapture
```

- [ ] **Step 5: Commit**

```powershell
git add -- src/ai/route_window_facts/mod.rs src/ai/route_planner_v1/features/path_summary.rs
git commit -m "refactor: share route path families"
```

---

### Task 2: Derive pure suffix viability

**Files:**
- Create: `src/ai/route_planner_v1/features/viability.rs`
- Modify: `src/ai/route_planner_v1/features.rs`
- Modify: `src/ai/route_planner_v1/types/features.rs`
- Modify: `src/ai/route_planner_v1/types.rs`

**Interfaces:**
- Consumes: ordered `RouteWindowPath`, current HP, unknown belief, and planner config.
- Produces: `RoutePathViabilityV1` and `project_route_path_viability`.

- [ ] **Step 1: Write failing invariant tests**

Use real `RouteWindowNode` sequences to assert:

```rust
let direct = viability(44, &[RoomType::MonsterRoomElite]);
let hallway_then_elite = viability(44, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);
assert!(hallway_then_elite.cumulative_hp_loss_p90 >= direct.cumulative_hp_loss_p90);
assert!(hallway_then_elite.projected_hp_after_segment <= direct.projected_hp_after_segment);

let low = viability(30, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);
let high = viability(60, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);
assert!(!low.survives_projected_segment || high.survives_projected_segment);

let shop = viability(44, &[RoomType::MonsterRoom, RoomType::ShopRoom, RoomType::MonsterRoomElite]);
assert_eq!(shop.cumulative_hp_loss_p90, 54.0);
assert!(shop.shop_seen_before_segment_end);

let fire = viability(44, &[RoomType::MonsterRoom, RoomType::RestRoom, RoomType::MonsterRoomElite]);
assert_eq!(fire.cumulative_hp_loss_p90, 14.0);
assert!(fire.campfire_reached_before_elite);
```

- [ ] **Step 2: Run and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_planner_v1::features::viability::tests -- --nocapture
```

Expected: compilation fails because the viability module and type do not exist.

- [ ] **Step 3: Implement the pure projector**

```rust
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RoutePathViabilityV1 {
    pub cumulative_hp_loss_p90: f32,
    pub projected_hp_after_segment: f32,
    pub elite_included_before_recovery: bool,
    pub campfire_reached_before_elite: bool,
    pub shop_seen_before_segment_end: bool,
    pub survives_projected_segment: bool,
}
```

Accumulate 14/40/60 for known hallway/elite/boss rooms and the existing
unknown-room estimate. Stop at the first campfire before charging later rooms;
include and stop after the first elite or boss. A shop records liquidity and
continues. Survival requires projected HP above zero.

- [ ] **Step 4: Run and verify GREEN**

Run the Task 2 focused command again.

- [ ] **Step 5: Commit**

```powershell
git add -- src/ai/route_planner_v1/features.rs src/ai/route_planner_v1/features/viability.rs src/ai/route_planner_v1/types.rs src/ai/route_planner_v1/types/features.rs
git commit -m "feat: project route suffix viability"
```

---

### Task 3: Evaluate one real continuation per candidate

**Files:**
- Modify: `src/ai/route_planner_v1/policy.rs`
- Modify: `src/ai/route_planner_v1/risk.rs`
- Modify: `src/ai/route_planner_v1/scorer.rs`
- Modify: `src/ai/route_planner_v1/types/features.rs`
- Modify: `src/ai/route_planner_v1/types/trace.rs`
- Modify: `src/ai/route_planner_v1/tests/scoring.rs`

**Interfaces:**
- Produces: `RouteCandidateViabilityV1` with family coverage, path counts, representative path index, representative summary, and representative viability.
- Changes: candidate score, reasons, and safety use the representative suffix; aggregate `path_summary` remains evidence only.

- [ ] **Step 1: Write failing behavior tests**

Add a 44-HP fixture with one target continuing `Monster -> Elite` and another
reaching `Rest`. Require the first candidate to expose 54 projected HP loss and
zero surviving paths, and require the rest candidate to be selected.

Add a fork where one suffix has only an elite and another only a campfire.
Require that the representative value factors cannot include both future elite
relic access and future campfire heal access.

- [ ] **Step 2: Run and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_planner_v1::tests::scoring -- --nocapture
```

Expected: compilation fails because candidate viability is absent.

- [ ] **Step 3: Implement per-suffix evaluation**

```rust
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RouteCandidateViabilityV1 {
    pub coverage_complete: bool,
    pub observed_path_count: usize,
    pub surviving_path_count: usize,
    pub representative_path_index: Option<usize>,
    pub representative: Option<RoutePathViabilityV1>,
    pub representative_path_summary: Option<RoutePathSummaryV1>,
}
```

Build one shared family per target. Evaluate each suffix independently and pick
the representative by safety, score, then stable path index. Reject a
non-surviving suffix. If coverage is partial and no observed suffix survives,
downgrade the candidate conclusion to risky instead of claiming rejection.
Remove shop bailout from elite HP safety. Pass family path count separately to
flexibility; use only the representative summary for reward-access terms.

- [ ] **Step 4: Run and verify GREEN**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_planner_v1::tests -- --nocapture
```

- [ ] **Step 5: Commit**

```powershell
git add -- src/ai/route_planner_v1
git commit -m "feat: select viable route continuations"
```

---

### Task 4: Expose typed reliability evidence and verify

**Files:**
- Modify: `src/ai/route_planner_v1/types/map_packet.rs`
- Modify: `src/ai/route_planner_v1/render.rs`
- Modify: `src/ai/route_planner_v1/tests/trace_contract.rs`

**Interfaces:**
- Consumes: `RouteCandidateViabilityV1`.
- Produces: route trace schema version 3 and map packet schema version 2 with typed reliability evidence.

- [ ] **Step 1: Write failing contract tests**

```rust
assert_eq!(trace.schema_version, 3);
assert_eq!(packet.schema_version, 2);
let selected = &packet.candidates[packet.selected_index.unwrap()];
assert!(selected.projection.viability.representative.is_some());
assert!(serialized.contains("projected_hp_after_segment"));
assert!(rendered.contains("projected HP after visible danger segment"));
```

Also remove `viability` from serialized legacy candidate data and require
deserialization to use `RouteCandidateViabilityV1::default()`.

- [ ] **Step 2: Run and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::route_planner_v1::tests::trace_contract -- --nocapture
```

- [ ] **Step 3: Thread evidence**

Add defaulted viability fields to `RouteCandidateTraceV1` and
`RouteProjectionFrontierV1`, copy them in the map packet conversion, advance the
numeric schema versions, and render concise coverage/path-count/projected-HP
lines. Keep all calculations in the AI layer.

- [ ] **Step 4: Run full verification**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo fmt -- --check
cargo test --lib ai::route_window_facts::tests -- --nocapture
cargo test --lib ai::route_planner_v1::tests -- --nocapture
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

- [ ] **Step 5: Inspect the preserved seed and commit**

Inspect `target/seed-fix-diagnosis-20260711004` without a full-seed rerun and
confirm that 44 HP minus one hallway p90 and one elite p90 is non-surviving.

```powershell
git add -- docs/superpowers/specs/2026-07-11-route-reliability-projection-design.md docs/superpowers/plans/2026-07-11-route-reliability-projection.md src/ai/route_planner_v1 src/ai/route_window_facts/mod.rs
git commit -m "feat: expose route reliability evidence"
```
