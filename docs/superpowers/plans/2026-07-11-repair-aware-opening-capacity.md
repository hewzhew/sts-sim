# Repair-Aware Opening Capacity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the false Pyramid/Apparition hard liability with repair-aware coverage evidence and make combat hand/draw estimates obey Choker and Pyramid capacity.

**Architecture:** Extend the existing startup profile with categorical facts rather than a new score, then let boss-relic admission report those facts. Keep simulator behavior authoritative and align only the compact combat pile evaluator with exact remaining card-play and retained-hand capacity.

**Tech Stack:** Rust 2021, Serde, existing `RunState`, `DeckStartupProfileV1`, boss-relic admission, and Combat Search V2 unit tests.

## Global Constraints

- Do not add an aggregate Pyramid, Choker, Enchiridion, or Toolbox score.
- Do not treat possible future permanent upgrades as guaranteed.
- Preserve the serialized `has_pyramid_unupgraded_apparition` field.
- Default every newly serialized startup-profile field for old stored data.
- Do not mutate live run or combat state during assessment.
- Do not change Toolbox's fixed shop-relic purchase score.
- Do not add a full-route, full-seed, random-output, or boss-outcome regression test.
- Do not redesign turn enumeration, action ordering, or Time Eater policy.

---

### Task 1: Repair-aware startup and opening-option facts

**Files:**
- Modify: `src/ai/deck_startup_profile_v1.rs`
- Test: `src/ai/deck_startup_profile_v1.rs`

**Interfaces:**
- Consumes: `RunState.act_num`, master-deck card ids/upgrades, and owned relic ids.
- Produces: `PyramidApparitionCoverageV1`, repair-access counters, generated-opening counters, and categorical Choker/Pyramid tradeoff flags on `DeckStartupProfileV1`.

- [ ] **Step 1: Write failing profile tests**

Add tests covering these exact relationships:

```rust
#[test]
fn pyramid_apparitions_report_live_whole_hand_repair_from_armaments_plus() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.act_num = 2;
    run.relics.push(RelicState::new(RelicId::RunicPyramid));
    run.add_card_to_deck(CardId::Apparition);
    let mut armaments = crate::runtime::combat::CombatCard::new(CardId::Armaments, 1001);
    armaments.upgrades = 1;
    run.master_deck.push(armaments);

    let profile = deck_startup_profile_v1(&run);

    assert_eq!(
        profile.pyramid_apparition_coverage,
        PyramidApparitionCoverageV1::CombatRepairAvailable
    );
    assert_eq!(profile.combat_upgrade_hand_access_count, 1);
    assert_eq!(profile.combat_upgrade_selected_access_count, 0);
}

#[test]
fn pyramid_apparition_coverage_distinguishes_ready_future_and_limited() {
    let mut ready = RunState::new(1, 0, false, "Ironclad");
    ready.relics.push(RelicState::new(RelicId::RunicPyramid));
    let mut apparition = crate::runtime::combat::CombatCard::new(CardId::Apparition, 1001);
    apparition.upgrades = 1;
    ready.master_deck.push(apparition);
    assert_eq!(
        deck_startup_profile_v1(&ready).pyramid_apparition_coverage,
        PyramidApparitionCoverageV1::Ready
    );

    let mut future = RunState::new(2, 0, false, "Ironclad");
    future.act_num = 2;
    future.relics.push(RelicState::new(RelicId::RunicPyramid));
    future.add_card_to_deck(CardId::Apparition);
    assert_eq!(
        deck_startup_profile_v1(&future).pyramid_apparition_coverage,
        PyramidApparitionCoverageV1::FutureUpgradeWindow
    );

    future.act_num = 3;
    assert_eq!(
        deck_startup_profile_v1(&future).pyramid_apparition_coverage,
        PyramidApparitionCoverageV1::Limited
    );
}

#[test]
fn generated_opening_options_are_budget_facts_not_shape_risk() {
    let mut run = RunState::new(3, 0, false, "Ironclad");
    run.relics = vec![
        RelicState::new(RelicId::VelvetChoker),
        RelicState::new(RelicId::RunicPyramid),
        RelicState::new(RelicId::Enchiridion),
        RelicState::new(RelicId::Toolbox),
    ];
    run.add_card_to_deck(CardId::Apparition);

    let profile = deck_startup_profile_v1(&run);

    assert_eq!(profile.opening_generated_option_count, 2);
    assert_eq!(profile.opening_generated_zero_cost_this_turn_count, 1);
    assert!(profile.has_choker_generated_opening_budget);
    assert!(profile.has_pyramid_choker_generated_opening_tradeoff);
    assert_eq!(profile.combat_shape_risk, 0);
    assert!(profile.has_pyramid_unupgraded_apparition);
}

#[test]
fn older_serialized_startup_profiles_default_new_capacity_fields() {
    let mut value = serde_json::to_value(DeckStartupProfileV1::default())
        .expect("profile should serialize");
    let object = value.as_object_mut().expect("profile should be an object");
    for field in [
        "pyramid_apparition_coverage",
        "combat_upgrade_selected_access_count",
        "combat_upgrade_hand_access_count",
        "opening_generated_option_count",
        "opening_generated_zero_cost_this_turn_count",
        "has_velvet_choker",
        "has_choker_generated_opening_budget",
        "has_pyramid_choker_generated_opening_tradeoff",
    ] {
        object.remove(field);
    }

    let decoded: DeckStartupProfileV1 =
        serde_json::from_value(value).expect("older profile should deserialize");

    assert_eq!(
        decoded.pyramid_apparition_coverage,
        PyramidApparitionCoverageV1::NotApplicable
    );
    assert_eq!(decoded.opening_generated_option_count, 0);
    assert!(!decoded.has_choker_generated_opening_budget);
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
```

