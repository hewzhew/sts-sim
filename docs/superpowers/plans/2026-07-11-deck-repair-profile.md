# Deck Repair Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hidden-information-safe Pandora offer evidence plus a score-free deck-repair profile that can admit narrowly safe functional-card purges and prioritize reliability-repair campfire upgrades.

**Architecture:** Keep offer-time uncertainty separate from the concrete post-mutation deck. Two focused AI modules derive `PandoraOfferProfileV1` and `DeckRepairProfileV1` from public `RunState`; boss-relic admission only renders the offer profile, while shop and campfire consume exact repair candidates through their existing policy/evaluator boundaries. No persisted lifecycle state, random-result preview, reward rewrite, combat-search change, or boss-relic rank change is introduced.

**Tech Stack:** Rust, existing `RunState`/card semantics, `DeckMutationCompilerV1`, `upgrade_planner_v1`, Serde-derived diagnostic facts, Cargo unit and architecture tests.

## Global Constraints

- Do not inspect or predict the seeded identities Pandora will generate before the relic resolves.
- Do not add a persistent Pandora lifecycle flag, transformed-card provenance, checkpoint field, or new persisted schema.
- Do not assign one aggregate Pandora, energy-gap, or deck-repair score.
- Do not change card-reward behavior, combat action ordering, combat state value, Collector tactics, or boss-relic ranking.
- Do not admit arbitrary functional-card purges; only exact `RedundantFunctional` targets with no currently thin/missing supplied function qualify.
- Preserve the existing rest-versus-smith safety boundary.
- Tests assert typed relationships and protection gates, not exact seed paths, random Pandora outcomes, aggregate scores, or boss wins.
- Do not run a full seed as a regression test for this implementation.

---

## File Structure

- Create `src/ai/pandora_offer_profile_v1.rs`: public-information-only Pandora transform opportunity and volatility facts.
- Create `src/ai/deck_repair_profile_v1.rs`: current-deck repair functions, safe removal candidates, and reliability upgrade candidates.
- Modify `src/ai/mod.rs`: export both focused modules.
- Modify `src/ai/strategy/boss_relic_admission.rs`: attach Pandora offer facts to existing `TransformAgency` evidence without changing order ranks.
- Modify `src/ai/deck_mutation_compiler_v1/compiler.rs`: expose exact removal snapshots without duplicating target-loss semantics.
- Modify `src/ai/deck_mutation_compiler_v1/mod.rs`: export the removal-snapshot function.
- Modify `src/ai/shop_policy_v1/types.rs`: add the functional-repair purge policy class and config gate.
- Modify `src/ai/shop_policy_v1/policy.rs`: map only profile-approved exact targets to that class.
- Modify `src/ai/shop_policy_v1/evaluator.rs`: admit the new class through the unified shop evaluator.
- Modify `src/ai/shop_policy_v1/tests.rs`: cover safe functional repair and protection precedence.
- Modify `src/ai/card_analysis_v1.rs`: centrally expose the Apparition upgrade's Ethereal-removal delta.
- Modify `src/ai/upgrade_planner_v1.rs`: carry the Ethereal-removal delta into upgrade candidates.
- Modify `src/ai/campfire_policy_v1/types.rs`: carry typed repair priority through candidate and plan records.
- Modify `src/ai/campfire_policy_v1/policy.rs`: attach repair candidates and order reliability repair before generic smith growth.
- Modify `src/ai/campfire_policy_v1/evaluator.rs`: allow the repair tag only after existing recovery/rest gates.
- Modify `src/ai/campfire_policy_v1/tests.rs`: cover Apparition reliability repair and rest safety.

---

### Task 1: Pandora Offer Evidence Without RNG Preview

**Files:**
- Create: `src/ai/pandora_offer_profile_v1.rs`
- Modify: `src/ai/mod.rs`
- Modify: `src/ai/strategy/boss_relic_admission.rs`

**Interfaces:**
- Consumes: `RunState`, `is_starter_strike`, `is_starter_defend`, `is_starter_basic`, and `card_reward_semantic_profile_v1`.
- Produces: `pandora_offer_profile_v1(&RunState) -> PandoraOfferProfileV1` and a `PandoraOfferFacts` admission reason.

- [ ] **Step 1: Write failing offer-profile tests**

Create `src/ai/pandora_offer_profile_v1.rs` with the public types, test module, and a deliberate `unimplemented!()` body:

```rust
use serde::Serialize;

use crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PandoraOfferHorizonV1 {
    AfterAct1,
    AfterAct2,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PandoraNonStarterSupportV1 {
    Frontload,
    Block,
    Access,
    Scaling,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PandoraOfferProfileV1 {
    pub starter_strikes: usize,
    pub starter_defends: usize,
    pub transform_targets: usize,
    pub deck_size: usize,
    pub transform_share_percent: u8,
    pub nonstarter_support: Vec<PandoraNonStarterSupportV1>,
    pub horizon: PandoraOfferHorizonV1,
    pub high_variance: bool,
}

pub fn pandora_offer_profile_v1(_run_state: &RunState) -> PandoraOfferProfileV1 {
    unimplemented!("derive Pandora offer facts without resolving the relic")
}

#[cfg(test)]
mod tests {
    use super::{pandora_offer_profile_v1, PandoraOfferHorizonV1};
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::run::RunState;

    fn card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    #[test]
    fn eight_starters_expose_more_transform_opportunity_than_two() {
        let mut many = RunState::new(1, 0, false, "Ironclad");
        many.act_num = 1;
        many.master_deck = (0..4)
            .map(|index| card(CardId::Strike, index + 1))
            .chain((0..4).map(|index| card(CardId::Defend, index + 10)))
            .chain([card(CardId::Bash, 30)])
            .collect();
        let mut few = many.clone();
        few.master_deck = vec![
            card(CardId::Strike, 1),
            card(CardId::Defend, 2),
            card(CardId::Bash, 3),
        ];

        let many_profile = pandora_offer_profile_v1(&many);
        let few_profile = pandora_offer_profile_v1(&few);

        assert_eq!(many_profile.transform_targets, 8);
        assert_eq!(many_profile.starter_strikes, 4);
        assert_eq!(many_profile.starter_defends, 4);
        assert!(many_profile.transform_share_percent > few_profile.transform_share_percent);
        assert_eq!(many_profile.horizon, PandoraOfferHorizonV1::AfterAct1);
        assert!(many_profile.high_variance);
    }
}
```

