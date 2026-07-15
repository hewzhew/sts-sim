# Campfire Aligned Survival Scenarios Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compile every shuffle-alignable exact Campfire projection into a matched, explicit-encounter combat start while recording chance and deck-identity gaps instead of fabricating survival evidence.

**Architecture:** A new offline-only `campfire_survival_scenarios` module consumes the already-built `CampfireEvaluationBatch`, so legality, exact transitions, and context provenance retain their existing owners. It derives one analysis shuffle seed through Combat Lab's schedule, compiles exact candidates whose stable deck UUID sequence still matches the public root, and records typed gaps for chance suffixes, post-reveal recourse, and Toke-like deck-identity changes. It does not run combat search, calculate a survival distribution, or gain a production reader.

**Tech Stack:** Rust, existing Campfire evaluation/projection types, Combat Lab SplitMix64 analysis schedule, natural combat-start engine, Cargo unit and architecture tests.

## Global Constraints

- Work in the stable checkout; do not create a worktree.
- Keep the module offline-only; do not import it from run-control, owner-audit, route planning, or the legacy Campfire policy.
- Consume `CampfireEvaluationBatch`; do not re-enumerate legality or reapply Campfire mechanics.
- Use `derive_shuffle_seed_v1`; do not read the live shuffle cursor or invent another sampling schedule.
- Call `build_natural_combat_start`; do not reproduce relic, draw, enemy, or combat initialization mechanics.
- Match candidates only when projected master-deck UUID order equals the public root UUID order. Smith upgrades remain alignable; Toke removal does not.
- Record `Chance`, `ChanceThenDecision`, and deck-identity changes as typed gaps. Do not silently omit them or assign zero survival value.
- Label the compiler `ExactStateOracle`; its explicit encounter and non-shuffle RNG state are offline evidence, not information available to the production Campfire owner.
- Do not run combat search, add HP thresholds, fill `SurvivalDistribution`, rank candidates, or change live Campfire behavior in this slice.
- Add only durable scenario-alignment and offline-isolation regressions; do not lock a named seed or preferred Campfire action.
- Run focused tests first, then the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Compile Shuffle-Aligned Exact Candidate Starts

**Files:**

- Create: `src/eval/campfire_survival_scenarios.rs`
- Modify: `src/eval/mod.rs`

**Interfaces:**

- Consumes: `CampfireEvaluationBatch`, `CampfireProjection`, `CombatLabShuffleScheduleV1`, `derive_shuffle_seed_v1`, `build_natural_combat_start`.
- Produces: `compile_aligned_campfire_survival_sample(evaluation: &CampfireEvaluationBatch, spec: CampfireSurvivalScenarioSpec) -> Result<CampfireSurvivalScenarioSample, CampfireSurvivalScenarioError>`.
- Produces: typed scenario cells, `ExactStateOracle` provenance, and typed unresolved gap records.

- [ ] **Step 1: Add the module export and failing scenario tests**

Add to `src/eval/mod.rs`:

```rust
pub mod campfire_survival_scenarios;
```

