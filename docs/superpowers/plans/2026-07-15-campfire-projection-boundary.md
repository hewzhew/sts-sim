# Campfire Projection Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Project deterministic Campfire candidates through the real engine on cloned state while representing Dig and Dream Catcher as non-peeking chance and recourse boundaries.

**Architecture:** Extract the exact Rest healing prefix from `campfire_handler` so both execution and analysis share one mechanic. Add an offline-only `eval::campfire_projection` module: deterministic candidates call the real handler on clones and must leave RNG unchanged; Dig returns a chance descriptor without drawing a relic; Rest with Dream Catcher returns exact healing plus a post-reveal card-reward recourse descriptor without generating cards.

**Tech Stack:** Rust 2021, Cargo, existing `RunState`/`EngineState` cloning, `CampfireCandidate`, `campfire_handler`, `RngPool`, and serde-derived engine state.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Do not run `cargo clean`.
- Do not wire projections into run-control, owner-audit, or `campfire_policy_v1` in this slice.
- Do not draw from, advance, clone-and-preview, or report the live run RNG for Dig or Dream Catcher.
- Do not replace the original mutable RNG streams with event-keyed randomness.
- Deterministic projection must use the real Campfire handler and reject unexpected RNG mutation.
- Dream Catcher is exact healing followed by chance and a post-reveal card-reward decision, not a scalar reward.
- Use focused tests during red/green work, then run the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Extract The Exact Rest Healing Kernel

**Files:**
- Modify: `src/engine/campfire_handler.rs`

**Interfaces:**
- Produces: `apply_campfire_rest_healing(&mut RunState)` as the exact healing prefix.
- Preserves: Rest healing, Regal Pillow, Mark of the Bloom, and Dream Catcher reward-screen behavior.

- [ ] **Step 1: Write a failing shared-kernel test**

Add to the existing `campfire_handler.rs` test module:

```rust
#[test]
fn rest_healing_kernel_matches_rest_execution_before_reward_side_effects() {
    let mut kernel_run = RunState::new(29, 0, false, "Ironclad");
    kernel_run.current_hp = 20;
    kernel_run.relics = vec![RelicState::new(RelicId::RegalPillow)];
    let mut handler_run = kernel_run.clone();

    super::apply_campfire_rest_healing(&mut kernel_run);
    let mut engine = EngineState::Campfire;
    assert!(super::handle(
        &mut engine,
        &mut handler_run,
        Some(ClientInput::CampfireOption(CampfireChoice::Rest)),
    ));

    assert_eq!(kernel_run.current_hp, handler_run.current_hp);
    assert_eq!(kernel_run.current_hp, 59);
    assert!(matches!(engine, EngineState::MapNavigation));
}
```

- [ ] **Step 2: Run the focused test and verify RED**

```powershell
cargo test -p sts_simulator rest_healing_kernel_matches_rest_execution_before_reward_side_effects --lib
```

Expected: compilation fails because `apply_campfire_rest_healing` does not exist.

- [ ] **Step 3: Extract and reuse the exact mechanic**

Add this function above `handle`:

```rust
pub fn apply_campfire_rest_healing(run_state: &mut RunState) {
    let heal_pct = if run_state.ascension_level >= 14 {
        0.25f32
    } else {
        0.3f32
    };
    let mut heal = (run_state.max_hp as f32 * heal_pct) as i32;

    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::RegalPillow)
    {
        heal += 15;
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MarkOfTheBloom)
    {
        heal = 0;
    }
    run_state.current_hp = (run_state.current_hp + heal).min(run_state.max_hp);
}
```

Replace the healing calculation inside `CampfireChoice::Rest` with:

```rust
apply_campfire_rest_healing(run_state);
```

Keep Dream Catcher generation after that call.

- [ ] **Step 4: Run Rest and Campfire tests and verify GREEN**

```powershell
cargo test -p sts_simulator engine::campfire_handler --lib
```

Expected: the new kernel test and all existing Campfire mechanics tests pass.

- [ ] **Step 5: Commit the shared exact prefix**

```powershell
git add src/engine/campfire_handler.rs
git commit -m "refactor: extract campfire rest healing kernel"
```

---

### Task 2: Add Offline Exact, Chance, And Recourse Projection

**Files:**
- Create: `src/eval/campfire_projection.rs`
- Modify: `src/eval/mod.rs`