Also add `pub mod pandora_offer_profile_v1;` to `src/ai/mod.rs`.

- [ ] **Step 2: Run the focused test and verify the red state**

Run:

```powershell
cargo test --lib pandora_offer_profile_v1::tests::eight_starters_expose_more_transform_opportunity_than_two
```

Expected: FAIL because `pandora_offer_profile_v1` reaches `unimplemented!()`.

- [ ] **Step 3: Implement the public-information-only profile**

Replace the function body and add these helpers in `src/ai/pandora_offer_profile_v1.rs`:

```rust
pub fn pandora_offer_profile_v1(run_state: &RunState) -> PandoraOfferProfileV1 {
    use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
    use crate::content::cards::{is_starter_basic, is_starter_defend, is_starter_strike};
    use crate::state::rewards::RewardCard;

    let starter_strikes = run_state
        .master_deck
        .iter()
        .filter(|card| is_starter_strike(card.id))
        .count();
    let starter_defends = run_state
        .master_deck
        .iter()
        .filter(|card| is_starter_defend(card.id))
        .count();
    let transform_targets = run_state
        .master_deck
        .iter()
        .filter(|card| is_starter_basic(card.id))
        .count();
    let deck_size = run_state.master_deck.len();
    let transform_share_percent = if deck_size == 0 {
        0
    } else {
        ((transform_targets.saturating_mul(100) / deck_size).min(100)) as u8
    };
    let mut nonstarter_support = Vec::new();
    for card in run_state
        .master_deck
        .iter()
        .filter(|card| !is_starter_basic(card.id))
    {
        let roles = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades)).roles;
        push_support_for_roles(&mut nonstarter_support, &roles);
    }
    nonstarter_support.sort();
    nonstarter_support.dedup();

    PandoraOfferProfileV1 {
        starter_strikes,
        starter_defends,
        transform_targets,
        deck_size,
        transform_share_percent,
        nonstarter_support,
        horizon: match run_state.act_num {
            1 => PandoraOfferHorizonV1::AfterAct1,
            2 => PandoraOfferHorizonV1::AfterAct2,
            _ => PandoraOfferHorizonV1::Other,
        },
        high_variance: true,
    }
}

fn push_support_for_roles(
    support: &mut Vec<PandoraNonStarterSupportV1>,
    roles: &[CardRewardSemanticRoleV1],
) {
    let mappings = [
        (
            PandoraNonStarterSupportV1::Frontload,
            &[CardRewardSemanticRoleV1::FrontloadDamage][..],
        ),
        (
            PandoraNonStarterSupportV1::Block,
            &[
                CardRewardSemanticRoleV1::Block,
                CardRewardSemanticRoleV1::Weak,
                CardRewardSemanticRoleV1::EnemyStrengthDown,
            ][..],
        ),
        (
            PandoraNonStarterSupportV1::Access,
            &[
                CardRewardSemanticRoleV1::CardDraw,
                CardRewardSemanticRoleV1::CycleAccess,
                CardRewardSemanticRoleV1::DiscardPileTopdeckAccess,
                CardRewardSemanticRoleV1::HandTopdeckSelection,
            ][..],
        ),
        (
            PandoraNonStarterSupportV1::Scaling,
            &[
                CardRewardSemanticRoleV1::ScalingSource,
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::BlockPayoff,
            ][..],
        ),
    ];
    for (item, accepted_roles) in mappings {
        if roles.iter().any(|role| accepted_roles.contains(role)) && !support.contains(&item) {
            support.push(item);
        }
    }
}
```

Do not clone, mutate, or apply the relic. The function may only read the current deck.

- [ ] **Step 4: Attach offer facts to boss-relic evidence without ranking changes**

In `src/ai/strategy/boss_relic_admission.rs`, import the profile, add this reason variant, and emit it only for Pandora:

```rust
use crate::ai::pandora_offer_profile_v1::{
    pandora_offer_profile_v1, PandoraOfferHorizonV1,
};

// In BossRelicAdmissionReason:
PandoraOfferFacts {
    starter_strikes: usize,
    starter_defends: usize,
    transform_targets: usize,
    transform_share_percent: u8,
    horizon: PandoraOfferHorizonV1,
    high_variance: bool,
},

// In assess_boss_relic_admission, after class/lane derivation:
if relic == RelicId::PandorasBox {
    let profile = pandora_offer_profile_v1(run_state);
    reasons.push(BossRelicAdmissionReason::PandoraOfferFacts {
        starter_strikes: profile.starter_strikes,
        starter_defends: profile.starter_defends,
        transform_targets: profile.transform_targets,
        transform_share_percent: profile.transform_share_percent,
        horizon: profile.horizon,
        high_variance: profile.high_variance,
    });
}

// In reason_tag:
BossRelicAdmissionReason::PandoraOfferFacts {
    starter_strikes,
    starter_defends,
    transform_targets,
    transform_share_percent,
    horizon,
    high_variance,
} => format!(
    "pandora-offer:targets={transform_targets},strikes={starter_strikes},defends={starter_defends},share={transform_share_percent}%,horizon={horizon:?},high-variance={high_variance}"
),
```

