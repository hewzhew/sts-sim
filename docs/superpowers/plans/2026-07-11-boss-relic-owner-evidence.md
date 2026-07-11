# Boss Relic Owner Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make boss-relic admission and ordering consume existing run-debt and startup-liability evidence before the narrow owner selects its first candidate.

**Architecture:** Keep `boss_relic_owner` unchanged as the runtime owner. Enrich `BossRelicAdmission` with a categorical burden computed from `run_debt_projection_for_relic_v1` and an assessment-only projected `DeckStartupProfileV1`, then include that burden between lane and class in the existing order key.

**Tech Stack:** Rust 2021, existing `RunState`, `DeckStartupProfileV1`, run-debt projection, and built-in Rust tests.

## Global Constraints

- Do not add an aggregate boss-relic score.
- Do not mutate the live `RunState` while projecting a candidate relic.
- Only `has_pyramid_unupgraded_apparition` is a startup-liability boundary in this pass.
- Keep all executable boss-relic candidates visible and auto-expandable.
- Do not model Velvet Choker plus Runic Pyramid or generated opening cards in this pass.
- Do not add a frozen-seed or full-run regression test.

---

### Task 1: Deck-relative boss-relic burden

**Files:**
- Modify: `src/ai/strategy/boss_relic_admission.rs`
- Test: `src/ai/strategy/boss_relic_admission.rs`

**Interfaces:**
- Consumes: `deck_startup_profile_v1(&RunState) -> DeckStartupProfileV1`, `run_debt_projection_for_relic_v1(&RunState, RelicId) -> RunDebtProjectionV1`, and `RelicState::new(RelicId)`.
- Produces: `BossRelicAdmissionBurden`, `BossRelicAdmission::burden`, and burden-aware `boss_relic_admission_order_rank(&BossRelicAdmission) -> u8`.

- [ ] **Step 1: Write failing behavior tests**

Add these tests to the existing `tests` module:

```rust
#[test]
fn strategic_power_defaults_to_probe() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
}

#[test]
fn pyramid_apparition_liability_is_projected_without_mutating_run() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;
    run.master_deck.push(CombatCard::new(CardId::Apparition, 1001));
    let relic_count = run.relics.len();

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
    assert_eq!(
        pyramid.burden,
        BossRelicAdmissionBurden::IntroducedStartupLiability
    );
    assert!(pyramid
        .reasons
        .contains(&BossRelicAdmissionReason::IntroducesStartupLiability));
    assert_eq!(run.relics.len(), relic_count);
    assert!(!run
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::RunicPyramid));
}

#[test]
fn same_lane_prefers_no_burden_then_run_debt_then_startup_liability() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;
    run.master_deck.push(CombatCard::new(CardId::Apparition, 1001));

    let bark = assess_boss_relic_admission(&run, RelicId::SacredBark);
    let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);
    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert_eq!(bark.lane, BossRelicAdmissionLane::Probe);
    assert_eq!(sozu.lane, BossRelicAdmissionLane::Probe);
    assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
    assert!(boss_relic_admission_order_rank(&bark)
        < boss_relic_admission_order_rank(&sozu));
    assert!(boss_relic_admission_order_rank(&sozu)
        < boss_relic_admission_order_rank(&pyramid));
}

#[test]
fn energy_gap_mainline_stays_ahead_of_burden_free_probe() {
    let run = RunState::new(1552225673, 0, false, "Ironclad");

    let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);
    let bark = assess_boss_relic_admission(&run, RelicId::SacredBark);

    assert_eq!(sozu.lane, BossRelicAdmissionLane::Mainline);
    assert_eq!(bark.lane, BossRelicAdmissionLane::Probe);
    assert!(boss_relic_admission_order_rank(&sozu)
        < boss_relic_admission_order_rank(&bark));
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test --lib ai::strategy::boss_relic_admission::tests -- --nocapture
```

