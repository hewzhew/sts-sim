# Action-Supply and Choker-Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Do not dispatch subagents; the user explicitly requested inline execution until effort controls are available.

**Goal:** Replace generic generated-card/Choker coupling with categorical action-supply facts and exact combat evidence that the six-card cap is binding.

**Architecture:** Extend the central card/relic mechanics registry with score-free action-supply traits, aggregate those traits in a focused `ActionSupplyProfileV1`, and let boss-relic admission consume only categorical facts. Keep combat legality authoritative and add a separate Choker-capacity report that measures stranded affordable cards without changing state ordering or acquisition rank.

**Tech Stack:** Rust 2021, Serde, existing `RunState`/`CombatState` models, Cargo unit and architecture tests.

## Global Constraints

- Execute inline; do not use subagents.
- Do not add an aggregate action-pressure score.
- Do not add or change a Choker acquisition/rank adjustment in this pass.
- Keep `CardPlayCapDebt` as the structural statement that Choker has a six-card constraint.
- Keep the existing serialized generated-opening counters and combination flags readable; mark the combination flags deprecated and stop consuming them in new decisions.
- Unknown mechanics are neutral and must not create exposure or penalties.
- The simulator remains authoritative for generated cards, additional plays, legal actions, and Choker counters.
- Do not run a complete seed as a regression test.
- Do not merge Time Eater's twelve-card cycle into this model.

---

## File Structure

- Create `src/ai/action_supply_v1.rs`: aggregate central card/relic action-supply traits into stable, serialized, score-free run facts.
- Modify `src/ai/card_semantics_v1.rs`: own the single ID-to-action-supply mechanics registry used by every consumer.
- Modify `src/ai/mod.rs`: expose the new focused action-supply module.
- Modify `src/ai/deck_startup_profile_v1.rs`: retain the old pair flags but document them as
  compatibility-only facts that new consumers must not use.
- Modify `src/ai/strategy/boss_relic_admission.rs`: remove Pyramid-specific opening-budget attribution and attach neutral supply facts only to a Choker candidate.
- Modify `src/ai/combat_search_v2/card_pile_value.rs`: compute exact current-turn Choker capacity and stranded affordable-card counts.
- Modify `src/ai/combat_search_v2/value/facts.rs`: carry Choker-capacity facts alongside hand/draw facts without adding them to ordering.
- Modify `src/ai/combat_search_v2/value/report.rs`: serialize the Choker-capacity facts in frontier reports.
- Modify `src/ai/combat_search_v2/types/report/frontier.rs`: define the public report shape and add it to `CombatSearchV2FrontierValueReport`.

---

### Task 1: Central action-supply semantics and run profile

**Files:**

- Create: `src/ai/action_supply_v1.rs`
- Modify: `src/ai/card_semantics_v1.rs`
- Modify: `src/ai/mod.rs`
- Test: unit tests in `src/ai/card_semantics_v1.rs` and `src/ai/action_supply_v1.rs`

**Interfaces:**

- Consumes: `card_mechanics_profile_v1(CardId) -> CardMechanicsProfileV1`, `relic_mechanics_profile_v1(RelicId) -> RelicMechanicsProfileV1`, and `RunState`.
- Produces: `ActionSupplyTraitsV1`, `ActionSupplySourceV1`, `ActionSupplySourceFactV1`, `ActionSupplyProfileV1`, and `action_supply_profile_v1(&RunState) -> ActionSupplyProfileV1`.
- Later tasks rely on these exact aggregate fields: `opening_once_options`, `delayed_per_turn_sources`, `same_turn_burst_sources`, `triggered_repeatable_sources`, `additional_play_sources`, `cost_or_resource_compression_sources`, and `potentially_recursive_sources`.

- [ ] **Step 1: Write failing mechanics and aggregation tests**

Append these tests to `src/ai/card_semantics_v1.rs`:

```rust
#[test]
fn action_supply_traits_distinguish_once_burst_repeatable_and_additional() {
    let enchiridion = relic_mechanics_profile_v1(RelicId::Enchiridion).action_supply;
    assert_eq!(enchiridion.opening_once_options, 1);
    assert!(enchiridion.immediate_hand);
    assert!(enchiridion.zero_cost_this_turn);
    assert!(!enchiridion.triggered_repeatable);
    assert!(!enchiridion.potentially_recursive);

    let toolbox = relic_mechanics_profile_v1(RelicId::Toolbox).action_supply;
    assert_eq!(toolbox.opening_once_options, 1);
    assert!(toolbox.immediate_hand);
    assert!(!toolbox.zero_cost_this_turn);

    let codex = relic_mechanics_profile_v1(RelicId::NilrysCodex).action_supply;
    assert!(codex.delayed_per_turn);
    assert!(codex.optional_supply);
    assert!(!codex.same_turn_burst());

    let branch = relic_mechanics_profile_v1(RelicId::DeadBranch).action_supply;
    assert!(branch.triggered_repeatable);
    assert!(branch.immediate_hand);
    assert!(branch.potentially_recursive);

    let blade_dance = card_mechanics_profile_v1(CardId::BladeDance).action_supply;
    assert_eq!(blade_dance.same_turn_burst_min_follow_ups, 3);
    assert!(blade_dance.same_turn_burst());

    let double_tap = card_mechanics_profile_v1(CardId::DoubleTap).action_supply;
    assert!(double_tap.additional_play);
    assert_eq!(double_tap.same_turn_burst_min_follow_ups, 0);

    assert!(card_mechanics_profile_v1(CardId::Corruption)
        .action_supply
        .cost_or_resource_compression);
    assert!(card_mechanics_profile_v1(CardId::Offering)
        .action_supply
        .cost_or_resource_compression);
}
```

Create `src/ai/action_supply_v1.rs` with the module imports and this initial test module; the referenced production types intentionally do not exist yet:

```rust
use crate::ai::card_semantics_v1::{card_mechanics_profile_v1, relic_mechanics_profile_v1};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;

    #[test]
    fn run_profile_keeps_opening_supply_neutral_and_repeatable_supply_separate() {
        let mut run = RunState::new(20260711002, 0, false, "Ironclad");
        run.relics = vec![
            RelicState::new(RelicId::Enchiridion),
            RelicState::new(RelicId::DeadBranch),
        ];
        run.add_card_to_deck(CardId::BladeDance);
        run.add_card_to_deck(CardId::DoubleTap);
        run.add_card_to_deck(CardId::Corruption);

        let profile = action_supply_profile_v1(&run);

        assert_eq!(profile.opening_once_options, 1);
        assert_eq!(profile.delayed_per_turn_sources, 0);
        assert_eq!(profile.same_turn_burst_sources, 1);
        assert_eq!(profile.triggered_repeatable_sources, 1);
        assert_eq!(profile.additional_play_sources, 1);
        assert_eq!(profile.cost_or_resource_compression_sources, 1);
        assert_eq!(profile.potentially_recursive_sources, 1);
        assert_eq!(profile.sources.len(), 5);
    }

    #[test]
    fn unknown_or_ordinary_mechanics_do_not_create_action_supply() {
        let mut run = RunState::new(20260711003, 0, false, "Ironclad");
        run.add_card_to_deck(CardId::Strike);
        run.relics.push(RelicState::new(RelicId::Vajra));

        assert!(action_supply_profile_v1(&run).is_empty());
    }
}
```

Add `pub mod action_supply_v1;` to `src/ai/mod.rs` so the new test module compiles through the crate.

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test --lib action_supply_traits_distinguish_once_burst_repeatable_and_additional
cargo test --lib run_profile_keeps_opening_supply_neutral_and_repeatable_supply_separate
```

Expected: compilation fails because `ActionSupplyTraitsV1`, the `action_supply` mechanics fields, and `action_supply_profile_v1` are not defined.

- [ ] **Step 3: Add the action-supply mechanics types and central registry mappings**

In `src/ai/card_semantics_v1.rs`, import Serde and define the structural traits before `CardMechanicsProfileV1`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ActionSupplyTraitsV1 {
    pub opening_once_options: u8,
    pub delayed_per_turn: bool,
    pub same_turn_burst_min_follow_ups: u8,
    pub triggered_repeatable: bool,
    pub additional_play: bool,
    pub cost_or_resource_compression: bool,
    pub potentially_recursive: bool,
    pub immediate_hand: bool,
    pub zero_cost_this_turn: bool,
    pub optional_supply: bool,
}

impl ActionSupplyTraitsV1 {
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    pub fn same_turn_burst(self) -> bool {
        self.same_turn_burst_min_follow_ups > 0
    }
}
```