Add this test inside the existing `boss_relic_admission.rs` test module (reuse its existing imports where present):

```rust
#[test]
fn pandora_offer_facts_do_not_change_admission_order_rank() {
    let mut many = RunState::new(1, 0, false, "Ironclad");
    many.act_num = 1;
    many.master_deck = (0..4)
        .map(|index| CombatCard::new(CardId::Strike, index + 1))
        .chain((0..4).map(|index| CombatCard::new(CardId::Defend, index + 10)))
        .collect();
    let mut few = many.clone();
    few.master_deck = vec![
        CombatCard::new(CardId::Strike, 1),
        CombatCard::new(CardId::Defend, 2),
    ];

    let many_admission = assess_boss_relic_admission(&many, RelicId::PandorasBox);
    let few_admission = assess_boss_relic_admission(&few, RelicId::PandorasBox);

    assert!(many_admission.reasons.iter().any(|reason| matches!(
        reason,
        BossRelicAdmissionReason::PandoraOfferFacts {
            transform_targets: 8,
            ..
        }
    )));
    assert_eq!(
        boss_relic_admission_order_rank(&many_admission),
        boss_relic_admission_order_rank(&few_admission),
    );
}
```

- [ ] **Step 5: Run focused tests and commit**

Run:

```powershell
cargo fmt --all
cargo test --lib pandora_offer_profile_v1
cargo test --lib strategy::boss_relic_admission::tests::pandora
```

Expected: both commands PASS; the boss-relic test confirms evidence changes without an order-rank change.

Commit:

```powershell
git add src/ai/mod.rs src/ai/pandora_offer_profile_v1.rs src/ai/strategy/boss_relic_admission.rs
git commit -m "feat: expose Pandora offer evidence"
```

---

### Task 2: General Deck Repair Profile and Exact Removal Evidence

**Files:**
- Create: `src/ai/deck_repair_profile_v1.rs`
- Modify: `src/ai/mod.rs`
- Modify: `src/ai/deck_mutation_compiler_v1/compiler.rs`
- Modify: `src/ai/deck_mutation_compiler_v1/mod.rs`

**Interfaces:**
- Consumes: `deck_removal_target_snapshots_v1(&RunState)`, `assess_deck_strategic_deficit`, semantic roles, and `plan_upgrades_v1`.
- Produces: `deck_repair_profile_v1(&RunState) -> DeckRepairProfileV1`, including exact low-loss removals and a stable slot for reliability upgrades.

- [ ] **Step 1: Expose removal snapshots with a failing compiler test**

Add this test to `src/ai/deck_mutation_compiler_v1/tests.rs` before implementing the function:

```rust
#[test]
fn removal_snapshots_preserve_redundant_functional_loss_tier() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = vec![
        CombatCard::new(CardId::Flex, 1),
        CombatCard::new(CardId::Flex, 2),
    ];

    let snapshots = super::deck_removal_target_snapshots_v1(&run);

    assert_eq!(snapshots.len(), 2);
    assert!(snapshots.iter().all(|snapshot| {
        snapshot.target_loss.tier == DeckMutationTargetLossTierV1::RedundantFunctional
    }));
}
```

Run:

```powershell
cargo test --lib deck_mutation_compiler_v1::tests::removal_snapshots_preserve_redundant_functional_loss_tier
```

Expected: FAIL because `deck_removal_target_snapshots_v1` does not exist.

- [ ] **Step 2: Implement and export the snapshot function**

Add to `src/ai/deck_mutation_compiler_v1/compiler.rs`:

```rust
pub fn deck_removal_target_snapshots_v1(
    run_state: &RunState,
) -> Vec<DeckMutationCardSnapshotV1> {
    run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| crate::state::core::master_deck_card_is_purgeable(card))
        .filter_map(|(deck_index, _)| {
            exact_target_for_deck_index(
                run_state,
                RunPendingChoiceReason::PurgeNonBottled,
                deck_index,
                true,
                None,
            )
            .map(|target| target.card)
        })
        .collect()
}
```

Export it from `src/ai/deck_mutation_compiler_v1/mod.rs` beside the other compiler functions.

- [ ] **Step 3: Write failing repair-profile tests and public types**

Create `src/ai/deck_repair_profile_v1.rs` with these public interfaces and tests. Keep the function body temporarily `unimplemented!()`:

```rust
use serde::Serialize;

use crate::ai::deck_mutation_compiler_v1::DeckMutationTargetLossV1;
use crate::content::cards::CardId;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckRepairFunctionV1 {
    Frontload,
    Aoe,
    Block,
    Scaling,
    Access,
    EnergyOrPlayability,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckRepairRemovalCandidateV1 {
    pub deck_index: usize,
    pub uuid: u32,
    pub card: CardId,
    pub target_loss: DeckMutationTargetLossV1,
    pub provided_functions: Vec<DeckRepairFunctionV1>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckRepairUpgradePriorityV1 {
    NeededFunction,
    Reliability,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckRepairUpgradeReasonV1 {
    RetainsTimeSensitiveDefense,
    LowersNeededFunctionCost,
    PaysImportantUpgradeDebt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeckRepairUpgradeCandidateV1 {
    pub deck_index: usize,
    pub uuid: u32,
    pub card: CardId,
    pub priority: DeckRepairUpgradePriorityV1,
    pub reasons: Vec<DeckRepairUpgradeReasonV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckRepairProfileV1 {
    pub thin_or_missing_functions: Vec<DeckRepairFunctionV1>,
    pub low_loss_removals: Vec<DeckRepairRemovalCandidateV1>,
    pub reliability_upgrades: Vec<DeckRepairUpgradeCandidateV1>,
    pub source_tags: Vec<String>,
}

pub fn deck_repair_profile_v1(_run_state: &RunState) -> DeckRepairProfileV1 {
    unimplemented!("derive repair facts from the concrete current deck")
}

#[cfg(test)]
mod tests {
    use super::deck_repair_profile_v1;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::run::RunState;

    #[test]
    fn duplicate_low_marginal_function_can_be_a_repair_removal() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.master_deck = vec![
            CombatCard::new(CardId::Flex, 1),
            CombatCard::new(CardId::Flex, 2),
            CombatCard::new(CardId::Bash, 3),
            CombatCard::new(CardId::ShrugItOff, 4),
        ];

        let profile = deck_repair_profile_v1(&run);

        assert!(profile.low_loss_removals.iter().any(|item| item.card == CardId::Flex));
    }

    #[test]
    fn singleton_core_function_and_pandora_tag_do_not_create_a_removal() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.master_deck = vec![CombatCard::new(CardId::Barricade, 1)];
        run.relics.push(RelicState::new(RelicId::PandorasBox));

        let profile = deck_repair_profile_v1(&run);

        assert!(profile.low_loss_removals.is_empty());
        assert_eq!(profile.source_tags, vec!["pandoras_box".to_string()]);
    }

    #[test]
    fn unsupported_card_semantics_do_not_create_repair_removal() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.master_deck = vec![
            CombatCard::new(CardId::Havoc, 1),
            CombatCard::new(CardId::Havoc, 2),
        ];

        let profile = deck_repair_profile_v1(&run);

        assert!(profile.low_loss_removals.is_empty());
    }
}
```

Add `pub mod deck_repair_profile_v1;` to `src/ai/mod.rs`.

- [ ] **Step 4: Run the repair tests and verify the red state**

Run:

```powershell
cargo test --lib deck_repair_profile_v1::tests
```

Expected: FAIL at `unimplemented!()`.

- [ ] **Step 5: Implement the score-free profile**

Implement `deck_repair_profile_v1` by:

1. Calling `assess_deck_strategic_deficit(&run_state.master_deck, RunStrategicFacts::from_run_state(run_state))`.
2. Converting only `Missing`/`Thin` fields to `DeckRepairFunctionV1`.
3. Calling `deck_removal_target_snapshots_v1(run_state)`.
4. Keeping only `DeckMutationTargetLossTierV1::RedundantFunctional` snapshots whose semantic functions do not intersect the thin/missing set.
5. Leaving `reliability_upgrades` empty until Task 4.
6. Adding `pandoras_box` to `source_tags` only when the relic is owned; the tag does not alter candidate admission.

Use this exact role mapping helper:

```rust
fn functions_for_card(card: CardId, upgrades: u8) -> Vec<DeckRepairFunctionV1> {
    use crate::ai::card_reward_policy_v1::{
        card_reward_semantic_profile_v1, CardRewardSemanticRoleV1 as Role,
    };
    use crate::state::rewards::RewardCard;

    let roles = card_reward_semantic_profile_v1(&RewardCard::new(card, upgrades)).roles;
    let mut functions = Vec::new();
    let mappings = [
        (DeckRepairFunctionV1::Frontload, vec![Role::FrontloadDamage]),
        (DeckRepairFunctionV1::Aoe, vec![Role::AoeDamage]),
        (
            DeckRepairFunctionV1::Block,
            vec![Role::Block, Role::BlockRetention, Role::Weak, Role::EnemyStrengthDown],
        ),
        (
            DeckRepairFunctionV1::Scaling,
            vec![Role::ScalingSource, Role::StrengthPayoff, Role::BlockPayoff],
        ),
        (
            DeckRepairFunctionV1::Access,
            vec![Role::CardDraw, Role::CycleAccess, Role::DiscardPileTopdeckAccess, Role::HandTopdeckSelection],
        ),
        (DeckRepairFunctionV1::EnergyOrPlayability, vec![Role::EnergySource]),
    ];
    for (function, accepted) in mappings {
        if roles.iter().any(|role| accepted.contains(role)) {
            functions.push(function);
        }
    }
    functions.sort();
    functions.dedup();
    functions
}

fn card_semantics_supported_for_repair(card: CardId, upgrades: u8) -> bool {
    use crate::ai::card_reward_policy_v1::{
        card_reward_semantic_profile_v1, CardRewardSemanticRoleV1,
    };
    use crate::state::rewards::RewardCard;

    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, upgrades));
    profile.unsupported_mechanics.is_empty()
        && !profile
            .roles
            .contains(&CardRewardSemanticRoleV1::UnsupportedMechanics)
}
```

Use these helpers and function body to construct the profile:

```rust
fn is_thin_or_missing(level: StrategicDeficitLevel) -> bool {
    matches!(level, StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin)
}

fn thin_or_missing_functions(
    deficit: &DeckStrategicDeficit,
) -> Vec<DeckRepairFunctionV1> {
    let fields = [
        (DeckRepairFunctionV1::Frontload, deficit.frontload_damage),
        (DeckRepairFunctionV1::Aoe, deficit.aoe_or_minion_control),
        (DeckRepairFunctionV1::Block, deficit.block_or_mitigation),
        (DeckRepairFunctionV1::Scaling, deficit.boss_scaling_plan),
        (DeckRepairFunctionV1::Access, deficit.deck_access),
        (
            DeckRepairFunctionV1::EnergyOrPlayability,
            deficit.energy_or_playability,
        ),
    ];
    fields
        .into_iter()
        .filter_map(|(function, level)| is_thin_or_missing(level).then_some(function))
        .collect()
}

pub fn deck_repair_profile_v1(run_state: &RunState) -> DeckRepairProfileV1 {
    let deficit = assess_deck_strategic_deficit(
        &run_state.master_deck,
        RunStrategicFacts::from_run_state(run_state),
    );
    let thin_or_missing_functions = thin_or_missing_functions(&deficit);
    let low_loss_removals = deck_removal_target_snapshots_v1(run_state)
        .into_iter()
        .filter(|snapshot| {
            snapshot.target_loss.tier
                == DeckMutationTargetLossTierV1::RedundantFunctional
        })
        .filter_map(|snapshot| {
            if !card_semantics_supported_for_repair(snapshot.card, snapshot.upgrades) {
                return None;
            }
            let provided_functions = functions_for_card(snapshot.card, snapshot.upgrades);
            if provided_functions
                .iter()
                .any(|function| thin_or_missing_functions.contains(function))
            {
                return None;
            }
            Some(DeckRepairRemovalCandidateV1 {
                deck_index: snapshot.deck_index,
                uuid: snapshot.uuid,
                card: snapshot.card,
                target_loss: snapshot.target_loss,
                provided_functions,
            })
        })
        .collect();
    let source_tags = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::PandorasBox)
        .then(|| vec!["pandoras_box".to_string()])
        .unwrap_or_default();

    DeckRepairProfileV1 {
        thin_or_missing_functions,
        low_loss_removals,
        reliability_upgrades: Vec::new(),
        source_tags,
    }
}
```

Preserve `target_loss` unchanged; do not convert its signals into a second score.

- [ ] **Step 6: Run focused tests and commit**

Run:

```powershell
cargo fmt --all
cargo test --lib deck_mutation_compiler_v1::tests::removal_snapshots_preserve_redundant_functional_loss_tier
cargo test --lib deck_repair_profile_v1
```

Expected: both commands PASS.

Commit:

```powershell
git add src/ai/mod.rs src/ai/deck_repair_profile_v1.rs src/ai/deck_mutation_compiler_v1/compiler.rs src/ai/deck_mutation_compiler_v1/mod.rs src/ai/deck_mutation_compiler_v1/tests.rs
git commit -m "feat: derive deck repair evidence"
```

---

### Task 3: Shop Consumer for Safe Functional Repair

**Files:**
- Modify: `src/ai/shop_policy_v1/types.rs`
- Modify: `src/ai/shop_policy_v1/policy.rs`
- Modify: `src/ai/shop_policy_v1/evaluator.rs`
- Modify: `src/ai/shop_policy_v1/tests.rs`

**Interfaces:**
- Consumes: `DeckRepairProfileV1.low_loss_removals` keyed by exact deck index and card UUID.
- Produces: `ShopPolicyClassV1::FunctionalRepairPurge` and an evaluator-admitted single-step purge plan.

- [ ] **Step 1: Write failing shop policy tests**

Add tests to `src/ai/shop_policy_v1/tests.rs`:

```rust
#[test]
fn functional_repair_purge_is_visible_only_for_profile_approved_target() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.gold = 200;
    run.master_deck = vec![
        crate::runtime::combat::CombatCard::new(CardId::Flex, 1),
        crate::runtime::combat::CombatCard::new(CardId::Flex, 2),
        crate::runtime::combat::CombatCard::new(CardId::Bash, 3),
        crate::runtime::combat::CombatCard::new(CardId::ShrugItOff, 4),
    ];
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run, &shop);

    assert!(context.candidates.iter().any(|candidate| {
        candidate.class == ShopPolicyClassV1::FunctionalRepairPurge
            && candidate.card == Some(CardId::Flex)
    }));
}

#[test]
fn starter_cleanup_stays_ahead_of_functional_repair() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.gold = 200;
    run.master_deck.push(crate::runtime::combat::CombatCard::new(CardId::Flex, 100));
    run.master_deck.push(crate::runtime::combat::CombatCard::new(CardId::Flex, 101));
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run, &shop);

    assert!(context.candidates.iter().any(|candidate| {
        matches!(candidate.class, ShopPolicyClassV1::StarterStrikePurge | ShopPolicyClassV1::StarterDefendPurge)
    }));
    assert!(!context.candidates.iter().any(|candidate| {
        candidate.class == ShopPolicyClassV1::FunctionalRepairPurge
    }));
}
```

Run:

```powershell
cargo test --lib shop_policy_v1::tests::functional_repair_purge_is_visible_only_for_profile_approved_target
```

Expected: FAIL because `FunctionalRepairPurge` does not exist.

- [ ] **Step 2: Add the typed class and config gate**

In `src/ai/shop_policy_v1/types.rs`:

```rust
pub enum ShopPolicyClassV1 {
    CursePurge,
    StarterStrikePurge,
    StarterDefendPurge,
    FunctionalRepairPurge,
    PurchaseOpportunity,
    Leave,
    Unknown,
}

pub struct ShopPolicyConfigV1 {
    pub allow_curse_purge: bool,
    pub allow_starter_strike_purge_when_core_plan_protected: bool,
    pub allow_functional_repair_purge: bool,
    pub allow_high_impact_purchase: bool,
    pub high_impact_card_legacy_estimate_threshold: i32,
    pub high_impact_relic_legacy_estimate_threshold: i32,
    pub high_impact_potion_legacy_estimate_threshold: i32,
}

impl Default for ShopPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_starter_strike_purge_when_core_plan_protected: true,
            allow_functional_repair_purge: true,
            allow_high_impact_purchase: true,
            high_impact_card_legacy_estimate_threshold: 650,
            high_impact_relic_legacy_estimate_threshold: 900,
            high_impact_potion_legacy_estimate_threshold: 780,
        }
    }
}
```