Expected: compilation fails because the coverage enum and new fields do not exist. Add only the enum/field/default skeleton needed to compile, rerun, and confirm the new assertions fail against the old profile behavior.

- [ ] **Step 3: Implement categorical repair and opening-option facts**

Add the serializable coverage type:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PyramidApparitionCoverageV1 {
    NotApplicable,
    Ready,
    CombatRepairAvailable,
    FutureUpgradeWindow,
    Limited,
}

impl Default for PyramidApparitionCoverageV1 {
    fn default() -> Self {
        Self::NotApplicable
    }
}

impl PyramidApparitionCoverageV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::Ready => "ready",
            Self::CombatRepairAvailable => "combat-repair",
            Self::FutureUpgradeWindow => "future-upgrade-window",
            Self::Limited => "limited",
        }
    }
}
```

Add these fields to `DeckStartupProfileV1`, each with `#[serde(default)]`:

```rust
pub pyramid_apparition_coverage: PyramidApparitionCoverageV1,
pub combat_upgrade_selected_access_count: u8,
pub combat_upgrade_hand_access_count: u8,
pub opening_generated_option_count: u8,
pub opening_generated_zero_cost_this_turn_count: u8,
pub has_velvet_choker: bool,
pub has_choker_generated_opening_budget: bool,
pub has_pyramid_choker_generated_opening_tradeoff: bool,
```

Initialize `has_velvet_choker` beside the existing Pyramid/Snecko facts. In the relic loop,
record Enchiridion and Toolbox as follows:

```rust
RelicId::Enchiridion => {
    profile.opening_generated_option_count =
        profile.opening_generated_option_count.saturating_add(1);
    profile.opening_generated_zero_cost_this_turn_count =
        profile.opening_generated_zero_cost_this_turn_count.saturating_add(1);
}
RelicId::Toolbox => {
    profile.opening_generated_option_count =
        profile.opening_generated_option_count.saturating_add(1);
}
```

Record combat repair while iterating master-deck cards:

```rust
match id {
    CardId::Armaments if card.upgrades > 0 => {
        profile.combat_upgrade_hand_access_count =
            profile.combat_upgrade_hand_access_count.saturating_add(1);
    }
    CardId::Armaments => {
        profile.combat_upgrade_selected_access_count =
            profile.combat_upgrade_selected_access_count.saturating_add(1);
    }
    CardId::Apotheosis => {
        profile.combat_upgrade_hand_access_count =
            profile.combat_upgrade_hand_access_count.saturating_add(1);
    }
    _ => {}
}
```

Remove the Pyramid/Apparition clause from the `combat_shape_risk` increment. After all card and
relic counts are known, derive the new facts:

```rust
profile.has_choker_generated_opening_budget =
    profile.has_velvet_choker && profile.opening_generated_option_count > 0;
profile.has_pyramid_choker_generated_opening_tradeoff =
    profile.has_runic_pyramid && profile.has_choker_generated_opening_budget;
profile.pyramid_apparition_coverage = pyramid_apparition_coverage_v1(&profile, run_state.act_num);
```

Use this exact helper:

```rust
fn pyramid_apparition_coverage_v1(
    profile: &DeckStartupProfileV1,
    act_num: u8,
) -> PyramidApparitionCoverageV1 {
    if !profile.has_runic_pyramid || profile.apparition_count == 0 {
        PyramidApparitionCoverageV1::NotApplicable
    } else if profile.apparition_count == profile.upgraded_apparition_count {
        PyramidApparitionCoverageV1::Ready
    } else if profile
        .combat_upgrade_selected_access_count
        .saturating_add(profile.combat_upgrade_hand_access_count)
        > 0
    {
        PyramidApparitionCoverageV1::CombatRepairAvailable
    } else if act_num <= 2 {
        PyramidApparitionCoverageV1::FutureUpgradeWindow
    } else {
        PyramidApparitionCoverageV1::Limited
    }
}
```

- [ ] **Step 4: Verify GREEN and commit**

Run:

```powershell
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Expected: all startup-profile tests pass and both checks exit 0.

Commit:

```powershell
git add -- src/ai/deck_startup_profile_v1.rs
git commit -m "fix: model repairable Pyramid retention"
```

---

### Task 2: Truthful Pyramid boss-relic evidence

**Files:**
- Modify: `src/ai/strategy/boss_relic_admission.rs`
- Test: `src/ai/strategy/boss_relic_admission.rs`

**Interfaces:**
- Consumes: `deck_startup_profile_v1(&RunState)` and `PyramidApparitionCoverageV1` from Task 1.
- Produces: repair-aware `BossRelicAdmissionReason` variants while preserving the existing lane, burden, and class order types.

- [ ] **Step 1: Replace stale expected behavior with failing repair-aware tests**

Replace the old Pyramid/Apparition liability and same-lane ordering tests with:

```rust
#[test]
fn pyramid_reports_repairable_coverage_without_mutating_run_or_adding_burden() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;
    run.master_deck
        .push(CombatCard::new(CardId::Apparition, 1001));
    let mut armaments = CombatCard::new(CardId::Armaments, 1002);
    armaments.upgrades = 1;
    run.master_deck.push(armaments);
    let relic_count = run.relics.len();

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
    assert_eq!(pyramid.burden, BossRelicAdmissionBurden::None);
    assert!(pyramid.reasons.contains(
        &BossRelicAdmissionReason::PyramidApparitionCoverage(
            PyramidApparitionCoverageV1::CombatRepairAvailable,
        )
    ));
    assert!(!pyramid
        .reasons
        .contains(&BossRelicAdmissionReason::IntroducesStartupLiability));
    assert_eq!(run.relics.len(), relic_count);
}