Add this field to both mechanics structs:

```rust
pub action_supply: ActionSupplyTraitsV1,
```

Initialize it in the existing profile constructors:

```rust
// In card_mechanics_profile_v1's CardMechanicsProfileV1 literal:
action_supply: card_action_supply_traits_v1(card),

// In relic_mechanics_profile_v1's RelicMechanicsProfileV1 literal:
action_supply: relic_action_supply_traits_v1(relic),
```

Add these central registry functions beside the existing mechanics-profile functions:

```rust
fn card_action_supply_traits_v1(card: CardId) -> ActionSupplyTraitsV1 {
    match card {
        CardId::BladeDance => ActionSupplyTraitsV1 {
            same_turn_burst_min_follow_ups: 3,
            immediate_hand: true,
            ..Default::default()
        },
        CardId::DoubleTap => ActionSupplyTraitsV1 {
            additional_play: true,
            ..Default::default()
        },
        CardId::Corruption | CardId::Offering => ActionSupplyTraitsV1 {
            cost_or_resource_compression: true,
            ..Default::default()
        },
        _ => ActionSupplyTraitsV1::default(),
    }
}

fn relic_action_supply_traits_v1(relic: RelicId) -> ActionSupplyTraitsV1 {
    match relic {
        RelicId::Enchiridion => ActionSupplyTraitsV1 {
            opening_once_options: 1,
            immediate_hand: true,
            zero_cost_this_turn: true,
            ..Default::default()
        },
        RelicId::Toolbox => ActionSupplyTraitsV1 {
            opening_once_options: 1,
            immediate_hand: true,
            ..Default::default()
        },
        RelicId::NilrysCodex => ActionSupplyTraitsV1 {
            delayed_per_turn: true,
            optional_supply: true,
            ..Default::default()
        },
        RelicId::DeadBranch => ActionSupplyTraitsV1 {
            triggered_repeatable: true,
            immediate_hand: true,
            potentially_recursive: true,
            ..Default::default()
        },
        _ => ActionSupplyTraitsV1::default(),
    }
}
```

This is the only ID mapping in the pass. Boss admission and combat search must not branch on these IDs.

- [ ] **Step 4: Implement the focused run profile**

Replace the temporary contents above the tests in `src/ai/action_supply_v1.rs` with:

```rust
use crate::ai::card_semantics_v1::{
    card_mechanics_profile_v1, relic_mechanics_profile_v1, ActionSupplyTraitsV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSupplySourceV1 {
    Card(CardId),
    Relic(RelicId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionSupplySourceFactV1 {
    pub source: ActionSupplySourceV1,
    pub traits: ActionSupplyTraitsV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ActionSupplyProfileV1 {
    pub opening_once_options: u8,
    pub delayed_per_turn_sources: u8,
    pub same_turn_burst_sources: u8,
    pub triggered_repeatable_sources: u8,
    pub additional_play_sources: u8,
    pub cost_or_resource_compression_sources: u8,
    pub potentially_recursive_sources: u8,
    pub sources: Vec<ActionSupplySourceFactV1>,
}

impl ActionSupplyProfileV1 {
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    fn record(&mut self, source: ActionSupplySourceV1, traits: ActionSupplyTraitsV1) {
        if traits.is_empty() {
            return;
        }
        self.opening_once_options = self
            .opening_once_options
            .saturating_add(traits.opening_once_options);
        self.delayed_per_turn_sources = self
            .delayed_per_turn_sources
            .saturating_add(u8::from(traits.delayed_per_turn));
        self.same_turn_burst_sources = self
            .same_turn_burst_sources
            .saturating_add(u8::from(traits.same_turn_burst()));
        self.triggered_repeatable_sources = self
            .triggered_repeatable_sources
            .saturating_add(u8::from(traits.triggered_repeatable));
        self.additional_play_sources = self
            .additional_play_sources
            .saturating_add(u8::from(traits.additional_play));
        self.cost_or_resource_compression_sources = self
            .cost_or_resource_compression_sources
            .saturating_add(u8::from(traits.cost_or_resource_compression));
        self.potentially_recursive_sources = self
            .potentially_recursive_sources
            .saturating_add(u8::from(traits.potentially_recursive));
        self.sources.push(ActionSupplySourceFactV1 { source, traits });
    }
}

pub fn action_supply_profile_v1(run_state: &RunState) -> ActionSupplyProfileV1 {
    let mut profile = ActionSupplyProfileV1::default();
    for relic in &run_state.relics {
        let traits = relic_mechanics_profile_v1(relic.id).action_supply;
        profile.record(ActionSupplySourceV1::Relic(relic.id), traits);
    }
    for card in &run_state.master_deck {
        let traits = card_mechanics_profile_v1(card.id).action_supply;
        profile.record(ActionSupplySourceV1::Card(card.id), traits);
    }
    profile
}
```

Keep the test module from Step 1 below this production code.

- [ ] **Step 5: Run focused tests and formatting**

Run:

```powershell
cargo fmt --all
cargo test --lib action_supply_traits_distinguish_once_burst_repeatable_and_additional
cargo test --lib run_profile_keeps_opening_supply_neutral_and_repeatable_supply_separate
cargo test --lib unknown_or_ordinary_mechanics_do_not_create_action_supply
```

Expected: all three named tests pass; no full seed is run.

- [ ] **Step 6: Commit Task 1**

```powershell
git add -- src/ai/card_semantics_v1.rs src/ai/action_supply_v1.rs src/ai/mod.rs
git commit -m "feat: classify action supply mechanics"
```

Expected: one commit containing only the central traits, aggregation module, and focused tests.

---

### Task 2: Truthful Choker and Pyramid admission evidence

**Files:**

- Modify: `src/ai/strategy/boss_relic_admission.rs`
- Modify: `src/ai/deck_startup_profile_v1.rs`
- Test: unit tests in `src/ai/strategy/boss_relic_admission.rs`

**Interfaces:**

- Consumes: `action_supply_profile_v1(&RunState) -> ActionSupplyProfileV1` from Task 1.
- Produces: `BossRelicAdmissionReason::ActionSupplyFacts` with the seven aggregate counts from Task 1.
- Removes: `BossRelicAdmissionReason::OpeningActionBudgetRequired` and its `opening-action-budget:*` formatter.
- Preserves: boss-relic lanes, classes, burdens, order rank, and all Choker run-debt behavior.

- [ ] **Step 1: Replace the old pair-coupling test with failing boundary tests**

Remove `pyramid_reports_choker_generated_opening_budget_as_evidence` and add:

```rust
#[test]
fn choker_reports_opening_once_supply_without_repeatable_exposure() {
    let baseline = RunState::new(20260711002, 0, false, "Ironclad");
    let mut with_enchiridion = baseline.clone();
    with_enchiridion
        .relics
        .push(RelicState::new(RelicId::Enchiridion));

    let baseline_choker = assess_boss_relic_admission(&baseline, RelicId::VelvetChoker);
    let choker = assess_boss_relic_admission(&with_enchiridion, RelicId::VelvetChoker);

    assert!(choker.reasons.contains(&BossRelicAdmissionReason::ActionSupplyFacts {
        opening_once_options: 1,
        delayed_per_turn_sources: 0,
        same_turn_burst_sources: 0,
        triggered_repeatable_sources: 0,
        additional_play_sources: 0,
        cost_or_resource_compression_sources: 0,
        potentially_recursive_sources: 0,
    }));
    assert_eq!(choker.burden, BossRelicAdmissionBurden::AddedRunDebt);
    assert_eq!(
        boss_relic_admission_order_rank(&choker),
        boss_relic_admission_order_rank(&baseline_choker),
        "opening-once supply is evidence, not a new ordering penalty"
    );
}

#[test]
fn choker_reports_repeatable_and_compressed_action_supply_as_facts() {
    let mut run = RunState::new(20260711002, 0, false, "Ironclad");
    run.relics.push(RelicState::new(RelicId::DeadBranch));
    run.add_card_to_deck(CardId::Corruption);

    let choker = assess_boss_relic_admission(&run, RelicId::VelvetChoker);

    assert!(choker.reasons.contains(&BossRelicAdmissionReason::ActionSupplyFacts {
        opening_once_options: 0,
        delayed_per_turn_sources: 0,
        same_turn_burst_sources: 0,
        triggered_repeatable_sources: 1,
        additional_play_sources: 0,
        cost_or_resource_compression_sources: 1,
        potentially_recursive_sources: 1,
    }));
}

#[test]
fn pyramid_does_not_inherit_existing_choker_enchiridion_supply() {
    let mut run = RunState::new(20260711002, 0, false, "Ironclad");
    run.relics = vec![
        RelicState::new(RelicId::VelvetChoker),
        RelicState::new(RelicId::Enchiridion),
    ];

    let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

    assert!(!pyramid
        .reasons
        .iter()
        .any(|reason| matches!(reason, BossRelicAdmissionReason::ActionSupplyFacts { .. })));
    assert!(!render_boss_relic_admission_compact(&pyramid)
        .contains("opening-action-budget"));
    assert_eq!(pyramid.burden, BossRelicAdmissionBurden::None);
}
```

- [ ] **Step 2: Run the boundary tests to verify they fail**

Run:

```powershell
cargo test --lib choker_reports_opening_once_supply_without_repeatable_exposure
cargo test --lib choker_reports_repeatable_and_compressed_action_supply_as_facts
```

Expected: compilation fails because `ActionSupplyFacts` is not defined and the old Pyramid-specific reason still exists.

- [ ] **Step 3: Replace candidate-specific opening budgeting with neutral Choker facts**

At the top of `src/ai/strategy/boss_relic_admission.rs`, import:

```rust
use crate::ai::action_supply_v1::action_supply_profile_v1;
```

Replace `OpeningActionBudgetRequired` in `BossRelicAdmissionReason` with:

```rust
ActionSupplyFacts {
    opening_once_options: u8,
    delayed_per_turn_sources: u8,
    same_turn_burst_sources: u8,
    triggered_repeatable_sources: u8,
    additional_play_sources: u8,
    cost_or_resource_compression_sources: u8,
    potentially_recursive_sources: u8,
},
```

Remove this block from the Pyramid path:

```rust
if projected_startup.has_pyramid_choker_generated_opening_tradeoff {
    reasons.push(BossRelicAdmissionReason::OpeningActionBudgetRequired {
        generated_options: projected_startup.opening_generated_option_count,
    });
}
```

After the Pyramid coverage block and before burden construction, add:

```rust
if relic == RelicId::VelvetChoker {
    let supply = action_supply_profile_v1(run_state);
    if !supply.is_empty() {
        reasons.push(BossRelicAdmissionReason::ActionSupplyFacts {
            opening_once_options: supply.opening_once_options,
            delayed_per_turn_sources: supply.delayed_per_turn_sources,
            same_turn_burst_sources: supply.same_turn_burst_sources,
            triggered_repeatable_sources: supply.triggered_repeatable_sources,
            additional_play_sources: supply.additional_play_sources,
            cost_or_resource_compression_sources: supply
                .cost_or_resource_compression_sources,
            potentially_recursive_sources: supply.potentially_recursive_sources,
        });
    }
}
```

Replace the old formatter arm with:

```rust
BossRelicAdmissionReason::ActionSupplyFacts {
    opening_once_options,
    delayed_per_turn_sources,
    same_turn_burst_sources,
    triggered_repeatable_sources,
    additional_play_sources,
    cost_or_resource_compression_sources,
    potentially_recursive_sources,
} => format!(
    "action-supply:opening={opening_once_options},delayed={delayed_per_turn_sources},burst={same_turn_burst_sources},triggered={triggered_repeatable_sources},extra={additional_play_sources},compression={cost_or_resource_compression_sources},recursive={potentially_recursive_sources}"
),
```

Do not read the deprecated startup combination flags anywhere in this module after this change.

Add explicit compatibility comments above the two retained fields in
`src/ai/deck_startup_profile_v1.rs`:

```rust
/// Deprecated compatibility fact. New decisions must consume ActionSupplyProfileV1 instead.
#[serde(default)]
pub has_choker_generated_opening_budget: bool,
/// Deprecated compatibility fact. Do not infer candidate-specific burden from this combination.
#[serde(default)]
pub has_pyramid_choker_generated_opening_tradeoff: bool,
```

Keep their existing derivation and Serde defaults so old and current evidence remains readable.

- [ ] **Step 4: Run the focused admission tests**

Run:

```powershell
cargo fmt --all
cargo test --lib choker_reports_opening_once_supply_without_repeatable_exposure
cargo test --lib choker_reports_repeatable_and_compressed_action_supply_as_facts
cargo test --lib pyramid_does_not_inherit_existing_choker_enchiridion_supply
cargo test --lib boss_relic_admission
```

Expected: all named tests pass. Existing lane, burden, and ordering tests remain green.

- [ ] **Step 5: Commit Task 2**

```powershell
git add -- src/ai/strategy/boss_relic_admission.rs src/ai/deck_startup_profile_v1.rs
git commit -m "fix: attribute Choker supply evidence"
```

Expected: one commit limited to admission evidence and its unit tests; no rank constant changes.

---

### Task 3: Exact combat evidence that Choker is binding

**Files:**

- Modify: `src/ai/combat_search_v2/card_pile_value.rs`
- Modify: `src/ai/combat_search_v2/value/facts.rs`
- Modify: `src/ai/combat_search_v2/value/report.rs`
- Modify: `src/ai/combat_search_v2/types/report/frontier.rs`
- Test: unit tests in `src/ai/combat_search_v2/card_pile_value.rs` and `src/ai/combat_search_v2/value/tests.rs`

**Interfaces:**

- Consumes: current `CombatState`, `turn.energy`, `cards_played_this_turn`, and real hand cards.
- Produces: internal `ChokerCapacityV1`, `choker_capacity(&CombatState) -> ChokerCapacityV1`, and serialized `CombatSearchV2ChokerCapacityReport`.
- Preserves: `CombatSearchStateValueV1` fields and ordering, `hand.playable_cards` capacity behavior, and simulator legality.

- [ ] **Step 1: Write failing capacity-boundary tests**

In `src/ai/combat_search_v2/card_pile_value.rs`, replace the existing Choker count test with these tests:

```rust
#[test]
fn choker_capacity_reports_affordable_cards_stranded_by_the_cap() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::VelvetChoker));
    combat.turn.energy = 3;
    combat.turn.counters.cards_played_this_turn = 5;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Strike, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let capacity = choker_capacity(&combat);

    assert!(capacity.has_velvet_choker);
    assert_eq!(capacity.cards_played_this_turn, 5);
    assert_eq!(capacity.remaining_slots, Some(1));
    assert_eq!(capacity.affordable_hand_cards, 3);
    assert_eq!(capacity.representable_affordable_cards, 1);
    assert_eq!(capacity.stranded_affordable_cards, 2);
    assert_eq!(hand_value(&combat).playable_cards, 1);
}

#[test]
fn choker_capacity_is_not_binding_below_remaining_slots() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::VelvetChoker));
    combat.turn.energy = 1;
    combat.turn.counters.cards_played_this_turn = 4;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Carnage, 12),
    ];

    let capacity = choker_capacity(&combat);

    assert_eq!(capacity.remaining_slots, Some(2));
    assert_eq!(capacity.affordable_hand_cards, 1);
    assert_eq!(capacity.representable_affordable_cards, 1);
    assert_eq!(capacity.stranded_affordable_cards, 0);
}

#[test]
fn absent_choker_reports_unbounded_capacity_without_stranding() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Strike, 12),
    ];

    let capacity = choker_capacity(&combat);

    assert!(!capacity.has_velvet_choker);
    assert_eq!(capacity.remaining_slots, None);
    assert_eq!(capacity.affordable_hand_cards, 2);
    assert_eq!(capacity.representable_affordable_cards, 2);
    assert_eq!(capacity.stranded_affordable_cards, 0);
}
```

