# Campfire Toke Growth Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the deck-mutation compiler's Campfire configuration dependency and add UUID-bound, typed Toke growth evidence without repeating Smith or removal analysis for every candidate.

**Architecture:** The deck-mutation compiler remains the sole producer of typed removal target class and loss facts. Campfire evaluation builds one root-scoped growth-fact index, verifies authoritative Smith and Toke projections by UUID, and copies only typed consequences; it never copies compiler scores, roles, or execution permissions.

**Tech Stack:** Rust, existing deck-mutation compiler, upgrade planner, Campfire projection/evaluation modules, Cargo unit and architecture tests.

## Global Constraints

- Work in the stable checkout; do not create a worktree.
- Keep the existing Campfire policy alive until the later production cutover, but do not add a new dependency on it.
- Bind Smith and Toke candidates by `CombatCard.uuid`; deck index remains display-only.
- Build upgrade and removal fact collections once per Campfire evaluation batch, not once per candidate.
- Do not copy `score_hint`, `DeckMutationPlanRoleV1`, or `AllowedDeckMutationConsumersV1` into the Campfire growth contract.
- Do not rank Campfire candidates, add a production reader, or change existing policy thresholds in this slice.
- Keep non-Smith/non-Toke growth `Unsupported`.
- Run focused tests first, then the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Remove The Deck-Mutation Compiler's Campfire Configuration Dependency

**Files:**

- Modify: `src/ai/deck_mutation_compiler_v1/compiler.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**

- Preserves: the existing numeric `180` upgrade-role threshold and all deck-mutation behavior.
- Removes: the compiler's production reference to `campfire_policy_v1::CampfirePolicyConfigV1` and `clear_core_smith_priority_threshold`.

- [ ] **Step 1: Write the failing architecture test**

Add a source-boundary test:

```rust
#[test]
fn deck_mutation_compiler_does_not_depend_on_campfire_policy_configuration() {
    let source = std::fs::read_to_string("src/ai/deck_mutation_compiler_v1/compiler.rs")
        .expect("read deck mutation compiler");
    for forbidden in ["campfire_policy_v1", "clear_core_smith_priority_threshold"] {
        assert!(!source.contains(forbidden));
    }
}
```

- [ ] **Step 2: Run the architecture filter and verify RED**

```powershell
cargo test -p sts_simulator --test architecture_runtime_boundaries deck_mutation_compiler_does_not_depend
```

Expected: the source guard fails on the current `CampfirePolicyConfigV1` lookup.

- [ ] **Step 3: Move the unchanged threshold to its semantic owner**

Add beside the compiler's other constants:

```rust
const POLICY_PREFERRED_UPGRADE_PRIORITY_THRESHOLD: i32 = 180;
```

Use the constant directly in `evaluate_candidate_for_reason` and delete `clear_upgrade_priority_threshold`. Do not change the threshold or any role logic.

- [ ] **Step 4: Run focused compiler and architecture tests**

```powershell
cargo test -p sts_simulator deck_mutation_compiler_v1 --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries deck_mutation_compiler_does_not_depend
```

Expected: compiler tests and the new boundary test pass.

- [ ] **Step 5: Commit the dependency removal**

```powershell
git add src/ai/deck_mutation_compiler_v1/compiler.rs tests/architecture_runtime_boundaries.rs
git commit -m "refactor: detach deck mutation facts from campfire policy"
```

---

### Task 2: Build One Shared Growth-Fact Index And Add Typed Toke Evidence

**Files:**

- Modify: `src/eval/campfire_evaluation/growth.rs`
- Modify: `src/eval/campfire_evaluation.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**

- Consumes: `plan_upgrades_v1`, `deck_removal_target_snapshots_v1`, `CampfireCandidate::{Smith,Toke}`, and `CampfireProjection::Exact`.
- Produces: internal `CampfireGrowthFacts`, public `CampfireTokeGrowth`, and `CampfireGrowth { smith, toke }`.

- [ ] **Step 1: Write failing Toke and batch-boundary tests**

Extend the Campfire evaluation test to assert:

```rust
let toke = toke.growth.toke.as_ref().unwrap();
assert_eq!(toke.card_uuid, 101);
assert_eq!(toke.card, CardId::Strike);
assert_eq!(toke.deck_size_before, 3);
assert_eq!(toke.deck_size_after, 2);
assert_eq!(toke.target_class, DeckMutationTargetClassV1::StarterStrike);
assert_eq!(toke.target_loss.tier, DeckMutationTargetLossTierV1::LowValue);
assert_eq!(
    candidate.evidence_for(CampfireProspectField::GrowthDistribution)
        .unwrap()
        .status,
    CampfireEvidenceStatus::Partial
);
```

Add an architecture source guard proving `growth.rs` no longer calls `upgrade_candidate_for_card_uuid_v1`, because that helper rebuilds the whole plan per candidate.

- [ ] **Step 2: Run focused tests and verify RED**

```powershell
cargo test -p sts_simulator campfire_evaluation --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries campfire_growth_facts_are_built_once
```

Expected: compilation fails for the missing Toke contract, while the architecture guard fails on the current per-candidate Smith helper call.

- [ ] **Step 3: Build the root-scoped fact index once**

Define an internal index in `growth.rs`:

```rust
pub(super) struct CampfireGrowthFacts {
    smith_by_uuid: HashMap<u32, UpgradeCandidateV1>,
    toke_by_uuid: HashMap<u32, DeckMutationCardSnapshotV1>,
}

pub(super) fn build_growth_facts(root: &RunState) -> CampfireGrowthFacts {
    let smith_by_uuid = plan_upgrades_v1(root)
        .candidates
        .into_iter()
        .map(|candidate| (candidate.card_uuid, candidate))
        .collect();
    let toke_by_uuid = deck_removal_target_snapshots_v1(root)
        .into_iter()
        .map(|candidate| (candidate.uuid, candidate))
        .collect();
    CampfireGrowthFacts {
        smith_by_uuid,
        toke_by_uuid,
    }
}
```

Construct it before the legal-candidate loop in `build_campfire_evaluation_batch`:

```rust
let growth_facts = build_growth_facts(root);
for candidate in legal_campfire_candidates(root) {
    // projection and sibling evidence
    let growth = assess_growth(root, &growth_facts, candidate, &projection)
        .map_err(|source| CampfireEvaluationError::Growth { candidate, source })?;
}
```

Smith reads its cached planner candidate rather than calling `upgrade_candidate_for_card_uuid_v1`.

- [ ] **Step 4: Implement exact typed Toke growth**

Add:

```rust
pub struct CampfireTokeGrowth {
    pub card_uuid: u32,
    pub card: CardId,
    pub upgrades: u8,
    pub deck_size_before: usize,
    pub deck_size_after: usize,
    pub target_class: DeckMutationTargetClassV1,
    pub target_loss: DeckMutationTargetLossV1,
}
```

For `Toke { card_uuid }`, require an exact projection, obtain the cached removal snapshot by UUID, and require the projected master deck to equal the root deck with exactly that UUID removed. Return typed `MissingRemovalTarget` or `RemovalProjectionMismatch` errors on disagreement. Emit `Partial` growth evidence with `EngineTransitionAndDeckMutationCompiler` provenance and `DownstreamGrowthDistributionNotEvaluated` limitation. Preserve current Smith evidence and keep every other family unsupported.

The exact projection check is:

```rust
let expected_deck = root
    .master_deck
    .iter()
    .filter(|card| card.uuid != card_uuid)
    .cloned()
    .collect::<Vec<_>>();
if exact.run_state.master_deck != expected_deck {
    return Err(CampfireGrowthError::RemovalProjectionMismatch { card_uuid });
}
```

- [ ] **Step 5: Run focused Campfire and architecture tests**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries campfire_growth_facts_are_built_once
```

Expected: all focused tests pass, both Smith and Toke expose typed evidence, and no scalar score enters the growth module.

- [ ] **Step 6: Commit typed Toke evidence**

```powershell
git add src/eval/campfire_evaluation.rs src/eval/campfire_evaluation/growth.rs tests/architecture_runtime_boundaries.rs
git commit -m "feat: add typed campfire toke growth evidence"
```

---

### Task 3: Verify The Slice

**Files:** verification only.

**Interfaces:** Verifies the dependency deletion, one-time fact construction, and typed Smith/Toke evidence under the repository completion gate.

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

Expected: no uncommitted changes and both implementation commits appear above this plan.