Expected: compilation fails because `BossRelicAdmissionBurden`, `BossRelicAdmission::burden`, and `BossRelicAdmissionReason::IntroducesStartupLiability` do not exist yet; after adding only the enum/field declarations needed to compile, the strategic-power and ordering assertions must still fail against the old behavior before implementation proceeds.

- [ ] **Step 3: Implement categorical burden and projected startup evidence**

Import the existing evidence providers:

```rust
use crate::ai::deck_startup_profile_v1::deck_startup_profile_v1;
use crate::ai::strategic::run_debt_projection_for_relic_v1;
use crate::content::relics::{RelicId, RelicState};
```

Add the burden enum:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicAdmissionBurden {
    None,
    AddedRunDebt,
    IntroducedStartupLiability,
}

impl BossRelicAdmissionBurden {
    fn order_rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::AddedRunDebt => 1,
            Self::IntroducedStartupLiability => 2,
        }
    }
}

```

Append these exact variants to `BossRelicAdmissionReason`:

```rust
AddsRunDebt { contracts: usize },
IntroducesStartupLiability,
```

Append this exact field to `BossRelicAdmission`:

```rust
pub burden: BossRelicAdmissionBurden,
```

Make ordering lexicographic and initialize skip with no burden:

```rust
pub fn boss_relic_admission_order_rank(admission: &BossRelicAdmission) -> u8 {
    admission.lane.order_rank() * 64
        + admission.burden.order_rank() * 16
        + admission.class.order_rank()
}

// inside skip_boss_relic_admission
burden: BossRelicAdmissionBurden::None,
```

Compute the candidate burden after existing class/lane assessment:

```rust
let mut lane = lane_for_relic(run_state, &facts, relic, class, &mut reasons);
let debt_projection = run_debt_projection_for_relic_v1(run_state, relic);
let introduces_startup_liability =
    introduces_known_startup_liability(run_state, relic);
let burden = if introduces_startup_liability {
    reasons.push(BossRelicAdmissionReason::IntroducesStartupLiability);
    lane = BossRelicAdmissionLane::Probe;
    BossRelicAdmissionBurden::IntroducedStartupLiability
} else if debt_projection.added_contracts.is_empty() {
    BossRelicAdmissionBurden::None
} else {
    reasons.push(BossRelicAdmissionReason::AddsRunDebt {
        contracts: debt_projection.added_contracts.len(),
    });
    BossRelicAdmissionBurden::AddedRunDebt
};
```

Project the established liability without mutating the live run:

```rust
fn introduces_known_startup_liability(run_state: &RunState, relic: RelicId) -> bool {
    let current = deck_startup_profile_v1(run_state);
    let mut projected_run = run_state.clone();
    if !projected_run.relics.iter().any(|state| state.id == relic) {
        projected_run.relics.push(RelicState::new(relic));
    }
    let projected = deck_startup_profile_v1(&projected_run);

    !current.has_pyramid_unupgraded_apparition
        && projected.has_pyramid_unupgraded_apparition
}
```

Change the `StrategicPower` arm in `default_lane` to `Probe`, add `burden` to the normal admission constructor, and render the new reasons:

```rust
BossRelicAdmissionReason::AddsRunDebt { contracts } => {
    format!("adds-run-debt:{contracts}")
}
BossRelicAdmissionReason::IntroducesStartupLiability => {
    "startup-liability".to_string()
}
```

- [ ] **Step 4: Run focused and neighboring tests and verify GREEN**

Run:

```powershell
cargo test --lib ai::strategy::boss_relic_admission::tests -- --nocapture
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
```

Expected: both commands exit 0 with no failing tests.

- [ ] **Step 5: Format and run the complete library and architecture suites**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture
git diff --check
```

Expected: formatting and diff checks exit 0; all library and architecture tests pass with zero failures.

- [ ] **Step 6: Commit the implementation**

```powershell
git add -- src/ai/strategy/boss_relic_admission.rs
git commit -m "fix: make boss relic admission evidence aware"
```