In the purge evidence match, treat `FunctionalRepairPurge` as deck cleaning and add the exact repair tag; in `purge_support_gate`, return `Strong` only for an exact repair-profile match. The evaluator match is completed in Step 4. No purchase or leave match changes behavior.

- [ ] **Step 3: Map exact profile candidates in the shop context**

In `build_shop_decision_context_v1`, derive the profile once and pass it into the purge adapter:

```rust
let repair_profile = crate::ai::deck_repair_profile_v1::deck_repair_profile_v1(run_state);

// Existing purge call:
candidates.extend(shop_purge_candidates_from_deck_mutation_compiler_v1(
    run_state,
    shop,
    &strategy,
    &repair_profile,
));
```

Change `purge_candidate_evidence` to receive the profile and classify an exact functional repair only when both deck index and UUID match:

```rust
fn repair_candidate_matches(
    profile: &crate::ai::deck_repair_profile_v1::DeckRepairProfileV1,
    snapshot: &crate::ai::deck_mutation_compiler_v1::DeckMutationCardSnapshotV1,
) -> bool {
    profile.low_loss_removals.iter().any(|candidate| {
        candidate.deck_index == snapshot.deck_index
            && candidate.uuid == snapshot.uuid
            && candidate.card == snapshot.card
    })
}

let class = match card_snapshot.target_class {
    DeckMutationTargetClassV1::Curse => ShopPolicyClassV1::CursePurge,
    DeckMutationTargetClassV1::StarterStrike => ShopPolicyClassV1::StarterStrikePurge,
    DeckMutationTargetClassV1::StarterDefend => ShopPolicyClassV1::StarterDefendPurge,
    DeckMutationTargetClassV1::Functional
        if !low_value_cleanup_available
            && repair_candidate_matches(repair_profile, card_snapshot) =>
    {
        ShopPolicyClassV1::FunctionalRepairPurge
    }
    _ => ShopPolicyClassV1::Unknown,
};
```

In `shop_purge_candidates_from_deck_mutation_compiler_v1`, compute cleanup precedence once and pass it into every candidate adapter call:

```rust
let low_value_cleanup_available = decision.candidate_plans.iter().any(|plan| {
    plan.step.cards.iter().any(|card| {
        matches!(
            card.target_class,
            DeckMutationTargetClassV1::Curse
                | DeckMutationTargetClassV1::StarterStrike
                | DeckMutationTargetClassV1::StarterDefend
        )
    })
});

decision
    .candidate_plans
    .iter()
    .filter_map(|plan| {
        purge_candidate_evidence(
            plan,
            shop.purge_cost,
            strategy,
            repair_profile,
            low_value_cleanup_available,
        )
    })
    .collect()
```

Guard the functional match with `!low_value_cleanup_available`. If a curse, Strike, or Defend cleanup target exists, leave every functional target as `Unknown`. Add evidence `deck_repair_profile=low_loss_redundant_functional` to an admitted candidate.

For `FunctionalRepairPurge`, set `support_gate` to `Strong` only when the exact profile match exists. Do not reuse the starter strategic trace because that trace represents a different action class.

- [ ] **Step 4: Admit the class through the unified evaluator**

In `src/ai/shop_policy_v1/evaluator.rs`, add the match arm and evaluator:

```rust
ShopPolicyClassV1::FunctionalRepairPurge => {
    evaluate_functional_repair_purge_v1(candidate, config)
}

fn evaluate_functional_repair_purge_v1(
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
) -> ShopPlanEvaluationV1 {
    if !config.allow_functional_repair_purge {
        return ShopPlanEvaluationV1::block(
            None,
            "functional repair purge disabled by shop policy config",
        );
    }
    if candidate.support_gate != StrategyPlanSupportV1::Strong
        || candidate.deck_index.is_none()
        || candidate.card.is_none()
        || !candidate
            .evidence
            .iter()
            .any(|item| item == "deck_repair_profile=low_loss_redundant_functional")
    {
        return ShopPlanEvaluationV1::block(
            None,
            "functional purge lacks exact low-loss deck-repair evidence",
        );
    }
    ShopPlanEvaluationV1::allow(
        305,
        450,
        0.74,
        None,
        "shop evaluator: evidence-backed functional deck repair",
    )
}
```

The numbers are existing shop-plan ordering metadata, not a deck-repair score. They place safe functional repair below curse cleanup and above unsupported candidates.

- [ ] **Step 5: Verify compiled execution and protection gates**

Extend the first shop test with:

```rust
let compiled = compile_shop_decision_v1(
    &context,
    &ShopPolicyConfigV1::default(),
    ShopCompileModeV1::ExecuteOne,
);
assert!(compiled.compat_selected_plan.steps.iter().any(|step| matches!(
    step,
    ShopPlanStepV1::RemoveCard {
        card: CardId::Flex,
        ..
    }
)));
```

Add the singleton protection test:

```rust
#[test]
fn singleton_core_function_is_not_functional_repair() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.gold = 200;
    run.master_deck = vec![
        crate::runtime::combat::CombatCard::new(CardId::Barricade, 1),
    ];
    let context = build_shop_decision_context_v1(&run, &ShopState::new());

    assert!(!context.candidates.iter().any(|candidate| {
        candidate.class == ShopPolicyClassV1::FunctionalRepairPurge
    }));
}
```

Run:

```powershell
cargo fmt --all
cargo test --lib shop_policy_v1::tests::functional_repair
cargo test --lib shop_policy_v1::tests::starter_cleanup_stays_ahead_of_functional_repair
cargo test --lib shop_policy_v1::tests::singleton_core_function_is_not_functional_repair
```

Expected: all commands PASS.

- [ ] **Step 6: Commit**