**Interfaces:**
- Consumes: `CampfireCandidate`, `resolve_campfire_candidate`, `apply_campfire_rest_healing`, and the real `campfire_handler::handle`.
- Produces: `project_campfire_candidate`, `CampfireProjection`, `CampfireExactProjection`, `CampfireChanceProjection`, `CampfireRecourseProjection`, `CampfireExactPrefix`, `CampfireChanceKind`, `CampfireRecourseKind`, and `CampfireProjectionError`.
- Does not produce: survival scores, action ranking, stochastic outcomes, or run-control decisions.

- [ ] **Step 1: Write failing projection and information-boundary tests**

Create `src/eval/campfire_projection.rs`, export it from `src/eval/mod.rs`, and add these tests before production definitions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::run::RunState;

    #[test]
    fn smith_projection_uses_real_engine_on_a_clone_without_mutating_root() {
        let mut root = RunState::new(31, 0, false, "Ironclad");
        root.master_deck = vec![CombatCard::new(CardId::Strike, 501)];
        let before = root.clone();

        let projection = project_campfire_candidate(
            &root,
            CampfireCandidate::Smith { card_uuid: 501 },
        )
        .unwrap();

        let CampfireProjection::Exact(exact) = projection else {
            panic!("Smith must be exact");
        };
        assert_eq!(root, before);
        assert_eq!(exact.run_state.master_deck[0].upgrades, 1);
        assert_eq!(exact.run_state.rng_pool, root.rng_pool);
        assert!(matches!(exact.engine_state, EngineState::MapNavigation));
    }

    #[test]
    fn dig_projection_is_invariant_to_hidden_relic_rng_and_pool_order() {
        let mut root = RunState::new(37, 0, false, "Ironclad");
        root.relics.push(RelicState::new(RelicId::Shovel));
        let mut hidden_variant = root.clone();
        hidden_variant.rng_pool.relic_rng.random(999);
        hidden_variant.common_relic_pool.reverse();
        hidden_variant.uncommon_relic_pool.reverse();

        let first = project_campfire_candidate(&root, CampfireCandidate::Dig).unwrap();
        let second =
            project_campfire_candidate(&hidden_variant, CampfireCandidate::Dig).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first,
            CampfireProjection::Chance(CampfireChanceProjection {
                candidate: CampfireCandidate::Dig,
                exact_prefix: CampfireExactPrefix {
                    hp_before: root.current_hp,
                    hp_after: root.current_hp,
                },
                chance: CampfireChanceKind::DigRelicReward,
            })
        );
    }

    #[test]
    fn dream_catcher_projection_is_exact_heal_then_post_reveal_recourse() {
        let mut root = RunState::new(41, 0, false, "Ironclad");
        root.current_hp = 20;
        root.relics = vec![RelicState::new(RelicId::DreamCatcher)];
        let mut hidden_variant = root.clone();
        hidden_variant.rng_pool.card_rng.random(999);

        let first = project_campfire_candidate(&root, CampfireCandidate::Rest).unwrap();
        let second =
            project_campfire_candidate(&hidden_variant, CampfireCandidate::Rest).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first,
            CampfireProjection::ChanceThenDecision(CampfireRecourseProjection {
                candidate: CampfireCandidate::Rest,
                exact_prefix: CampfireExactPrefix {
                    hp_before: 20,
                    hp_after: 44,
                },
                chance: CampfireChanceKind::DreamCatcherCardReward,
                recourse: CampfireRecourseKind::ExistingCardRewardOwner,
            })
        );
    }
}
```

- [ ] **Step 2: Run projection tests and verify RED**

```powershell
cargo test -p sts_simulator eval::campfire_projection::tests --lib
```

Expected: compilation fails because the projection contract and function do not exist.

- [ ] **Step 3: Implement the minimal non-peeking projection contract**

Add these definitions above the tests:

```rust
use crate::content::relics::RelicId;
use crate::engine::campfire_candidates::{
    resolve_campfire_candidate, CampfireCandidate, CampfireCandidateResolutionError,
};
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::{with_suppressed_obtain_logs, RunState};

