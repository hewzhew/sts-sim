# Combat-Upgrade Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make permanent upgrade planning and campfire Smith decisions account conservatively for Armaments, Armaments+, and Apotheosis combat-upgrade coverage.

**Architecture:** Add one typed fact producer for exact combat-upgrade scope, derive existing startup compatibility facts from it, and let the upgrade planner grant at most one ordinal level of candidate-specific credit. Campfire policy consumes the adjusted planner output unchanged; run-control owner remains untouched.

**Tech Stack:** Rust 2021, existing `RunState`, `DeckStartupProfileV1`, `UpgradePlanV1`, `campfire_policy_v1`, Serde compatibility tests, and Cargo unit/architecture tests.

## Global Constraints

- Do not add Armaments, Apotheosis, or card-delta matching to campfire policy or campfire owner.
- Do not model exact Armaments/target co-draw probability.
- Do not run combat search or campaign counterfactuals from campfire policy.
- Combat-upgrade credit lowers an eligible candidate by at most one ordinal level and never stacks.
- Never lower `ImportantBeforeBoss` or `CriticalBeforeBoss` urgency.
- Never lower boss-specific, timing-sensitive, Innate, Armaments-self, or Apotheosis-self upgrade value.
- Preserve existing serialized startup fields; add `combat_upgrade_all_access_count` with a Serde default.
- Do not change recovery pressure, HP thresholds, Smith thresholds, route risk, or owner fallback order.
- Add only categorical relationship tests; do not add complete-seed, random-draw, boss-outcome, or exact-total-score assertions.
- Do not use subagents for execution while the current Codex app effort-control limitation remains.

## File structure

- Create `src/ai/combat_upgrade_coverage_v1.rs`: exact source/scope facts only; no policy scores.
- Modify `src/ai/mod.rs`: export the new analysis module.
- Modify `src/ai/deck_startup_profile_v1.rs`: derive compatibility counters and Pyramid repair availability from the new facts.
- Modify `src/ai/upgrade_planner_v1.rs`: classify candidates, apply bounded credit, and expose typed evidence.
- Modify `src/ai/campfire_policy_v1/tests.rs`: one policy-boundary relationship test; no production campfire changes.

---

### Task 1: Exact combat-upgrade scope and startup compatibility

**Files:**
- Create: `src/ai/combat_upgrade_coverage_v1.rs`
- Modify: `src/ai/mod.rs`
- Modify: `src/ai/deck_startup_profile_v1.rs`
- Test: `src/ai/combat_upgrade_coverage_v1.rs`
- Test: `src/ai/deck_startup_profile_v1.rs`

**Interfaces:**
- Consumes: `combat_upgrade_coverage_profile_v1(&RunState)` reads master-deck card ids, upgrades, and deck indices.
- Produces: `CombatUpgradeCoverageProfileV1`, `CombatUpgradeSourceV1`, `CombatUpgradeScopeV1`, `source_count`, `has_scope`, and `strongest_scope` for Task 2.
- Preserves: existing serialized startup fields while adding `combat_upgrade_all_access_count: u8` with `#[serde(default)]`.

- [ ] **Step 1: Add the module boundary and a failing exact-scope test**

Add this declaration in `src/ai/mod.rs`:

```rust
pub mod combat_upgrade_coverage_v1;
```

Create `src/ai/combat_upgrade_coverage_v1.rs` with the failing test first:

```rust
use crate::content::cards::CardId;
use crate::state::run::RunState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::combat::CombatCard;

    #[test]
    fn distinguishes_selected_hand_whole_hand_and_all_combat_zones() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        let armaments = CombatCard::new(CardId::Armaments, 1001);
        let mut armaments_plus = CombatCard::new(CardId::Armaments, 1002);
        armaments_plus.upgrades = 1;
        let apotheosis = CombatCard::new(CardId::Apotheosis, 1003);
        run.master_deck = vec![armaments, armaments_plus, apotheosis];

        let profile = combat_upgrade_coverage_profile_v1(&run);

        assert_eq!(
            profile.sources.iter().map(|source| source.scope).collect::<Vec<_>>(),
            vec![
                CombatUpgradeScopeV1::SelectedCardInHand,
                CombatUpgradeScopeV1::WholeHand,
                CombatUpgradeScopeV1::AllCombatZones,
            ]
        );
        assert_eq!(
            profile.source_count(CombatUpgradeScopeV1::SelectedCardInHand),
            1
        );
        assert_eq!(profile.strongest_scope(), Some(CombatUpgradeScopeV1::AllCombatZones));
    }
}
```