```powershell
git add src/ai/shop_policy_v1/types.rs src/ai/shop_policy_v1/policy.rs src/ai/shop_policy_v1/evaluator.rs src/ai/shop_policy_v1/tests.rs
git commit -m "feat: admit safe functional shop repair"
```

---

### Task 4: Reliability Upgrade Evidence and Campfire Consumer

**Files:**
- Modify: `src/ai/card_analysis_v1.rs`
- Modify: `src/ai/upgrade_planner_v1.rs`
- Modify: `src/ai/deck_repair_profile_v1.rs`
- Modify: `src/ai/campfire_policy_v1/types.rs`
- Modify: `src/ai/campfire_policy_v1/policy.rs`
- Modify: `src/ai/campfire_policy_v1/evaluator.rs`
- Modify: `src/ai/campfire_policy_v1/tests.rs`

**Interfaces:**
- Consumes: `CardAnalysisProfileV1.is_upgrade_ethereal_removed_delta`, `UpgradeCandidateV1.mechanical_delta`, and `DeckRepairProfileV1.reliability_upgrades`.
- Produces: exact `DeckRepairUpgradeCandidateV1` entries and categorical campfire `repair_priority` ordering that remains behind rest safety.

- [ ] **Step 1: Write failing mechanics and profile tests**

Add to the existing `card_analysis_v1` tests:

```rust
#[test]
fn apparition_upgrade_records_ethereal_removal() {
    let profile = card_analysis_profile_v1(CardId::Apparition, 0);
    assert!(profile.is_upgrade_ethereal_removed_delta);
}
```

Add to `deck_repair_profile_v1` tests:

```rust
#[test]
fn unupgraded_apparitions_expose_reliability_repair_without_fixed_quota() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = (0..5)
        .map(|index| CombatCard::new(CardId::Apparition, index + 1))
        .chain([CombatCard::new(CardId::SpotWeakness, 20)])
        .collect();

    let profile = deck_repair_profile_v1(&run);

    assert_eq!(
        profile
            .reliability_upgrades
            .iter()
            .filter(|candidate| candidate.card == CardId::Apparition)
            .count(),
        5
    );
    assert!(profile
        .reliability_upgrades
        .iter()
        .filter(|candidate| candidate.card == CardId::Apparition)
        .all(|candidate| candidate.reasons.contains(
            &DeckRepairUpgradeReasonV1::RetainsTimeSensitiveDefense,
        )));
}
```

Run:

```powershell
cargo fmt --all
cargo test --lib card_analysis_v1::tests::apparition_upgrade_records_ethereal_removal
cargo test --lib deck_repair_profile_v1::tests::unupgraded_apparitions_expose_reliability_repair_without_fixed_quota
```

Expected: FAIL because the mechanics field and populated reliability upgrades do not exist yet.

- [ ] **Step 2: Add the central Ethereal-removal mechanical delta**

In `CardAnalysisProfileV1`, add:

```rust
pub is_upgrade_ethereal_removed_delta: bool,
```

Populate it in `card_analysis_profile_v1` with:

```rust
is_upgrade_ethereal_removed_delta: matches!(card, CardId::Apparition),
```

In `UpgradeMechanicalDeltaV1`, add:

```rust
pub ethereal_removed_delta: bool,
```

Populate it in `mechanical_upgrade_delta` and add a diagnostic note:

```rust
ethereal_removed_delta: analysis.is_upgrade_ethereal_removed_delta,

if delta.ethereal_removed_delta {
    delta
        .notes
        .push("upgrade removes Ethereal and preserves the card across an unused draw".to_string());
}
```

Do not add Apparition to `is_upgrade_exhaust_removed_delta_v1`; Ethereal and Exhaust remain distinct mechanics.

- [ ] **Step 3: Populate exact reliability upgrades in the repair profile**

In `deck_repair_profile_v1`, call `plan_upgrades_v1(run_state)` once. Add a categorical repair candidate when an upgrade retains time-sensitive defense, lowers the cost of a currently thin/missing function, or pays an already-important upgrade debt:

```rust
let upgrade_plan = crate::ai::upgrade_planner_v1::plan_upgrades_v1(run_state);
let reliability_upgrades = upgrade_plan
    .candidates
    .iter()
    .filter_map(|candidate| {
        let card = run_state.master_deck.get(candidate.deck_index)?;
        let mut reasons = Vec::new();
        let mut priority = None;
        if candidate.mechanical_delta.ethereal_removed_delta
            && candidate
                .roles
                .contains(&crate::ai::upgrade_planner_v1::UpgradeRoleV1::DefensiveSurvival)
        {
            reasons.push(DeckRepairUpgradeReasonV1::RetainsTimeSensitiveDefense);
            priority = Some(DeckRepairUpgradePriorityV1::Reliability);
        }
        let supplied_functions = functions_for_card(card.id, card.upgrades);
        if candidate.mechanical_delta.cost_delta > 0
            && supplied_functions
                .iter()
                .any(|function| thin_or_missing_functions.contains(function))
        {
            reasons.push(DeckRepairUpgradeReasonV1::LowersNeededFunctionCost);
            priority = Some(priority.unwrap_or(DeckRepairUpgradePriorityV1::NeededFunction));
        }
        if candidate.urgency
            >= crate::ai::upgrade_planner_v1::UpgradeDebtSeverityV1::ImportantBeforeBoss
        {
            reasons.push(DeckRepairUpgradeReasonV1::PaysImportantUpgradeDebt);
            priority = Some(priority.unwrap_or(DeckRepairUpgradePriorityV1::NeededFunction));
        }
        let priority = priority?;
        Some(DeckRepairUpgradeCandidateV1 {
            deck_index: candidate.deck_index,
            uuid: card.uuid,
            card: card.id,
            priority,
            reasons,
        })
    })
    .collect();

// Return the populated upgrade-repair evidence:
DeckRepairProfileV1 {
    thin_or_missing_functions,
    low_loss_removals,
    reliability_upgrades,
    source_tags,
}
```