#[derive(Clone, Debug, PartialEq)]
pub enum CampfireProjection {
    Exact(CampfireExactProjection),
    Chance(CampfireChanceProjection),
    ChanceThenDecision(CampfireRecourseProjection),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireExactProjection {
    pub candidate: CampfireCandidate,
    pub engine_state: EngineState,
    pub run_state: RunState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampfireChanceProjection {
    pub candidate: CampfireCandidate,
    pub exact_prefix: CampfireExactPrefix,
    pub chance: CampfireChanceKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampfireRecourseProjection {
    pub candidate: CampfireCandidate,
    pub exact_prefix: CampfireExactPrefix,
    pub chance: CampfireChanceKind,
    pub recourse: CampfireRecourseKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampfireExactPrefix {
    pub hp_before: i32,
    pub hp_after: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireChanceKind {
    DigRelicReward,
    DreamCatcherCardReward,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireRecourseKind {
    ExistingCardRewardOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireProjectionError {
    Candidate(CampfireCandidateResolutionError),
    EngineDidNotAdvance { candidate: CampfireCandidate },
    UnexpectedRngMutation { candidate: CampfireCandidate },
}

impl From<CampfireCandidateResolutionError> for CampfireProjectionError {
    fn from(value: CampfireCandidateResolutionError) -> Self {
        Self::Candidate(value)
    }
}

pub fn project_campfire_candidate(
    root: &RunState,
    candidate: CampfireCandidate,
) -> Result<CampfireProjection, CampfireProjectionError> {
    let choice = resolve_campfire_candidate(root, candidate)?;
    if candidate == CampfireCandidate::Dig {
        return Ok(CampfireProjection::Chance(CampfireChanceProjection {
            candidate,
            exact_prefix: unchanged_hp_prefix(root),
            chance: CampfireChanceKind::DigRelicReward,
        }));
    }
    if candidate == CampfireCandidate::Rest && has_dream_catcher(root) {
        let mut prefix = root.clone();
        crate::engine::campfire_handler::apply_campfire_rest_healing(&mut prefix);
        return Ok(CampfireProjection::ChanceThenDecision(
            CampfireRecourseProjection {
                candidate,
                exact_prefix: CampfireExactPrefix {
                    hp_before: root.current_hp,
                    hp_after: prefix.current_hp,
                },
                chance: CampfireChanceKind::DreamCatcherCardReward,
                recourse: CampfireRecourseKind::ExistingCardRewardOwner,
            },
        ));
    }

    let mut engine_state = EngineState::Campfire;
    let mut run_state = root.clone();
    let original_rng = run_state.rng_pool.clone();
    with_suppressed_obtain_logs(|| {
        crate::engine::campfire_handler::handle(
            &mut engine_state,
            &mut run_state,
            Some(ClientInput::CampfireOption(choice)),
        )
    });
    if matches!(engine_state, EngineState::Campfire) {
        return Err(CampfireProjectionError::EngineDidNotAdvance { candidate });
    }
    if run_state.rng_pool != original_rng {
        return Err(CampfireProjectionError::UnexpectedRngMutation { candidate });
    }
    Ok(CampfireProjection::Exact(CampfireExactProjection {
        candidate,
        engine_state,
        run_state,
    }))
}

fn unchanged_hp_prefix(root: &RunState) -> CampfireExactPrefix {
    CampfireExactPrefix {
        hp_before: root.current_hp,
        hp_after: root.current_hp,
    }
}

fn has_dream_catcher(root: &RunState) -> bool {
    root.relics
        .iter()
        .any(|relic| relic.id == RelicId::DreamCatcher)
}
```

Export the module in `src/eval/mod.rs`:

```rust
pub mod campfire_projection;
```

- [ ] **Step 4: Run focused projection and Campfire tests and verify GREEN**

```powershell
cargo test -p sts_simulator campfire_projection --lib
cargo test -p sts_simulator engine::campfire --lib
```

Expected: projection information-boundary tests and all Campfire mechanics tests pass.

- [ ] **Step 5: Commit the offline projection boundary**

```powershell
git add src/eval/campfire_projection.rs src/eval/mod.rs
git commit -m "feat: add non-peeking campfire projections"
```

---

### Task 3: Verify The Projection Slice

**Files:**
- Verify only; no planned source changes.

**Interfaces:**
- Verifies: shared Rest mechanics, deterministic exact projection, RNG invariance for chance/recourse boundaries, and repository architecture constraints.

- [ ] **Step 1: Format and run focused tests**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire_projection --lib
cargo test -p sts_simulator engine::campfire --lib
```

Expected: formatting succeeds and every focused test passes.

- [ ] **Step 2: Run completion suites required by `AGENTS.md`**

```powershell
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: the full library and architecture boundary suites pass.

- [ ] **Step 3: Confirm clean local history**

```powershell
git status --short --branch
git log -5 --oneline
```

Expected: no uncommitted source changes and two projection-slice commits above the plan commit.