In `src/ai/combat_search_v2/value/tests.rs`, add the relic import and a report-throughput test:

```rust
use crate::content::relics::{RelicId, RelicState};

#[test]
fn frontier_report_carries_choker_binding_facts_without_changing_ordering() {
    let mut node = test_node();
    node.combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::VelvetChoker));
    node.combat.turn.energy = 3;
    node.combat.turn.counters.cards_played_this_turn = 5;
    node.combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Strike, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let before = combat_search_state_value(&node);
    let report = combat_search_frontier_value_report(&node);
    let after = combat_search_state_value(&node);

    assert_eq!(before, after);
    assert_eq!(report.choker_capacity.remaining_slots, Some(1));
    assert_eq!(report.choker_capacity.affordable_hand_cards, 3);
    assert_eq!(report.choker_capacity.representable_affordable_cards, 1);
    assert_eq!(report.choker_capacity.stranded_affordable_cards, 2);
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run:

```powershell
cargo test --lib choker_capacity_reports_affordable_cards_stranded_by_the_cap
cargo test --lib frontier_report_carries_choker_binding_facts_without_changing_ordering
```

Expected: compilation fails because `ChokerCapacityV1`, `choker_capacity`, and the public report
field do not exist.

- [ ] **Step 3: Implement exact capacity facts without changing ordering**

In `src/ai/combat_search_v2/card_pile_value.rs`, add:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ChokerCapacityV1 {
    pub(super) has_velvet_choker: bool,
    pub(super) cards_played_this_turn: u8,
    pub(super) remaining_slots: Option<u8>,
    pub(super) affordable_hand_cards: u8,
    pub(super) representable_affordable_cards: u8,
    pub(super) stranded_affordable_cards: u8,
}

pub(super) fn choker_capacity(combat: &CombatState) -> ChokerCapacityV1 {
    let has_velvet_choker = combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::VelvetChoker);
    let cards_played_this_turn = combat.turn.counters.cards_played_this_turn;
    let remaining_slots = has_velvet_choker
        .then(|| 6u8.saturating_sub(cards_played_this_turn));
    let affordable_hand_cards = combat
        .zones
        .hand
        .iter()
        .filter(|card| {
            let cost = card.cost_for_turn_java();
            cost >= 0 && cost <= combat.turn.energy as i32
        })
        .count()
        .min(u8::MAX as usize) as u8;
    let representable_affordable_cards = remaining_slots
        .map_or(affordable_hand_cards, |slots| affordable_hand_cards.min(slots));
    ChokerCapacityV1 {
        has_velvet_choker,
        cards_played_this_turn,
        remaining_slots,
        affordable_hand_cards,
        representable_affordable_cards,
        stranded_affordable_cards: affordable_hand_cards
            .saturating_sub(representable_affordable_cards),
    }
}
```

Replace `hand_value` and remove the now-redundant `remaining_card_play_capacity` helper:

```rust
pub(super) fn hand_value(combat: &CombatState) -> CardPileValueV1 {
    let mut value = card_pile_value(combat.zones.hand.iter(), combat.turn.energy as i32);
    value.playable_cards = choker_capacity(combat).representable_affordable_cards as i32;
    value
}
```

Do not add any Choker-capacity field to `CombatSearchStateValueV1` or its `Ord` implementation.

- [ ] **Step 4: Carry capacity facts into the public frontier report**