- [ ] **Step 2: Run the new test and verify RED**

```powershell
cargo test --lib ai::combat_upgrade_coverage_v1::tests -- --nocapture
```

Expected: compilation fails because the fact function and types do not exist.

- [ ] **Step 3: Implement the exact fact producer**

Insert this implementation above the test module:

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CombatUpgradeScopeV1 {
    SelectedCardInHand,
    WholeHand,
    AllCombatZones,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatUpgradeSourceV1 {
    pub deck_index: usize,
    pub card: CardId,
    pub upgrades: u8,
    pub scope: CombatUpgradeScopeV1,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CombatUpgradeCoverageProfileV1 {
    pub sources: Vec<CombatUpgradeSourceV1>,
}

impl CombatUpgradeCoverageProfileV1 {
    pub fn source_count(&self, scope: CombatUpgradeScopeV1) -> u8 {
        self.sources
            .iter()
            .filter(|source| source.scope == scope)
            .count()
            .min(u8::MAX as usize) as u8
    }

    pub fn has_scope(&self, scope: CombatUpgradeScopeV1) -> bool {
        self.sources.iter().any(|source| source.scope == scope)
    }

    pub fn strongest_scope(&self) -> Option<CombatUpgradeScopeV1> {
        self.sources.iter().map(|source| source.scope).max()
    }
}

pub fn combat_upgrade_coverage_profile_v1(
    run_state: &RunState,
) -> CombatUpgradeCoverageProfileV1 {
    let sources = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter_map(|(deck_index, card)| {
            let scope = match card.id {
                CardId::Armaments if card.upgrades > 0 => CombatUpgradeScopeV1::WholeHand,
                CardId::Armaments => CombatUpgradeScopeV1::SelectedCardInHand,
                CardId::Apotheosis => CombatUpgradeScopeV1::AllCombatZones,
                _ => return None,
            };
            Some(CombatUpgradeSourceV1 {
                deck_index,
                card: card.id,
                upgrades: card.upgrades,
                scope,
            })
        })
        .collect();

    CombatUpgradeCoverageProfileV1 { sources }
}
```

- [ ] **Step 4: Run the exact-scope test and verify GREEN**

```powershell
cargo test --lib ai::combat_upgrade_coverage_v1::tests -- --nocapture
```

Expected: one test passes with zero failures.

- [ ] **Step 5: Add failing startup compatibility tests**

Add this test in `src/ai/deck_startup_profile_v1.rs`:

```rust
#[test]
fn startup_profile_keeps_apotheosis_distinct_from_whole_hand_access() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = vec![crate::runtime::combat::CombatCard::new(
        CardId::Apotheosis,
        1001,
    )];

    let profile = deck_startup_profile_v1(&run);

    assert_eq!(profile.combat_upgrade_selected_access_count, 0);
    assert_eq!(profile.combat_upgrade_hand_access_count, 0);
    assert_eq!(profile.combat_upgrade_all_access_count, 1);
}
```

Add `"combat_upgrade_all_access_count"` to the field-removal list in `older_serialized_startup_profiles_default_new_capacity_fields`, then add:

```rust
assert_eq!(decoded.combat_upgrade_all_access_count, 0);
```

- [ ] **Step 6: Run startup tests and verify RED**

```powershell
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
```

Expected: compilation fails because `DeckStartupProfileV1` has no `combat_upgrade_all_access_count` field.

- [ ] **Step 7: Derive startup counters from the typed facts**

Add this import:

```rust
use crate::ai::combat_upgrade_coverage_v1::{
    combat_upgrade_coverage_profile_v1, CombatUpgradeScopeV1,
};
```

Add this field after `combat_upgrade_hand_access_count`:

```rust
#[serde(default)]
pub combat_upgrade_all_access_count: u8,
```

Compute coverage before constructing the startup profile and initialize all three counters:

```rust
let combat_upgrade_coverage = combat_upgrade_coverage_profile_v1(run_state);
let mut profile = DeckStartupProfileV1 {
    has_runic_pyramid: run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::RunicPyramid),
    has_snecko_eye: run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::SneckoEye),
    has_velvet_choker: run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::VelvetChoker),
    combat_upgrade_selected_access_count: combat_upgrade_coverage
        .source_count(CombatUpgradeScopeV1::SelectedCardInHand),
    combat_upgrade_hand_access_count: combat_upgrade_coverage
        .source_count(CombatUpgradeScopeV1::WholeHand),
    combat_upgrade_all_access_count: combat_upgrade_coverage
        .source_count(CombatUpgradeScopeV1::AllCombatZones),
    ..Default::default()
};
```

Delete the existing Armaments/Apotheosis match that increments the two old access counters. Keep the separate startup-key match that counts Armaments cards.

Replace the repair-access condition in `pyramid_apparition_coverage_v1` with:

```rust
} else if profile
    .combat_upgrade_selected_access_count
    .saturating_add(profile.combat_upgrade_hand_access_count)
    .saturating_add(profile.combat_upgrade_all_access_count)
    > 0
{
    PyramidApparitionCoverageV1::CombatRepairAvailable
```

- [ ] **Step 8: Verify Task 1 GREEN**

```powershell
cargo test --lib ai::combat_upgrade_coverage_v1::tests -- --nocapture
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Expected: both focused suites pass; Armaments+ remains whole-hand access and Apotheosis becomes all-combat-zone access.

- [ ] **Step 9: Commit the fact boundary**

```powershell
git add -- src/ai/mod.rs src/ai/combat_upgrade_coverage_v1.rs src/ai/deck_startup_profile_v1.rs
git commit -m "feat: model combat upgrade coverage"
```

---

### Task 2: Bounded planner credit and campfire consumption

**Files:**
- Modify: `src/ai/upgrade_planner_v1.rs`
- Modify: `src/ai/campfire_policy_v1/tests.rs`
- Test: `src/ai/upgrade_planner_v1.rs`
- Test: `src/ai/campfire_policy_v1/tests.rs`

**Interfaces:**
- Consumes: `CombatUpgradeCoverageProfileV1` and `CombatUpgradeScopeV1` from Task 1.
- Produces: `UpgradePlanV1.combat_upgrade_coverage`, `UpgradeCandidateV1.combat_upgrade_class`, and `UpgradeCandidateV1.combat_upgrade_credit`.
- Preserves: existing campfire production code, HP/recovery gates, strategy tags, score thresholds, and owner delegation.

- [ ] **Step 1: Add failing planner relationship tests**

Add these tests to `src/ai/upgrade_planner_v1.rs`:

```rust
#[test]
fn selected_armaments_credit_is_limited_to_ordinary_starter_targets() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = vec![
        CombatCard::new(CardId::Armaments, 1001),
        CombatCard::new(CardId::Strike, 1002),
        CombatCard::new(CardId::Cleave, 1003),
    ];

    let plan = plan_upgrades_v1(&run);
    let strike = plan
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Strike)
        .expect("Strike should remain an upgrade candidate");
    let cleave = plan
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Cleave)
        .expect("Cleave should remain an upgrade candidate");

    assert_eq!(strike.combat_upgrade_class, CombatUpgradeCandidateClassV1::OrdinaryStat);
    assert!(matches!(
        strike.combat_upgrade_credit,
        CombatUpgradeCreditV1::PriorityReduced {
            scope: CombatUpgradeScopeV1::SelectedCardInHand,
            previous_urgency: UpgradeDebtSeverityV1::Opportunistic,
            resulting_urgency: UpgradeDebtSeverityV1::Defer,
        }
    ));
    assert_eq!(strike.verdict, UpgradeVerdictV1::Defer);
    assert_eq!(cleave.combat_upgrade_class, CombatUpgradeCandidateClassV1::OrdinaryStat);
    assert_eq!(
        cleave.combat_upgrade_credit,
        CombatUpgradeCreditV1::EvidenceOnly {
            scope: CombatUpgradeScopeV1::SelectedCardInHand,
        }
    );
    assert_eq!(cleave.urgency, UpgradeDebtSeverityV1::Opportunistic);
}

#[test]
fn armaments_plus_reduces_an_ordinary_nonstarter_by_one_level() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    let mut armaments_plus = CombatCard::new(CardId::Armaments, 1001);
    armaments_plus.upgrades = 1;
    run.master_deck = vec![armaments_plus, CombatCard::new(CardId::Cleave, 1002)];

    let plan = plan_upgrades_v1(&run);
    let cleave = plan
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Cleave)
        .expect("Cleave should remain an upgrade candidate");

    assert_eq!(cleave.combat_upgrade_class, CombatUpgradeCandidateClassV1::OrdinaryStat);
    assert!(matches!(
        cleave.combat_upgrade_credit,
        CombatUpgradeCreditV1::PriorityReduced {
            scope: CombatUpgradeScopeV1::WholeHand,
            previous_urgency: UpgradeDebtSeverityV1::Opportunistic,
            resulting_urgency: UpgradeDebtSeverityV1::Defer,
        }
    ));
    assert_eq!(cleave.verdict, UpgradeVerdictV1::Defer);
}

#[test]
fn combat_upgrade_coverage_does_not_discount_providers_innate_or_phase_burst() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.act_num = 2;
    run.boss_key = Some(EncounterId::TheChamp);
    let mut armaments_plus = CombatCard::new(CardId::Armaments, 1001);
    armaments_plus.upgrades = 1;
    run.master_deck = vec![
        armaments_plus,
        CombatCard::new(CardId::Armaments, 1002),
        CombatCard::new(CardId::BootSequence, 1003),
        CombatCard::new(CardId::Whirlwind, 1004),
    ];

    let plan = plan_upgrades_v1(&run);
    let candidate = |card| {
        plan.candidates
            .iter()
            .find(|candidate| candidate.card == card)
            .expect("protected card should remain an upgrade candidate")
    };

    assert_eq!(
        candidate(CardId::Armaments).combat_upgrade_class,
        CombatUpgradeCandidateClassV1::NoCombatCredit
    );
    assert_eq!(
        candidate(CardId::BootSequence).combat_upgrade_class,
        CombatUpgradeCandidateClassV1::NoCombatCredit
    );
    assert_eq!(
        candidate(CardId::Whirlwind).combat_upgrade_class,
        CombatUpgradeCandidateClassV1::TimingSensitive
    );
    assert_eq!(
        candidate(CardId::Whirlwind).urgency,
        UpgradeDebtSeverityV1::ImportantBeforeBoss
    );
    assert!(!matches!(
        candidate(CardId::Whirlwind).combat_upgrade_credit,
        CombatUpgradeCreditV1::PriorityReduced { .. }
    ));
}
```

- [ ] **Step 2: Add the failing campfire-boundary test**

Extend imports in `src/ai/campfire_policy_v1/tests.rs`:

```rust
use crate::ai::combat_upgrade_coverage_v1::CombatUpgradeScopeV1;
use crate::ai::upgrade_planner_v1::{
    plan_upgrades_v1, CombatUpgradeCreditV1, UpgradeVerdictV1,
};
use crate::runtime::combat::CombatCard;
```

Add this test before the route helpers:

```rust
#[test]
fn combat_repairable_transitional_attack_no_longer_clears_smith_gate() {
    let mut baseline = RunState::new(1, 0, false, "Ironclad");
    baseline.current_hp = baseline.max_hp;
    baseline.master_deck = vec![CombatCard::new(CardId::PerfectedStrike, 1001)];

    let baseline_context = build_campfire_decision_context_v1(
        &baseline,
        vec![CampfireChoice::Smith(0)],
    );
    let baseline_decision =
        plan_campfire_decision_v1(&baseline_context, &CampfirePolicyConfigV1::default());
    assert!(matches!(
        baseline_decision.action,
        CampfirePolicyActionV1::Smith { .. }
    ));

    let mut covered = baseline.clone();
    let mut armaments_plus = CombatCard::new(CardId::Armaments, 1002);
    armaments_plus.upgrades = 1;
    covered.master_deck.insert(0, armaments_plus);

    let upgrade_plan = plan_upgrades_v1(&covered);
    let perfected_strike = upgrade_plan
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::PerfectedStrike)
        .expect("Perfected Strike should remain an upgrade candidate");
    assert_eq!(perfected_strike.verdict, UpgradeVerdictV1::Defer);
    assert!(matches!(
        perfected_strike.combat_upgrade_credit,
        CombatUpgradeCreditV1::PriorityReduced {
            scope: CombatUpgradeScopeV1::WholeHand,
            ..
        }
    ));

    let covered_context = build_campfire_decision_context_v1(
        &covered,
        vec![CampfireChoice::Smith(1)],
    );
    let covered_decision =
        plan_campfire_decision_v1(&covered_context, &CampfirePolicyConfigV1::default());
    assert!(matches!(
        covered_decision.action,
        CampfirePolicyActionV1::Stop { .. }
    ));
}
```

- [ ] **Step 3: Run planner and campfire tests and verify RED**

```powershell
cargo test --lib ai::upgrade_planner_v1::tests -- --nocapture
cargo test --lib ai::campfire_policy_v1::tests::combat_repairable_transitional_attack_no_longer_clears_smith_gate -- --nocapture
```

Expected: compilation fails because the planner does not expose the new class/credit types or fields. After type skeletons exist, relationship assertions still fail because urgency is unchanged.

- [ ] **Step 4: Add typed planner fields and imports**

Add this import to `src/ai/upgrade_planner_v1.rs`:

```rust
use crate::ai::combat_upgrade_coverage_v1::{
    combat_upgrade_coverage_profile_v1, CombatUpgradeCoverageProfileV1,
    CombatUpgradeScopeV1,
};
```

Add this field to `UpgradePlanV1`:

```rust
pub combat_upgrade_coverage: CombatUpgradeCoverageProfileV1,
```

Add these fields to `UpgradeCandidateV1` after `mechanical_delta`:

```rust
pub combat_upgrade_class: CombatUpgradeCandidateClassV1,
pub combat_upgrade_credit: CombatUpgradeCreditV1,
```

Add these types after `UpgradeMechanicalDeltaV1`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatUpgradeCandidateClassV1 {
    OrdinaryStat,
    TimingSensitive,
    NoCombatCredit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatUpgradeCreditV1 {
    None,
    EvidenceOnly {
        scope: CombatUpgradeScopeV1,
    },
    PriorityReduced {
        scope: CombatUpgradeScopeV1,
        previous_urgency: UpgradeDebtSeverityV1,
        resulting_urgency: UpgradeDebtSeverityV1,
    },
}
```

Initialize new candidates in `build_upgrade_candidate`:

```rust
combat_upgrade_class: CombatUpgradeCandidateClassV1::NoCombatCredit,
combat_upgrade_credit: CombatUpgradeCreditV1::None,
```

- [ ] **Step 5: Refactor verdict derivation so a reduced urgency is representable**

Replace the final verdict assignment inside `apply_debt_ledger` with:

```rust
candidate.verdict = upgrade_verdict_for_candidate(candidate);
```

Add this helper immediately after `apply_debt_ledger`:

```rust
fn upgrade_verdict_for_candidate(candidate: &UpgradeCandidateV1) -> UpgradeVerdictV1 {
    if candidate.roles.contains(&UpgradeRoleV1::LowMarginalRepeat) {
        return if candidate.urgency >= UpgradeDebtSeverityV1::ImportantBeforeBoss {
            UpgradeVerdictV1::Defer
        } else {
            UpgradeVerdictV1::Avoid
        };
    }

    match candidate.urgency {
        UpgradeDebtSeverityV1::CriticalBeforeBoss => UpgradeVerdictV1::CoreDebtPayment,
        UpgradeDebtSeverityV1::ImportantBeforeBoss => UpgradeVerdictV1::Important,
        UpgradeDebtSeverityV1::UsefulSoon => UpgradeVerdictV1::Useful,
        UpgradeDebtSeverityV1::Opportunistic => UpgradeVerdictV1::Opportunistic,
        UpgradeDebtSeverityV1::Defer => UpgradeVerdictV1::Defer,
        UpgradeDebtSeverityV1::Avoid => UpgradeVerdictV1::Avoid,
    }
}
```

- [ ] **Step 6: Implement fail-closed classification and bounded credit**

Add these helpers before `rest_vs_smith_plan`:

```rust
fn combat_upgrade_candidate_class(
    candidate: &UpgradeCandidateV1,
) -> CombatUpgradeCandidateClassV1 {
    if matches!(candidate.card, CardId::Armaments | CardId::Apotheosis)
        || candidate.mechanical_delta.innate_delta
    {
        return CombatUpgradeCandidateClassV1::NoCombatCredit;
    }
    if candidate.mechanical_delta.cost_delta != 0
        || candidate.mechanical_delta.exhaust_control_delta
        || candidate.mechanical_delta.exhaust_removed_delta
        || candidate.mechanical_delta.ethereal_removed_delta
    {
        return CombatUpgradeCandidateClassV1::TimingSensitive;
    }

    let timing_sensitive_role = candidate.roles.iter().any(|role| {
        !matches!(
            role,
            UpgradeRoleV1::FrontloadDamage
                | UpgradeRoleV1::TransitionalPower
                | UpgradeRoleV1::LowMarginalRepeat
                | UpgradeRoleV1::Speculative
        )
    });
    if timing_sensitive_role {
        return CombatUpgradeCandidateClassV1::TimingSensitive;
    }

    if candidate.mechanical_delta.damage_delta > 0
        || candidate.mechanical_delta.block_delta > 0
        || candidate.mechanical_delta.magic_delta != 0
    {
        CombatUpgradeCandidateClassV1::OrdinaryStat
    } else {
        CombatUpgradeCandidateClassV1::NoCombatCredit
    }
}

fn lower_upgrade_urgency_one_level(
    urgency: UpgradeDebtSeverityV1,
) -> UpgradeDebtSeverityV1 {
    match urgency {
        UpgradeDebtSeverityV1::CriticalBeforeBoss
        | UpgradeDebtSeverityV1::ImportantBeforeBoss => urgency,
        UpgradeDebtSeverityV1::UsefulSoon => UpgradeDebtSeverityV1::Opportunistic,
        UpgradeDebtSeverityV1::Opportunistic => UpgradeDebtSeverityV1::Defer,
        UpgradeDebtSeverityV1::Defer => UpgradeDebtSeverityV1::Avoid,
        UpgradeDebtSeverityV1::Avoid => UpgradeDebtSeverityV1::Avoid,
    }
}

fn combat_upgrade_source_evidence(
    coverage: &CombatUpgradeCoverageProfileV1,
) -> String {
    coverage
        .sources
        .iter()
        .map(|source| {
            format!(
                "{:?}@{}+{}:{:?}",
                source.card, source.deck_index, source.upgrades, source.scope
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn apply_combat_upgrade_coverage(
    candidate: &mut UpgradeCandidateV1,
    coverage: &CombatUpgradeCoverageProfileV1,
) {
    candidate.combat_upgrade_class = combat_upgrade_candidate_class(candidate);
    let Some(scope) = coverage.strongest_scope() else {
        candidate.combat_upgrade_credit = CombatUpgradeCreditV1::None;
        return;
    };

    candidate.evidence.push(format!(
        "combat upgrade coverage sources={} class={:?}",
        combat_upgrade_source_evidence(coverage),
        candidate.combat_upgrade_class
    ));
    if candidate.combat_upgrade_class != CombatUpgradeCandidateClassV1::OrdinaryStat {
        candidate.evidence.push(match candidate.combat_upgrade_class {
            CombatUpgradeCandidateClassV1::TimingSensitive =>
                "combat upgrade credit withheld: timing-sensitive or unrecognized role/delta",
            CombatUpgradeCandidateClassV1::NoCombatCredit =>
                "combat upgrade credit withheld: provider, innate, or no ordinary stat delta",
            CombatUpgradeCandidateClassV1::OrdinaryStat => unreachable!(),
        }.to_string());
        candidate.combat_upgrade_credit = CombatUpgradeCreditV1::EvidenceOnly { scope };
        return;
    }

    let broad_coverage = coverage.has_scope(CombatUpgradeScopeV1::WholeHand)
        || coverage.has_scope(CombatUpgradeScopeV1::AllCombatZones);
    let selected_coverage_applies = coverage.has_scope(CombatUpgradeScopeV1::SelectedCardInHand)
        && (is_starter(candidate.card)
            || candidate.roles.contains(&UpgradeRoleV1::LowMarginalRepeat));
    if !broad_coverage && !selected_coverage_applies {
        candidate.evidence.push(
            "combat upgrade credit withheld: selected-card access is too narrow for this target"
                .to_string(),
        );
        candidate.combat_upgrade_credit = CombatUpgradeCreditV1::EvidenceOnly { scope };
        return;
    }
    if candidate.urgency >= UpgradeDebtSeverityV1::ImportantBeforeBoss {
        candidate.evidence.push(
            "combat upgrade credit withheld: high-severity permanent upgrade debt".to_string(),
        );
        candidate.combat_upgrade_credit = CombatUpgradeCreditV1::EvidenceOnly { scope };
        return;
    }

    let previous_urgency = candidate.urgency;
    let resulting_urgency = lower_upgrade_urgency_one_level(previous_urgency);
    if resulting_urgency == previous_urgency {
        candidate.combat_upgrade_credit = CombatUpgradeCreditV1::EvidenceOnly { scope };
        return;
    }

    candidate.urgency = resulting_urgency;
    candidate.verdict = upgrade_verdict_for_candidate(candidate);
    candidate.combat_upgrade_credit = CombatUpgradeCreditV1::PriorityReduced {
        scope,
        previous_urgency,
        resulting_urgency,
    };
    candidate.evidence.push(format!(
        "combat upgrade coverage reduced urgency {:?}->{:?}",
        previous_urgency, resulting_urgency
    ));
}
```

- [ ] **Step 7: Wire coverage into planning and evidence**

Update `plan_upgrades_v1` to compute coverage once, apply it after debt annotation, and return it:

```rust
pub fn plan_upgrades_v1(run_state: &RunState) -> UpgradePlanV1 {
    let combat_upgrade_coverage = combat_upgrade_coverage_profile_v1(run_state);
    let mut candidates = enumerate_upgrade_candidates(run_state);
    let debt_ledger = build_upgrade_debt_ledger(run_state, &candidates);
    for candidate in &mut candidates {
        apply_debt_ledger(candidate, &debt_ledger);
        apply_combat_upgrade_coverage(candidate, &combat_upgrade_coverage);
    }
    candidates.sort_by(compare_upgrade_candidates);
    let best_smith = candidates.first().map(|candidate| candidate.deck_index);
    let rest_vs_smith = rest_vs_smith_plan(run_state, &candidates);
    let mut notes = Vec::new();
    if candidates
        .iter()
        .any(|candidate| candidate.roles.contains(&UpgradeRoleV1::LowMarginalRepeat))
    {
        notes.push("upgrade planner detected low-marginal repeat upgrade targets".to_string());
    }
    if debt_ledger.unpaid_core_count > 0 {
        notes.push(format!(
            "upgrade planner has {} unpaid core upgrade debt(s)",
            debt_ledger.unpaid_core_count
        ));
    }

    UpgradePlanV1 {
        combat_upgrade_coverage,
        candidates,
        debt_ledger,
        rest_vs_smith,
        best_smith,
        notes,
    }
}
```

In `upgrade_plan_evidence_for_deck_index_v1`, add this immediately after the candidate summary:

```rust
evidence.push(format!(
    "combat_upgrade_coverage: class={:?} credit={:?}",
    candidate.combat_upgrade_class, candidate.combat_upgrade_credit
));
```

Do not modify production files under `campfire_policy_v1` or `runtime/branch/owner_audit`.

Do not add a Rest-vs-Smith reason for the ordinary transitional fixture: before coverage its
`Opportunistic` debt does not satisfy the existing `best_smith_debt_paid` requirement, so combat
coverage changes the campfire Smith gate but does not change Rest-vs-Smith debt justification.

- [ ] **Step 8: Verify Task 2 GREEN**

```powershell
cargo test --lib ai::upgrade_planner_v1::tests -- --nocapture
cargo test --lib ai::campfire_policy_v1::tests -- --nocapture
cargo fmt --all -- --check
git diff --check
```

Expected: planner tests prove bounded credit and protected targets; campfire tests prove baseline Smith versus covered Stop for Perfected Strike. No exact score is asserted.

- [ ] **Step 9: Commit planner and policy-boundary behavior**

```powershell
git add -- src/ai/upgrade_planner_v1.rs src/ai/campfire_policy_v1/tests.rs
git commit -m "feat: discount combat-repairable upgrades"
```

---

### Task 3: Complete verification and scope audit

**Files:**
- Verify: `src/ai/combat_upgrade_coverage_v1.rs`
- Verify: `src/ai/deck_startup_profile_v1.rs`
- Verify: `src/ai/upgrade_planner_v1.rs`
- Verify: `src/ai/campfire_policy_v1/tests.rs`
- Verify unchanged: `src/ai/campfire_policy_v1/policy.rs`
- Verify unchanged: `src/ai/campfire_policy_v1/evaluator.rs`
- Verify unchanged: `src/runtime/branch/owner_audit/campfire_owner.rs`

**Interfaces:**
- Consumes: committed outputs of Tasks 1 and 2.
- Produces: fresh repository-wide evidence that formatting, library behavior, and architecture boundaries remain valid.

- [ ] **Step 1: Run formatting and focused suites once more**

```powershell
cargo fmt --all -- --check
cargo test --lib ai::combat_upgrade_coverage_v1::tests -- --nocapture
cargo test --lib ai::deck_startup_profile_v1::tests -- --nocapture
cargo test --lib ai::upgrade_planner_v1::tests -- --nocapture
cargo test --lib ai::campfire_policy_v1::tests -- --nocapture
git diff --check
```

Expected: every command exits zero with no failed tests.

- [ ] **Step 2: Run the large Rust test binary only once**

```powershell
cargo test --lib
```

Expected: all library tests pass. Do not rerun this command for each filter; the focused suites reuse the compiled artifact and this is the single repository-wide library invocation.

- [ ] **Step 3: Run architecture boundaries once**

```powershell
cargo test --test architecture_runtime_boundaries
```

Expected: all architecture boundary tests pass, including owner delegation rather than embedded strategic card knowledge.

- [ ] **Step 4: Audit scope and commits**

```powershell
git status --short --branch
git diff HEAD~2..HEAD --stat
git diff HEAD~2..HEAD -- src/ai/campfire_policy_v1/policy.rs src/ai/campfire_policy_v1/evaluator.rs src/runtime/branch/owner_audit/campfire_owner.rs
git log -2 --oneline
```

Expected: the worktree is clean; only the five planned source/test files changed across the two implementation commits; production campfire policy/evaluator and campfire owner have an empty diff.