- [ ] **Step 4: Carry repair priority through campfire candidates and plans**

In `CampfireCandidateEvidenceV1` and `CampfirePlanCandidateV1`, add:

```rust
pub repair_priority: Option<
    crate::ai::deck_repair_profile_v1::DeckRepairUpgradePriorityV1,
>,
```

In `build_campfire_decision_context_v1`, derive the repair profile once. Pass it to `candidate_evidence`; for a Smith candidate, look up an exact match by deck index and UUID. Set:

```rust
let repair_priority = repair_profile
    .reliability_upgrades
    .iter()
    .find(|repair| {
        repair.deck_index == idx
            && run_state
                .master_deck
                .get(idx)
                .is_some_and(|card| card.uuid == repair.uuid)
    })
    .map(|repair| repair.priority);
```

When a match exists, append each typed reason to evidence. Use strategy tag `deck_repair:reliability` for `Reliability` and `deck_repair:needed_function` for `NeededFunction`, instead of the generic first upgrade-role tag. Copy `repair_priority` into the plan in `campfire_candidate_plan` and set `None` in every stop/fallback constructor and explicit test fixture.

Change `compare_campfire_plan_candidates_v1` so repair priority is considered after executable plan role but before existing numeric score:

```rust
campfire_plan_role_rank(left.role)
    .cmp(&campfire_plan_role_rank(right.role))
    .then_with(|| right.repair_priority.cmp(&left.repair_priority))
    .then_with(|| right.score_hint.cmp(&left.score_hint))
    .then_with(|| right.confidence.total_cmp(&left.confidence))
    .then_with(|| left.plan_id.cmp(&right.plan_id))
```

- [ ] **Step 5: Preserve rest safety and allow the repair tag**

In `campfire_policy_v1/evaluator.rs`, keep every existing early return in `smith_is_autopilot_allowed` unchanged. Add `"deck_repair:reliability"` and `"deck_repair:needed_function"` to `clear_core_upgrade_tag` and `combat_patch_upgrade_tag`; those helpers are reached only after the existing recovery-pressure and `RestFavored` checks.

Add tests to `campfire_policy_v1/tests.rs`:

```rust
#[test]
fn reliability_repair_smith_precedes_generic_growth_when_safe() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.current_hp = run.max_hp;
    run.master_deck = vec![
        CombatCard::new(CardId::Cleave, 1),
        CombatCard::new(CardId::Apparition, 2),
        CombatCard::new(CardId::Apparition, 3),
    ];
    let context = build_campfire_decision_context_v1(
        &run,
        vec![CampfireChoice::Smith(0)],
    );

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Smith { deck_index: 1 | 2, .. }
    ));
}

#[test]
fn rest_favored_still_blocks_reliability_repair_smith() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.current_hp = 1;
    run.master_deck = vec![CombatCard::new(CardId::Apparition, 1)];
    let context = build_campfire_decision_context_v1(
        &run,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(decision.action, CampfirePolicyActionV1::Rest { .. }));
}
```

- [ ] **Step 6: Run focused tests and commit**

Run:

```powershell
cargo test --lib card_analysis_v1::tests::apparition_upgrade_records_ethereal_removal
cargo test --lib deck_repair_profile_v1::tests::unupgraded_apparitions_expose_reliability_repair_without_fixed_quota
cargo test --lib campfire_policy_v1::tests::reliability_repair
cargo test --lib campfire_policy_v1::tests::rest_favored_still_blocks_reliability_repair_smith
```

Expected: all commands PASS.

Commit:

```powershell
git add src/ai/card_analysis_v1.rs src/ai/upgrade_planner_v1.rs src/ai/deck_repair_profile_v1.rs src/ai/campfire_policy_v1/types.rs src/ai/campfire_policy_v1/policy.rs src/ai/campfire_policy_v1/evaluator.rs src/ai/campfire_policy_v1/tests.rs
git commit -m "feat: prioritize reliability repair upgrades"
```

---

### Task 5: Integrated Verification Without a Full Seed

**Files:**
- Verify only; no new production files.

**Interfaces:**
- Consumes: all four task commits.
- Produces: fresh evidence that focused behavior, the full library, and runtime architecture boundaries pass together.

- [ ] **Step 1: Format and inspect the exact diff**

Run:

```powershell
cargo fmt --all -- --check
git diff --check HEAD~4..HEAD
git status --short
```

Expected: formatting and diff checks exit 0; worktree is clean.

- [ ] **Step 2: Run focused semantic suites**

Run:

```powershell
cargo test --lib pandora_offer_profile_v1
cargo test --lib deck_repair_profile_v1
cargo test --lib shop_policy_v1::tests::functional_repair
cargo test --lib campfire_policy_v1::tests::reliability_repair
```

Expected: every command reports 0 failed tests.

- [ ] **Step 3: Run the full library suite once**

Run:

```powershell
cargo test --lib
```

Expected: the complete library test binary reports 0 failed tests. Run it once, not once per module.

- [ ] **Step 4: Run the architecture boundary suite**

Run:

```powershell
cargo test --test architecture_runtime_boundaries
```

Expected: all architecture boundary tests pass.

- [ ] **Step 5: Report the evidence boundary**

Record in the implementation handoff:

- Pandora evidence does not change relic ordering.
- Functional repair cannot remove singleton/core or thin-function cards.
- Reliability smith remains behind rest safety.
- No reward or combat-search file changed.
- No complete seed was run, and no claim is made about Collector or overall win rate.

Do not create a verification-only commit when the worktree is already clean.
