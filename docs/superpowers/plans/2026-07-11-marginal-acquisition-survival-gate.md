# Marginal Acquisition And Survival Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make engine admission and boss-scaling credit depend on marginal deck improvement, then prevent non-survival setup from displacing immediate defense at low HP.

**Architecture:** Keep the existing package transition, deck plan, acquisition, and decision-pipeline boundaries. Tighten reward admission using transition deltas, centralize the existing strength reliability predicate on `DeckPlanSnapshot`, and apply an acute-survival lane cap after scoring instead of adding another large score penalty.

**Tech Stack:** Rust, existing strategy modules, Cargo library tests, and the architecture boundary integration suite.

## Global Constraints

- Do not redesign Velvet Choker, Runic Pyramid, Apparition, or generated-startup-card compatibility in this pass.
- Do not change combat search.
- Do not replace the strategic-deficit schema or tune every acquisition score.
- Do not add an exact replay, frontier, checkpoint, or full-seed regression test.
- Do not forbid duplicate powers globally.
- Preserve the existing multiplier plus exactly one stable strength source reliability repair.
- Assert classifications and lane relationships, not exact aggregate scores.

---

### Task 1: Deck-relative engine admission

**Files:**
- Modify: `src/ai/strategy/reward_admission.rs`

**Interfaces:**
- Consumes: `PackageTransitionReport::{newly_open_requirements,new_installed_rules,new_mechanics,new_event_streams,package_changes}` and `PackageMaturity`.
- Produces: unchanged `assess_reward_admission*` APIs; `EngineSeed` now means that the candidate establishes a capability relative to the current deck.

- [ ] **Step 1: Write failing reward-admission tests**

Append a test module to `reward_admission.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_rupture_is_payoff_not_engine_seed() {
        let admission = assess_reward_admission(
            &[CardId::Strike, CardId::Defend, CardId::Bash],
            CardId::Rupture,
        );

        assert_eq!(
            admission.class,
            RewardAdmissionClass::OpensUnsupportedPayoff
        );
        assert!(admission.reasons.iter().any(|reason| matches!(
            reason,
            RewardAdmissionReason::Opens(PayoffRequirement::WantsEventStream(
                CombatEvent::CardSelfDamage
            ))
        )));
    }

    #[test]
    fn repeated_supported_rupture_is_not_a_new_engine_seed() {
        let admission = assess_reward_admission(
            &[CardId::Rupture, CardId::Hemokinesis],
            CardId::Rupture,
        );

        assert_ne!(admission.class, RewardAdmissionClass::EngineSeed);
    }

    #[test]
    fn first_independent_installed_rule_remains_an_engine_seed() {
        let admission = assess_reward_admission(
            &[CardId::Strike, CardId::Defend, CardId::Bash],
            CardId::Corruption,
        );

        assert_eq!(admission.class, RewardAdmissionClass::EngineSeed);
    }
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::strategy::reward_admission::tests -- --nocapture
```

Expected: the unsupported and repeated Rupture tests fail because both candidates are currently classified from their intrinsic event handler as `EngineSeed`; the Corruption guard passes.

- [ ] **Step 3: Implement deck-relative seed establishment**

Import `PackageTransitionReport` beside `PackageKind`, replace the intrinsic `engine_seed` expression with `establishes_engine_seed(&transition)`, and add:

```rust
fn establishes_engine_seed(transition: &PackageTransitionReport) -> bool {
    if !transition.newly_open_requirements.is_empty() {
        return false;
    }

    !transition.new_installed_rules.is_empty()
        || !transition.new_mechanics.is_empty()
        || !transition.new_event_streams.is_empty()
        || transition.package_changes.iter().any(|change| {
            matches!(
                change.to,
                PackageMaturity::SourceOnly | PackageMaturity::Seeded
            )
        })
}
```

Retain the existing class ordering. Once `engine_seed` is false, a candidate that opens an unsupported requirement falls through to `OpensUnsupportedPayoff`; a repeated supported handler with no transition falls through to `EmptyOrDeferred` unless it has another immediate or supported role.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run the Step 2 command again. Expected: all three reward-admission tests pass.