#[test]
fn repairable_pyramid_competes_normally_inside_probe_lane() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;
    run.master_deck
        .push(CombatCard::new(CardId::Apparition, 1001));
    let mut armaments = CombatCard::new(CardId::Armaments, 1002);
    armaments.upgrades = 1;
    run.master_deck.push(armaments);

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);
    let bark = assess_boss_relic_admission(&run, RelicId::SacredBark);
    let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);

    assert!(boss_relic_admission_order_rank(&pyramid)
        < boss_relic_admission_order_rank(&bark));
    assert!(boss_relic_admission_order_rank(&bark)
        < boss_relic_admission_order_rank(&sozu));
}
```

Add one test for projected opening budgeting:

```rust
#[test]
fn pyramid_reports_choker_generated_opening_budget_as_evidence() {
    let mut run = RunState::new(1552225673, 0, false, "Ironclad");
    run.act_num = 2;
    run.relics = vec![
        RelicState::new(RelicId::VelvetChoker),
        RelicState::new(RelicId::Enchiridion),
    ];

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert!(pyramid.reasons.contains(
        &BossRelicAdmissionReason::OpeningActionBudgetRequired {
            generated_options: 1,
        }
    ));
    assert_eq!(pyramid.burden, BossRelicAdmissionBurden::None);
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
cargo test --lib ai::strategy::boss_relic_admission::tests -- --nocapture
```

Expected: compilation fails because the new reason variants do not exist; after adding only
those variants, assertions still fail because projected Pyramid is classified by the old hard
liability path.

- [ ] **Step 3: Implement projected coverage evidence**

Import `PyramidApparitionCoverageV1` and add these reason variants:

```rust
PyramidApparitionCoverage(PyramidApparitionCoverageV1),
OpeningActionBudgetRequired { generated_options: u8 },
```

Replace `introduces_known_startup_liability` with a projection helper:

```rust
fn projected_startup_profile(
    run_state: &RunState,
    relic: RelicId,
) -> crate::ai::deck_startup_profile_v1::DeckStartupProfileV1 {
    let mut projected_run = run_state.clone();
    if !projected_run.relics.iter().any(|state| state.id == relic) {
        projected_run.relics.push(RelicState::new(relic));
    }
    deck_startup_profile_v1(&projected_run)
}
```

After existing class/lane assessment, project the startup profile. For Runic Pyramid, append
coverage and action-budget evidence:

```rust
let projected_startup = projected_startup_profile(run_state, relic);
if relic == RelicId::RunicPyramid {
    reasons.push(BossRelicAdmissionReason::PyramidApparitionCoverage(
        projected_startup.pyramid_apparition_coverage,
    ));
    if projected_startup.has_pyramid_choker_generated_opening_tradeoff {
        reasons.push(BossRelicAdmissionReason::OpeningActionBudgetRequired {
            generated_options: projected_startup.opening_generated_option_count,
        });
    }
}
```

Compute burden only from real added run debt in this pass:

```rust
let burden = if debt_projection.added_contracts.is_empty() {
    BossRelicAdmissionBurden::None
} else {
    reasons.push(BossRelicAdmissionReason::AddsRunDebt {
        contracts: debt_projection.added_contracts.len(),
    });
    BossRelicAdmissionBurden::AddedRunDebt
};
```

Render the new reasons with stable compact labels:

```rust
BossRelicAdmissionReason::PyramidApparitionCoverage(coverage) => {
    format!("apparition-coverage:{}", coverage.label())
}
BossRelicAdmissionReason::OpeningActionBudgetRequired { generated_options } => {
    format!("opening-action-budget:{generated_options}")
}
```

- [ ] **Step 4: Verify GREEN and commit**

Run:

```powershell
cargo test --lib ai::strategy::boss_relic_admission::tests -- --nocapture
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Expected: both focused suites pass; formatting and diff checks exit 0.

Commit:

```powershell
git add -- src/ai/strategy/boss_relic_admission.rs
git commit -m "fix: preserve repairable Pyramid value"
```

---

### Task 3: Choker play capacity and Pyramid draw capacity

**Files:**
- Modify: `src/ai/combat_search_v2/card_pile_value.rs`
- Test: `src/ai/combat_search_v2/card_pile_value.rs`

**Interfaces:**
- Consumes: exact `CombatState` relics, `cards_played_this_turn`, effective Ethereal/retain semantics, and `compute_player_turn_start_draw_count`.
- Produces: capacity-aware `hand_value(&CombatState)` and `next_draw_value(&CombatState)` without changing their public signatures.

- [ ] **Step 1: Write failing exact-capacity tests**

Add these tests:

```rust
#[test]
fn hand_playable_count_obeys_remaining_velvet_choker_slots() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::VelvetChoker));
    combat.turn.energy = 3;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Strike, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    combat.turn.counters.cards_played_this_turn = 5;
    assert_eq!(hand_value(&combat).playable_cards, 1);

    combat.turn.counters.cards_played_this_turn = 6;
    assert_eq!(hand_value(&combat).playable_cards, 0);
}

#[test]
fn pyramid_retained_hand_caps_next_turn_draw() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RunicPyramid));
    combat.zones.hand = (0..8)
        .map(|index| CombatCard::new(CardId::Defend, 100 + index))
        .collect();
    combat.zones.draw_pile = (0..5)
        .map(|index| CombatCard::new(CardId::Strike, 200 + index))
        .collect();

    let value = next_draw_value(&combat);

    assert_eq!(value.playable_cards, 2);
    assert_eq!(value.damage, 12);
}

#[test]
fn ethereal_apparitions_release_pyramid_draw_capacity_unless_explicitly_retained() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RunicPyramid));
    combat.zones.hand = (0..5)
        .map(|index| CombatCard::new(CardId::Defend, 100 + index))
        .chain((0..3).map(|index| CombatCard::new(CardId::Apparition, 200 + index)))
        .collect();
    combat.zones.draw_pile = (0..5)
        .map(|index| CombatCard::new(CardId::Strike, 300 + index))
        .collect();

    assert_eq!(next_draw_value(&combat).damage, 30);

    combat.zones.hand[5].retain_override = Some(true);
    assert_eq!(next_draw_value(&combat).damage, 24);
}
```

Import `RelicId` and `RelicState` in the test module.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
cargo test --lib ai::combat_search_v2::card_pile_value::tests -- --nocapture
```

Expected: the Choker test reports 3 instead of 1/0, the Pyramid hand test reports five drawn
cards instead of two, and explicit retain does not change the old next-draw estimate.

- [ ] **Step 3: Implement exact capacity helpers**

Remove the duplicated base draw constant. Make `hand_value` cap only the playable-card count:

```rust
pub(super) fn hand_value(combat: &CombatState) -> CardPileValueV1 {
    let mut value = card_pile_value(combat.zones.hand.iter(), combat.turn.energy as i32);
    if let Some(capacity) = remaining_card_play_capacity(combat) {
        value.playable_cards = value.playable_cards.min(capacity as i32);
    }
    value
}

fn remaining_card_play_capacity(combat: &CombatState) -> Option<usize> {
    combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::VelvetChoker)
        .then(|| {
            6usize.saturating_sub(
                combat.turn.counters.cards_played_this_turn as usize,
            )
        })
}
```

Compute next draw from the simulator draw helper and retained-hand capacity:

```rust
pub(super) fn next_draw_value(combat: &CombatState) -> CardPileValueV1 {
    let requested = crate::engine::core::compute_player_turn_start_draw_count(combat)
        .max(0) as usize;
    let retained = projected_retained_hand_count(combat);
    let hand_capacity = 10usize.saturating_sub(retained);
    let draw_count = requested
        .min(hand_capacity)
        .min(combat.zones.draw_pile.len());
    card_pile_value(
        combat.zones.draw_pile.iter().take(draw_count),
        combat.entities.player.energy_master as i32,
    )
}

