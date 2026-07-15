# Campfire Smith Growth Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move durable Smith upgrade facts out of the Rest-versus-Smith structure and expose UUID-bound, typed Smith growth evidence in the offline Campfire evaluation batch.

**Architecture:** The upgrade planner remains the sole producer of upgrade mechanics and strategic upgrade facts. Campfire evaluation verifies the authoritative projected card change, copies typed planner facts without scalar scores, and marks the aggregate growth field partial because downstream outcome value is still absent.

**Tech Stack:** Rust, existing upgrade planner, existing Campfire projection/evaluation modules, Cargo unit and architecture tests.

## Global Constraints

- Work in the stable checkout; do not create a worktree.
- Do not add another upgrade evaluator or recompute upgrade mechanics in `eval`.
- Bind Smith candidates and planner facts by `CombatCard.uuid`; deck index is display-only.
- Do not copy `upgrade_candidate_score_hint_v1` into the Campfire growth contract.
- Keep non-Smith growth `Unsupported` in this slice.
- Do not rank Campfire candidates, call the legacy policy, or add a production reader.
- Keep the legacy Rest-versus-Smith value only as a temporary mirror for the legacy Campfire owner.
- Run focused tests first, then the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Establish Durable UUID-Bound Upgrade Facts

**Files:**

- Modify: `src/ai/upgrade_planner_v1.rs`
- Modify: `src/ai/random_upgrade_opportunity_v1.rs`
- Modify: `src/ai/shop_policy_v1/policy.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**

- Produces: `UpgradeCandidateV1.card_uuid`, `UpgradePlanV1.best_smith_debt_paid`, and `upgrade_candidate_for_card_uuid_v1(&RunState, u32) -> Option<UpgradeCandidateV1>`.
- Preserves: `RestVsSmithPlanV1.best_smith_debt_paid` as a legacy mirror derived from the plan-level fact.

- [ ] **Step 1: Write failing planner and architecture tests**

Add planner tests proving:

```rust
assert_eq!(candidate.card_uuid, 7002);
assert_eq!(
    upgrade_candidate_for_card_uuid_v1(&reordered, 7002)
        .unwrap()
        .card_uuid,
    7002
);
assert_eq!(
    plan.best_smith_debt_paid,
    plan.rest_vs_smith.best_smith_debt_paid
);
```

Add an architecture test reading `random_upgrade_opportunity_v1.rs` and `shop_policy_v1/policy.rs` and asserting neither contains `rest_vs_smith.best_smith_debt_paid`.

- [ ] **Step 2: Run focused tests and verify RED**

```powershell
cargo test -p sts_simulator upgrade_planner --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries durable_upgrade_consumers
```

Expected: compilation fails for missing UUID/top-level fields and the source guard fails while durable readers still use the legacy structure.

- [ ] **Step 3: Implement the durable planner facts**

Add `card_uuid` when `UpgradeCandidateV1` is built. Compute `best_smith_debt_paid` once after candidate sorting, store it on `UpgradePlanV1`, and pass it into `rest_vs_smith_plan` so the legacy field is only a mirror. Implement UUID lookup by filtering the completed plan rather than indexing the live deck.

Update these readers:

```rust
upgrade_plan.best_smith_debt_paid
plan.best_smith_debt_paid
```

Do not change their behavior or thresholds.

- [ ] **Step 4: Run focused planner, shop, random-upgrade, and architecture tests**

```powershell
cargo test -p sts_simulator upgrade_planner --lib
cargo test -p sts_simulator random_upgrade --lib
cargo test -p sts_simulator shop_upgrade_need --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries durable_upgrade_consumers
```

Expected: all focused tests pass and the architecture source guard finds no durable legacy reader.

- [ ] **Step 5: Commit the planner boundary**

```powershell
git add src/ai/upgrade_planner_v1.rs src/ai/random_upgrade_opportunity_v1.rs src/ai/shop_policy_v1/policy.rs tests/architecture_runtime_boundaries.rs
git commit -m "refactor: expose durable smith upgrade facts"
```

---

### Task 2: Produce Typed Smith Growth Evidence

**Files:**

- Create: `src/eval/campfire_evaluation/growth.rs`
- Modify: `src/eval/campfire_evaluation.rs`

**Interfaces:**

- Consumes: `upgrade_candidate_for_card_uuid_v1`, `CampfireCandidate::Smith`, and `CampfireProjection::Exact`.
- Produces: `CampfireGrowth`, `CampfireSmithGrowth`, and `assess_growth(...) -> Result<CampfireGrowthAssessment, CampfireGrowthError>`.

- [ ] **Step 1: Write a failing Smith growth test**

Build a batch with a known Strike UUID and assert:

```rust
let smith = smith.growth.smith.as_ref().unwrap();
assert_eq!(smith.card_uuid, 101);
assert_eq!(smith.upgrades_before, 0);
assert_eq!(smith.upgrades_after, 1);
assert_eq!(smith.mechanical_delta.damage_delta, 3);
assert_eq!(
    candidate.evidence_for(CampfireProspectField::GrowthDistribution)
        .unwrap()
        .status,
    CampfireEvidenceStatus::Partial
);
assert!(rest.growth.smith.is_none());
```

Also assert the Smith record contains no scalar score field by constructing and comparing only its typed fields.

- [ ] **Step 2: Run the Campfire evaluation filter and verify RED**

```powershell
cargo test -p sts_simulator campfire_evaluation --lib
```

Expected: compilation fails because the growth contract and candidate field do not exist.

- [ ] **Step 3: Implement UUID-bound Smith growth assembly**

Define:

```rust
pub struct CampfireGrowth {
    pub smith: Option<CampfireSmithGrowth>,
}

pub struct CampfireSmithGrowth {
    pub card_uuid: u32,
    pub card: CardId,
    pub upgrades_before: u8,
    pub upgrades_after: u8,
    pub mechanical_delta: UpgradeMechanicalDeltaV1,
    pub roles: Vec<UpgradeRoleV1>,
    pub pays_debts: Vec<UpgradeDebtKindV1>,
    pub opportunity_costs: Vec<String>,
    pub urgency: UpgradeDebtSeverityV1,
    pub verdict: UpgradeVerdictV1,
}
```

For Smith, require an exact projection, find the projected card by UUID, and require exactly one upgrade level of change. Missing planner/projection identity or an upgrade-count mismatch returns a typed `CampfireGrowthError` and blocks batch construction. Emit `Partial` evidence with `EngineTransitionAndUpgradePlanner` provenance and `DownstreamGrowthDistributionNotEvaluated`. For every non-Smith candidate, keep the existing unsupported growth evidence and no Smith record.

- [ ] **Step 4: Run focused Campfire tests and verify GREEN**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire --lib
```

Expected: all Campfire evaluation, projection, engine, and legacy policy tests pass.

- [ ] **Step 5: Commit Smith growth evidence**

```powershell
git add src/eval/campfire_evaluation.rs src/eval/campfire_evaluation/growth.rs
git commit -m "feat: add typed smith growth evidence"
```

---

### Task 3: Verify The Slice

**Files:** verification only.

**Interfaces:** Verifies all durable upgrade readers and offline Campfire evidence under the repository completion gate.

- [ ] **Step 1: Run completion suites**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: formatting succeeds and both suites report zero failures.

- [ ] **Step 2: Confirm clean history**

```powershell
git status --short --branch
git log -7 --oneline
```

Expected: no uncommitted changes and the durable-upgrade and Smith-growth commits appear above this plan.