Create `src/eval/campfire_survival_scenarios.rs` with a `#[cfg(test)]` module that builds a public Campfire evaluation fixture without Dream Catcher:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::engine::campfire_candidates::CampfireCandidate;
    use crate::eval::campfire_evaluation::{
        build_campfire_evaluation_batch, CampfireContinuationProfile,
        CampfireEvaluationHorizon, CampfireEvaluationSpec, CampfireRunGoal,
    };
    use crate::eval::combat_lab_v1::{
        CombatLabShuffleGeneratorV1, CombatLabShuffleScheduleV1,
    };
    use crate::runtime::combat::CombatCard;
    use crate::state::map::node::RoomType;
    use crate::state::run::RunState;

    fn evaluation_spec() -> CampfireEvaluationSpec {
        CampfireEvaluationSpec {
            run_goal: CampfireRunGoal::Act3Victory,
            horizon: CampfireEvaluationHorizon::UntilNextCampfireOrActTerminal {
                route_horizon_nodes: 5,
            },
            route_path_budget: 2_000,
            continuation_profile: CampfireContinuationProfile {
                profile_id: "survival-scenario-test".to_string(),
                source_identity: "test-source".to_string(),
            },
            public_scenario_distribution_id: "explicit-encounter-test".to_string(),
            mechanics_version: "sts-simulator-test-v1".to_string(),
        }
    }

    fn candidate_run() -> RunState {
        let mut run = RunState::new(17, 0, false, "Ironclad");
        run.current_hp = 20;
        run.master_deck = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
        ];
        run.relics = vec![
            RelicState::new(RelicId::Girya),
            RelicState::new(RelicId::Shovel),
            RelicState::new(RelicId::PeacePipe),
        ];
        run
    }

    fn scenario_spec() -> CampfireSurvivalScenarioSpec {
        CampfireSurvivalScenarioSpec {
            encounter_id: EncounterId::JawWorm,
            room_type: RoomType::MonsterRoom,
            schedule: CombatLabShuffleScheduleV1 {
                generator: CombatLabShuffleGeneratorV1::SplitMix64V1,
                seed: 91,
            },
            sample_index: 0,
        }
    }

    #[test]
    fn exact_rest_and_smith_share_one_aligned_natural_combat_scenario() {
        let evaluation =
            build_campfire_evaluation_batch(&candidate_run(), evaluation_spec()).unwrap();
        let sample =
            compile_aligned_campfire_survival_sample(&evaluation, scenario_spec()).unwrap();
        let rest = sample
            .cells
            .iter()
            .find(|cell| cell.candidate == CampfireCandidate::Rest)
            .expect("exact Rest should be compiled");
        let smith = sample
            .cells
            .iter()
            .find(|cell| cell.candidate == CampfireCandidate::Smith { card_uuid: 101 })
            .expect("UUID-preserving Smith should be compiled");

        assert_eq!(sample.information_scope, CampfireSurvivalInformationScope::ExactStateOracle);
        assert_eq!(rest.start.combat.entities.player.current_hp, 44);
        assert_eq!(smith.start.combat.entities.player.current_hp, 20);
        assert_eq!(rest.start.combat.entities.monsters, smith.start.combat.entities.monsters);
        assert_eq!(
            rest.start
                .combat
                .zones
                .hand
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            smith
                .start
                .combat
                .zones
                .hand
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>()
        );
        assert!(sample.gaps.iter().any(|gap| {
            gap.gap == CampfireSurvivalScenarioGap::ChanceOutcomeNotMaterialized
                && gap.candidate == CampfireCandidate::Dig
        }));
        assert!(sample.gaps.iter().any(|gap| {
            gap.gap == CampfireSurvivalScenarioGap::DeckIdentityChanged
                && matches!(gap.candidate, CampfireCandidate::Toke { .. })
        }));
    }

    #[test]
    fn dream_catcher_rest_stays_a_post_reveal_recourse_gap() {
        let mut root = candidate_run();
        root.relics.push(RelicState::new(RelicId::DreamCatcher));
        let evaluation = build_campfire_evaluation_batch(&root, evaluation_spec()).unwrap();
        let sample =
            compile_aligned_campfire_survival_sample(&evaluation, scenario_spec()).unwrap();

        assert!(!sample
            .cells
            .iter()
            .any(|cell| cell.candidate == CampfireCandidate::Rest));
        assert!(sample.gaps.iter().any(|gap| {
            gap.candidate == CampfireCandidate::Rest
                && gap.gap == CampfireSurvivalScenarioGap::PostRevealRecourseNotMaterialized
        }));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
cargo test -p sts_simulator campfire_survival_scenarios --lib
```

Expected: compilation fails because the scenario compiler types and function do not exist.

- [ ] **Step 3: Implement the offline aligned-scenario compiler**

Above the tests in `src/eval/campfire_survival_scenarios.rs`, add:

```rust
use crate::content::monsters::factory::EncounterId;
use crate::engine::campfire_candidates::CampfireCandidate;
use crate::eval::campfire_evaluation::CampfireEvaluationBatch;
use crate::eval::campfire_projection::CampfireProjection;
use crate::eval::combat_lab_v1::{
    derive_shuffle_seed_v1, CombatLabShuffleScheduleV1,
};
use crate::eval::fingerprint::{combat_state_fingerprint_v1, StateFingerprintV1};
use crate::runtime::combat::CombatCard;
use crate::runtime::rng::StsRng;
use crate::sim::combat::CombatPosition;
use crate::sim::combat_start::build_natural_combat_start;
use crate::state::map::node::RoomType;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalInformationScope {
    ExactStateOracle,
}

#[derive(Clone, Debug)]
pub struct CampfireSurvivalScenarioSpec {
    pub encounter_id: EncounterId,
    pub room_type: RoomType,
    pub schedule: CombatLabShuffleScheduleV1,
    pub sample_index: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalScenarioGap {
    ChanceOutcomeNotMaterialized,
    PostRevealRecourseNotMaterialized,
    DeckIdentityChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampfireSurvivalScenarioGapRecord {
    pub candidate: CampfireCandidate,
    pub gap: CampfireSurvivalScenarioGap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSurvivalScenarioCell {
    pub candidate: CampfireCandidate,
    pub shuffle_seed: u64,
    pub start: CombatPosition,
    pub state_fingerprint: StateFingerprintV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSurvivalScenarioSample {
    pub context_fingerprint: String,
    pub information_scope: CampfireSurvivalInformationScope,
    pub encounter_id: EncounterId,
    pub room_type: RoomType,
    pub sample_index: u64,
    pub shuffle_seed: u64,
    pub cells: Vec<CampfireSurvivalScenarioCell>,
    pub gaps: Vec<CampfireSurvivalScenarioGapRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalScenarioError {
    NonCombatRoomType { room_type: RoomType },
    CombatStart {
        candidate: CampfireCandidate,
        message: String,
    },
}

pub fn compile_aligned_campfire_survival_sample(
    evaluation: &CampfireEvaluationBatch,
    spec: CampfireSurvivalScenarioSpec,
) -> Result<CampfireSurvivalScenarioSample, CampfireSurvivalScenarioError> {
    if !matches!(
        spec.room_type,
        RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss
    ) {
        return Err(CampfireSurvivalScenarioError::NonCombatRoomType {
            room_type: spec.room_type,
        });
    }

    let shuffle_seed = derive_shuffle_seed_v1(&spec.schedule, spec.sample_index);
    let root_deck = &evaluation.context.public_root.master_deck;
    let mut cells = Vec::new();
    let mut gaps = Vec::new();

    for candidate in &evaluation.candidates {
        let exact = match &candidate.projection {
            CampfireProjection::Exact(exact) => exact,
            CampfireProjection::Chance(_) => {
                gaps.push(CampfireSurvivalScenarioGapRecord {
                    candidate: candidate.candidate,
                    gap: CampfireSurvivalScenarioGap::ChanceOutcomeNotMaterialized,
                });
                continue;
            }
            CampfireProjection::ChanceThenDecision(_) => {
                gaps.push(CampfireSurvivalScenarioGapRecord {
                    candidate: candidate.candidate,
                    gap: CampfireSurvivalScenarioGap::PostRevealRecourseNotMaterialized,
                });
                continue;
            }
        };
        if !same_deck_identity(root_deck, &exact.run_state.master_deck) {
            gaps.push(CampfireSurvivalScenarioGapRecord {
                candidate: candidate.candidate,
                gap: CampfireSurvivalScenarioGap::DeckIdentityChanged,
            });
            continue;
        }

        let mut projected = exact.run_state.clone();
        projected.rng_pool.shuffle_rng = StsRng::new(shuffle_seed);
        let (engine, combat) = build_natural_combat_start(
            &mut projected,
            spec.encounter_id,
            spec.room_type,
        )
        .map_err(|message| CampfireSurvivalScenarioError::CombatStart {
            candidate: candidate.candidate,
            message,
        })?;
        let start = CombatPosition::new(engine, combat);
        cells.push(CampfireSurvivalScenarioCell {
            candidate: candidate.candidate,
            shuffle_seed,
            state_fingerprint: combat_state_fingerprint_v1(&start),
            start,
        });
    }

    Ok(CampfireSurvivalScenarioSample {
        context_fingerprint: evaluation.context.context_fingerprint.clone(),
        information_scope: CampfireSurvivalInformationScope::ExactStateOracle,
        encounter_id: spec.encounter_id,
        room_type: spec.room_type,
        sample_index: spec.sample_index,
        shuffle_seed,
        cells,
        gaps,
    })
}

fn same_deck_identity(root: &[CombatCard], projected: &[CombatCard]) -> bool {
    root.iter()
        .map(|card| card.uuid)
        .eq(projected.iter().map(|card| card.uuid))
}
```

- [ ] **Step 4: Run formatting and focused tests and verify GREEN**

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo test -p sts_simulator campfire_survival_scenarios --lib
```

Expected: both scenario-alignment tests pass. Rest and Smith share enemy state and UUID draw order; Dig, Dream Catcher Rest, and Toke remain explicit gaps.

- [ ] **Step 5: Commit the compiler slice**

```powershell
git add src/eval/mod.rs src/eval/campfire_survival_scenarios.rs
git commit -m "feat: compile aligned campfire survival scenarios"
```

---

### Task 2: Guard The Offline Boundary

**Files:**

- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**

- Consumes: the source-tree boundary established by `campfire_survival_scenarios`.
- Produces: a durable architecture test that rejects imports of offline laboratories from live decision layers.

- [ ] **Step 1: Extend the live-layer architecture regression**

Rename `live_decision_layers_do_not_depend_on_combat_laboratory` to `live_decision_layers_do_not_depend_on_offline_laboratories`. Add `src/runtime/branch/owner_audit` and `src/ai/campfire_policy_v1` to the scanned roots, then replace the single assertion with:

```rust
for path in sources {
    let source = std::fs::read_to_string(&path).expect("read live decision-layer source");
    for forbidden in ["combat_lab_v1", "campfire_survival_scenarios"] {
        assert!(
            !source.contains(forbidden),
            "live decision layer '{}' must not import or read offline laboratory `{forbidden}`",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Run the architecture filter and verify GREEN**

```powershell
cargo test -p sts_simulator --test architecture_runtime_boundaries live_decision_layers_do_not_depend_on_offline_laboratories
```

Expected: the single architecture regression passes, proving the new module has no live reader.

- [ ] **Step 3: Commit the isolation guard**

```powershell
git add tests/architecture_runtime_boundaries.rs
git commit -m "test: isolate offline campfire survival scenarios"
```

---

### Task 3: Verify The Slice

**Files:** verification only.

**Interfaces:** Verifies the scenario compiler, the existing Campfire evaluation suite, and repository architecture gates without claiming survival outcomes.

- [ ] **Step 1: Run completion suites**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire --lib
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: formatting succeeds and all focused, library, and architecture tests report zero failures.

- [ ] **Step 2: Confirm clean history and no production readers**

```powershell
rg -n "campfire_survival_scenarios" src/runtime/branch/owner_audit src/ai/campfire_policy_v1 src/eval/run_control src/ai/route_planner_v1 src/ai/strategy/acquisition.rs
git status --short --branch
git log -7 --oneline
```

Expected: `rg` returns no matches, Git reports no uncommitted files, and both implementation commits appear above this plan.