fn projected_retained_hand_count(combat: &CombatState) -> usize {
    let has_pyramid = combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::RunicPyramid);
    combat
        .zones
        .hand
        .iter()
        .filter(|card| {
            let explicitly_retained = card.retain_override == Some(true)
                || crate::content::cards::is_self_retain(card);
            explicitly_retained
                || (has_pyramid && !crate::content::cards::is_ethereal(card))
        })
        .count()
}
```

- [ ] **Step 4: Verify GREEN and commit**

Run:

```powershell
cargo test --lib ai::combat_search_v2::card_pile_value::tests -- --nocapture
cargo test --lib ai::combat_search_v2::segment_plan::tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Expected: both focused suites pass and checks exit 0.

Commit:

```powershell
git add -- src/ai/combat_search_v2/card_pile_value.rs
git commit -m "fix: respect opening action and hand capacity"
```

---

### Task 4: Complete verification

**Files:**
- Verify: `src/ai/deck_startup_profile_v1.rs`
- Verify: `src/ai/strategy/boss_relic_admission.rs`
- Verify: `src/ai/combat_search_v2/card_pile_value.rs`

**Interfaces:**
- Consumes: all three committed task outputs.
- Produces: fresh repository-wide verification evidence.

- [ ] **Step 1: Run formatting, library, and architecture suites**

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected: formatting and diff checks exit 0; all library and seven architecture tests pass with
zero failures.

- [ ] **Step 2: Confirm branch scope**

```powershell
git status --short --branch
git log --oneline master..HEAD
```

Expected: the worktree is clean and the branch contains only the three implementation commits.