In `src/ai/combat_search_v2/types/report/frontier.rs`, add:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ChokerCapacityReport {
    pub has_velvet_choker: bool,
    pub cards_played_this_turn: u8,
    pub remaining_slots: Option<u8>,
    pub affordable_hand_cards: u8,
    pub representable_affordable_cards: u8,
    pub stranded_affordable_cards: u8,
}
```

Add this field beside `hand` in `CombatSearchV2FrontierValueReport`:

```rust
pub choker_capacity: CombatSearchV2ChokerCapacityReport,
```

In `src/ai/combat_search_v2/card_pile_value.rs`, add this converter:

```rust
pub(super) fn choker_capacity_report(
    capacity: ChokerCapacityV1,
) -> CombatSearchV2ChokerCapacityReport {
    CombatSearchV2ChokerCapacityReport {
        has_velvet_choker: capacity.has_velvet_choker,
        cards_played_this_turn: capacity.cards_played_this_turn,
        remaining_slots: capacity.remaining_slots,
        affordable_hand_cards: capacity.affordable_hand_cards,
        representable_affordable_cards: capacity.representable_affordable_cards,
        stranded_affordable_cards: capacity.stranded_affordable_cards,
    }
}
```

In `src/ai/combat_search_v2/value/facts.rs`, import `choker_capacity` and `ChokerCapacityV1`, add this field to `CombatSearchCoreValueFactsV1`, and initialize it:

```rust
pub(in crate::ai::combat_search_v2::value) choker_capacity: ChokerCapacityV1,

// Inside combat_search_core_value_facts:
choker_capacity: choker_capacity(combat),
```

In `src/ai/combat_search_v2/value/report.rs`, import `choker_capacity_report` with `card_pile_value_report` and add:

```rust
choker_capacity: choker_capacity_report(facts.choker_capacity),
```

Do not change `COMBAT_SEARCH_FRONTIER_VALUE_POLICY`: this pass adds diagnostics but does not change the comparison policy.

- [ ] **Step 5: Run focused and report-adjacent tests**

Run:

```powershell
cargo fmt --all
cargo test --lib choker_capacity_reports_affordable_cards_stranded_by_the_cap
cargo test --lib choker_capacity_is_not_binding_below_remaining_slots
cargo test --lib absent_choker_reports_unbounded_capacity_without_stranding
cargo test --lib card_pile_value
cargo test --lib frontier_report_carries_choker_binding_facts_without_changing_ordering
```

Expected: all named tests pass; existing Pyramid draw-capacity tests remain green.

- [ ] **Step 6: Run final bounded verification**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected:

- formatting exits 0;
- the full library test binary reports zero failures;
- `architecture_runtime_boundaries` reports zero failures;
- `git diff --check` emits no whitespace errors;
- no complete seed is executed.

- [ ] **Step 7: Commit Task 3**

```powershell
git add -- src/ai/combat_search_v2/card_pile_value.rs src/ai/combat_search_v2/value/facts.rs src/ai/combat_search_v2/value/report.rs src/ai/combat_search_v2/value/tests.rs src/ai/combat_search_v2/types/report/frontier.rs
git commit -m "feat: report binding Choker capacity"
```

Expected: one commit containing exact combat binding facts and public diagnostics, with no state-order or rank changes.

---

## Completion Checklist

- [ ] `ActionSupplyTraitsV1` is owned by `card_semantics_v1`; no decision consumer has an independent ID list.
- [ ] Enchiridion and Toolbox are opening-once facts, not repeatable exposure.
- [ ] Nilry's Codex, Dead Branch, Blade Dance, Double Tap, Corruption, and Offering have the planned structural traits.
- [ ] `ActionSupplyProfileV1` contains facts and source evidence but no aggregate score.
- [ ] Choker admission reports neutral supply facts without changing order rank.
- [ ] Pyramid admission no longer emits `OpeningActionBudgetRequired` for an existing Choker/Enchiridion condition.
- [ ] Deprecated startup combination flags remain serialized and readable but are no longer consumed by boss-relic admission.
- [ ] Combat reports exact remaining slots, affordable actions, representable actions, and stranded actions.
- [ ] Combat state ordering is unchanged.
- [ ] Full library and architecture-boundary tests pass.
- [ ] No complete seed was run.