- [ ] **Step 5: Run adjacent package tests**

```powershell
cargo test --lib ai::strategy::package_ -- --nocapture
```

Expected: package-state and package-transition tests pass with zero failures.

- [ ] **Step 6: Commit Task 1**

```powershell
git add src/ai/strategy/reward_admission.rs
git commit -m "fix: make engine admission deck relative"
```

### Task 2: Marginal boss-scaling source evidence

**Files:**
- Modify: `src/ai/strategy/deck_plan.rs`
- Modify: `src/ai/strategy/acquisition.rs`
- Modify: `src/ai/strategy/boss_scaling_evidence.rs`
- Modify: `src/ai/strategy/decision_pipeline.rs`

**Interfaces:**
- Consumes: `DeckPlanSnapshot`, stable and conditional strength-source inventory, candidate card identity, and reward admission class.
- Produces: `DeckPlanSnapshot::repairs_strength_package_reliability(Option<(CardId, u8)>) -> bool`; unchanged `assess_boss_scaling_evidence` signature with marginal source semantics.

- [ ] **Step 1: Write failing boss-scaling tests**

Extend the existing test module in `boss_scaling_evidence.rs`:

```rust
#[test]
fn unsupported_rupture_is_not_a_usable_boss_scaling_source() {
    let (deck, plan) = deck_plan(&[CardId::Strike, CardId::Defend, CardId::Bash]);
    let admission = assess_reward_admission_from_master_deck(&deck, CardId::Rupture, 0);
    let evidence = assess_boss_scaling_evidence(plan, Some((CardId::Rupture, 0)), &admission);

    assert!(!evidence.relevant_to_boss_plan);
}

#[test]
fn repeated_rupture_does_not_repeat_full_boss_scaling_credit() {
    let (deck, plan) = deck_plan(&[CardId::Rupture, CardId::Hemokinesis]);
    let admission = assess_reward_admission_from_master_deck(&deck, CardId::Rupture, 1);
    let evidence = assess_boss_scaling_evidence(plan, Some((CardId::Rupture, 1)), &admission);

    assert!(!evidence.relevant_to_boss_plan);
    assert_eq!(evidence.score_delta, 0);
}

#[test]
fn multiplier_with_one_stable_source_keeps_reliability_repair() {
    let (deck, plan) = deck_plan(&[CardId::Inflame, CardId::LimitBreak]);
    let admission = assess_reward_admission_from_master_deck(&deck, CardId::DemonForm, 0);
    let evidence = assess_boss_scaling_evidence(plan, Some((CardId::DemonForm, 0)), &admission);

    assert!(evidence.relevant_to_boss_plan);
}

#[test]
fn conditional_source_does_not_masquerade_as_reliability_repair() {
    let (deck, plan) = deck_plan(&[CardId::Inflame, CardId::LimitBreak]);
    let admission = assess_reward_admission_from_master_deck(&deck, CardId::SpotWeakness, 1);
    let evidence = assess_boss_scaling_evidence(plan, Some((CardId::SpotWeakness, 1)), &admission);

    assert!(!evidence.relevant_to_boss_plan);
}
```

Add this lane-level test to the existing `decision_pipeline.rs` test module:

```rust
#[test]
fn unsupported_rupture_shop_purchase_is_not_mainline() {
    let rupture = shop_card(
        &[CardId::Strike, CardId::Defend, CardId::Bash],
        CardId::Rupture,
    );

    assert_ne!(rupture.lane, CandidateLane::Mainline);
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib ai::strategy::boss_scaling_evidence::tests -- --nocapture
cargo test --lib unsupported_rupture_shop_purchase_is_not_mainline -- --nocapture
```

Expected: the first, repeated, conditional-source, and shop-lane tests fail because every strength source currently receives `boss-scaling-source +70`; the multiplier repair may pass incidentally but protects the intended exception.

- [ ] **Step 3: Centralize the existing reliability predicate**

In `deck_plan.rs`, import `card_is_stable_strength_source`, `StrategicDeficitLevel`, and `CardId`, then add this method to `impl DeckPlanSnapshot`:

```rust
pub fn repairs_strength_package_reliability(
    self,
    candidate: Option<(CardId, u8)>,
) -> bool {
    self.roles.strength_multiplier_units > 0
        && self.roles.strength_source_units == 1
        && matches!(
            self.strategic_deficit.boss_scaling_plan,
            StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
        )
        && candidate
            .is_some_and(|(card, upgrades)| card_is_stable_strength_source(card, upgrades))
}
```

In `acquisition.rs`, replace the private helper call with:

```rust
let repairs_package_reliability =
    context.deck_plan.repairs_strength_package_reliability(candidate);
```

Delete the old private `repairs_strength_package_reliability` function and remove its now-unused `card_is_stable_strength_source` import.

- [ ] **Step 4: Implement marginal source evidence**

In `boss_scaling_evidence.rs`, import `RewardAdmissionClass`. Replace the unconditional strength-source branch with:

```rust
if admission_provides(admission, Mechanic::Strength)
    || card_grants_strength(card_semantics.as_ref())
{
    if admission.class == RewardAdmissionClass::OpensUnsupportedPayoff {
        return BossScalingEvidence::score_only("boss-unsupported-scaling-source", -35);
    }

    let existing_sources = deck
        .roles
        .strength_source_units
        .saturating_add(deck.roles.conditional_strength_source_units);
    if existing_sources == 0 {
        return BossScalingEvidence::relevant("boss-scaling-source", 70);
    }
    if deck.repairs_strength_package_reliability(card) {
        return BossScalingEvidence::relevant("boss-scaling-reliability", 70);
    }
    return BossScalingEvidence::score_only("boss-marginal-scaling-source", 0);
}
```

Do not change strength-payoff, exhaust, block, access, support, or boss-specific evidence branches.

- [ ] **Step 5: Preserve candidate identity through strategic-gap checks**

In `decision_pipeline.rs`, change:

```rust
fn improves_strategic_gap(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: &RewardAdmission,
) -> bool
```

Pass `kind` from `heavy_burden_penalty_applies`, and use:

```rust
assess_boss_scaling_evidence(context.deck_plan, candidate_card(kind), admission)
```

instead of passing `None`. This lets the existing multiplier reliability repair survive the heavy-burden exception without restoring unconditional source credit.

- [ ] **Step 6: Run focused boss-scaling and acquisition tests and verify GREEN**

```powershell
cargo test --lib ai::strategy::boss_scaling_evidence::tests -- --nocapture
cargo test --lib unsupported_rupture_shop_purchase_is_not_mainline -- --nocapture
cargo test --lib strength_package_reliability -- --nocapture
```

Expected: all new boss-scaling and shop-lane tests pass; the existing positive second-stable-source and negative third/conditional/payoff acquisition tests pass.

- [ ] **Step 7: Commit Task 2**

```powershell
git add src/ai/strategy/deck_plan.rs src/ai/strategy/acquisition.rs src/ai/strategy/boss_scaling_evidence.rs src/ai/strategy/decision_pipeline.rs
git commit -m "fix: require marginal boss scaling value"
```

### Task 3: Acute survival lane boundary

**Files:**
- Modify: `src/ai/strategy/acquisition.rs`
- Modify: `src/ai/strategy/decision_pipeline.rs`

**Interfaces:**
- Consumes: the existing survival-pressure predicate, card burdens, reward reasons, candidate identity, and boss-survival evidence.
- Produces: unchanged candidate-evaluation API; pure block qualifies as acute survival, while setup-only candidates are capped at `ProbeOnly` under survival pressure.

- [ ] **Step 1: Add a low-HP reward-context helper and failing lane tests**

Refactor the existing reward context helper in the `decision_pipeline.rs` test module:

```rust
fn reward_context_with_act(cards: &[CardId], act: u8) -> DecisionPipelineContext {
    reward_context_with_act_and_hp(cards, act, 70, 80)
}

fn reward_context_with_act_and_hp(
    cards: &[CardId],
    act: u8,
    current_hp: i32,
    max_hp: i32,
) -> DecisionPipelineContext {
    let deck = test_deck(cards);
    DecisionPipelineContext::reward(DeckPlanSnapshot::from_deck(
        &deck,
        DeckAdmissionContext {
            act,
            current_hp,
            max_hp,
        },
        RunStrategicFacts {
            entering_act: act,
            starter_basic_count: deck
                .iter()
                .filter(|card| matches!(card.id, CardId::Strike | CardId::Defend))
                .count(),
            curse_count: 0,
            has_energy_relic: false,
        },
    ))
}
```

Add an evaluator using that context:

```rust
fn reward_card_with_act_and_hp(
    cards: &[CardId],
    candidate: CardId,
    upgrades: u8,
    act: u8,
    current_hp: i32,
    max_hp: i32,
) -> CandidateEvaluation {
    let deck = test_deck(cards);
    let context = reward_context_with_act_and_hp(cards, act, current_hp, max_hp);
    let admission = assess_reward_admission_from_master_deck(&deck, candidate, upgrades);
    evaluate_decision_candidate(
        context,
        DecisionCandidateKind::CardRewardPick {
            card: candidate,
            upgrades,
        },
        Some(&admission),
    )
}
```

Add the stable invariant tests:

```rust
fn low_hp_heavy_burden_deck() -> Vec<CardId> {
    vec![
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
        CardId::PommelStrike,
        CardId::ShrugItOff,
        CardId::Armaments,
        CardId::Cleave,
        CardId::Cleave,
        CardId::Rupture,
        CardId::Apparition,
        CardId::Apparition,
        CardId::Apparition,
        CardId::Apparition,
        CardId::Apparition,
        CardId::Hemokinesis,
        CardId::ShrugItOff,
        CardId::Offering,
    ]
}

#[test]
fn low_hp_pure_block_survives_heavy_burden_lane_cap() {
    let deck = low_hp_heavy_burden_deck();
    let flame_barrier =
        reward_card_with_act_and_hp(&deck, CardId::FlameBarrier, 1, 3, 12, 39);

    assert_eq!(flame_barrier.lane, CandidateLane::Mainline);
    assert!(!flame_barrier
        .adjudication
        .caps
        .iter()
        .any(|cap| cap.source == CandidateLaneCapSource::Strategic));
}

#[test]
fn low_hp_redundant_rupture_cannot_enter_mainline() {
    let deck = low_hp_heavy_burden_deck();
    let rupture = reward_card_with_act_and_hp(&deck, CardId::Rupture, 1, 3, 13, 39);

    assert_ne!(rupture.lane, CandidateLane::Mainline);
}

#[test]
fn low_hp_setup_only_scaling_is_capped_below_mainline() {
    let deck = vec![
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
        CardId::PommelStrike,
        CardId::ShrugItOff,
        CardId::Cleave,
    ];
    let demon_form =
        reward_card_with_act_and_hp(&deck, CardId::DemonForm, 0, 3, 12, 39);

    assert_ne!(demon_form.lane, CandidateLane::Mainline);
    assert!(demon_form.adjudication.caps.iter().any(|cap| {
        cap.source == CandidateLaneCapSource::Strategic && cap.cap == LaneCap::ProbeOnly
    }));
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib low_hp_pure_block_survives_heavy_burden_lane_cap -- --nocapture
cargo test --lib low_hp_setup_only_scaling_is_capped_below_mainline -- --nocapture
```

Expected: Flame Barrier+ remains capped at probe because pure block is not currently a heavy-burden survival exception; Demon Form remains mainline because no acute-survival setup cap exists.

- [ ] **Step 3: Admit pure block as acute survival**

Replace `survival_pressure_exception` with:

```rust
fn survival_pressure_exception(
    context: DecisionPipelineContext,
    admission: &RewardAdmission,
) -> bool {
    context.deck_plan.survival_pressure()
        && (admission_provides(admission, Mechanic::Block)
            || admission_provides(admission, Mechanic::EnemyStrengthDown)
            || admission_provides(admission, Mechanic::Weak))
}
```

This removes the heavy-burden penalty and strategic cap from pure immediate block only while survival pressure is active.

The acquisition boundary also treats the same acute survival tool as an exception to generic
two-cost deployability debt. Add this condition to `adds_deployability_debt` before the energy
and expensive-card checks:

```rust
&& !(deck_plan.survival_pressure() && admission_survival_tool(admission))
```

Without this paired exception, the candidate has a mainline raw score and no strategic cap but
is still demoted by the acquisition contract before it can serve as a survival stabilizer.

- [ ] **Step 4: Add the categorical setup lane cap**

At the start of `strategic_lane_cap`, after obtaining `admission`, return `ProbeOnly` when this helper is true:

```rust
fn acute_survival_setup_only(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: &RewardAdmission,
) -> bool {
    if !context.deck_plan.survival_pressure()
        || admission.class == RewardAdmissionClass::ClosesRequirement
        || assess_boss_survival_evidence(
            context.deck_plan,
            candidate_card(kind),
            admission,
        )
        .relevant_to_boss_survival_plan
    {
        return false;
    }

    let immediate_survival_or_access = admission_survival_tool(admission)
        || admission_frontloads(admission)
        || admission_provides(admission, Mechanic::CardDraw)
        || admission_provides(admission, Mechanic::Energy)
        || admission
            .reasons
            .contains(&RewardAdmissionReason::RecoverCurrentHp);
    if immediate_survival_or_access {
        return false;
    }

    let setup_burden = candidate_card(kind).is_some_and(|(card, upgrades)| {
        card_definition_with_upgrades(card, upgrades)
            .burdens
            .contains(&CardBurden::PowerSetup)
    });
    setup_burden || admission_scaling_or_engine(admission)
}
```

The resulting `strategic_lane_cap` begins with:

```rust
let admission = admission?;
if acute_survival_setup_only(context, kind, admission) {
    return Some(LaneCap::ProbeOnly);
}
if !heavy_burden_penalty_applies(context, kind, admission) {
    return None;
}
Some(LaneCap::ProbeOnly)
```

- [ ] **Step 5: Run all three low-HP tests and verify GREEN**

```powershell
cargo test --lib low_hp_pure_block_survives_heavy_burden_lane_cap -- --nocapture
cargo test --lib low_hp_redundant_rupture_cannot_enter_mainline -- --nocapture
cargo test --lib low_hp_setup_only_scaling_is_capped_below_mainline -- --nocapture
```

Expected: all three tests pass. Flame Barrier+ is mainline; redundant Rupture and setup-only scaling are below mainline.

- [ ] **Step 6: Run adjacent decision-pipeline tests**

```powershell
cargo test --lib ai::strategy::decision_pipeline::tests -- --nocapture
```

Expected: all decision-pipeline tests pass with zero failures.

- [ ] **Step 7: Commit Task 3**

```powershell
git add docs/superpowers/plans/2026-07-11-marginal-acquisition-survival-gate.md src/ai/strategy/acquisition.rs src/ai/strategy/decision_pipeline.rs
git commit -m "fix: prioritize acute survival over setup"
```

### Task 4: Repository verification

**Files:**
- No additional source files.

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: fresh library, architecture, formatting, and repository-hygiene evidence.

- [ ] **Step 1: Run the complete library suite**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib
```

Expected: all library tests pass with zero failures.

- [ ] **Step 2: Run architecture and formatting checks**

```powershell
cargo test --test architecture_runtime_boundaries
cargo fmt --all -- --check
git diff --check
```

Expected: seven architecture tests pass; formatting and whitespace checks exit successfully.

- [ ] **Step 3: Confirm scope and history**

```powershell
git status --short --branch
git log -4 --oneline
git diff HEAD~3 -- src/ai/strategy/reward_admission.rs src/ai/strategy/deck_plan.rs src/ai/strategy/acquisition.rs src/ai/strategy/boss_scaling_evidence.rs src/ai/strategy/decision_pipeline.rs
```

Expected: only the planned strategy files and their focused tests changed across the three implementation commits; the design and plan remain separate earlier commits; no relic-combination or combat-search files changed.
