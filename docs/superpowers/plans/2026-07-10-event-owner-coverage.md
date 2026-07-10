# Complete Event Owner Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give all 52 regular events an explicit executable owner policy, keep `Neow` with `NeowStart`, and make future missing event owners fail compilation.

**Architecture:** Keep the mature policies and shared facts in `owner_policy.rs`, place the 15 conservative completion rules in a focused `owner_policy/coverage_rules.rs`, and protect them with a separate real-option contract matrix. Once all events are explicit, remove marker discovery and narrow the content event owner to event-room selection while the existing `RunChoice` owner remains the sole pending deck-selection owner.

**Tech Stack:** Rust 2021, Cargo unit tests, typed `EventOptionSemantics`, `RunControlSession`, `DecisionSurface`, existing event-resource budgets and deck-mutation compiler.

## Global Constraints

- Reliability takes priority over event expected value.
- No panel, fixed-seed, frontier, checkpoint, or source-identity command is an acceptance gate.
- Do not add a generic "first enabled option" fallback.
- Do not change event mechanics, rewards, RNG consumption, or combat-search behavior.
- Keep every owner decision hidden-free and based only on public run state plus declared route facts.
- The full library suite runs once, after focused red-green cycles are complete.
- Use `apply_patch` for source and documentation edits.

## File Structure

- Create `src/content/events/owner_policy/coverage_rules.rs`: conservative policies for the 15 newly explicit regular events only.
- Create `src/content/events/owner_policy/coverage_tests.rs`: shared fixtures and the selector-to-real-option contract matrix for those 15 events.
- Modify `src/content/events/owner_policy.rs`: register the focused modules, add exhaustive dispatch, and retire marker/pending-selection duplication.
- Modify `src/runtime/branch/owner_audit/event_owner_bridge.rs`: consume the narrowed selector API.
- Modify `src/runtime/branch/owner_audit/boundary_router.rs`: add stable ownership tests for `Neow`, regular events, and event-origin deck selections.
- Modify `src/state/events/mod.rs`: remove the obsolete owner marker from event option semantics.
- Modify `src/content/events/{fountain,knowing_skull,sensory_stone,sssserpent,woman_in_blue}.rs`: expose narrow policy facts where needed and remove marker writes.
- Modify `src/ai/event_policy_v1/tests.rs`: remove the obsolete marker field from its explicit semantics fixture.
- Modify `docs/superpowers/specs/2026-07-10-event-owner-coverage-design.md`: mark implementation verified only after all checks pass.

---

### Task 1: Forced-flow event owners and real-option contract harness

**Files:**
- Create: `src/content/events/owner_policy/coverage_rules.rs`
- Create: `src/content/events/owner_policy/coverage_tests.rs`
- Modify: `src/content/events/owner_policy.rs`
- Test: `src/content/events/owner_policy/coverage_tests.rs`

**Interfaces:**
- Consumes: `event_owner_policy_action(&EngineState, &RunState) -> Result<EventOwnerAction, EventOwnerPolicyGap>` during the migration.
- Produces: `coverage_rules::{gremlin_wheel_choice, lab_choice, colosseum_choice, the_joust_choice}(&RunState) -> EventOwnerOptionSelector` and the shared `assert_unique_selector` test helper.

- [ ] **Step 1: Write the failing forced-flow contract tests**

Create `src/content/events/owner_policy/coverage_tests.rs` with this content:

```rust
use super::*;
use crate::engine::event_handler::get_event_options;
use crate::state::events::{EventId, EventState};

fn event_run(event_id: EventId, screen: usize) -> RunState {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    let mut event_state = EventState::new(event_id);
    event_state.current_screen = screen;
    run_state.event_state = Some(event_state);
    run_state
}

fn assert_unique_selector(run_state: &RunState, expected: EventOwnerOptionSelector) {
    let action = event_owner_policy_action(&EngineState::EventRoom, run_state).unwrap();
    let EventOwnerAction::ChooseOption(selector) = action else {
        panic!("event-room owner must choose an event option");
    };
    assert_eq!(selector, expected);

    let options = get_event_options(run_state);
    let matches = options
        .iter()
        .enumerate()
        .filter(|(index, option)| {
            !option.ui.disabled && selector.matches(*index, &option.semantics)
        })
        .count();
    assert_eq!(matches, 1, "selector must resolve to one enabled real option");
}

#[test]
fn forced_flow_events_select_one_real_option_on_every_screen() {
    let cases = [
        (EventId::GremlinWheelGame, 0, action(EventActionKind::Special)),
        (EventId::GremlinWheelGame, 1, action(EventActionKind::Leave)),
        (EventId::Lab, 0, action(EventActionKind::Gain)),
        (EventId::Lab, 1, action(EventActionKind::Leave)),
        (EventId::Colosseum, 0, action(EventActionKind::Continue)),
        (EventId::Colosseum, 1, action(EventActionKind::Fight)),
        (EventId::Colosseum, 2, action(EventActionKind::Leave)),
        (EventId::Colosseum, 3, action(EventActionKind::Leave)),
        (EventId::TheJoust, 0, action(EventActionKind::Continue)),
        (EventId::TheJoust, 1, option_index(0)),
        (EventId::TheJoust, 2, action(EventActionKind::Continue)),
        (EventId::TheJoust, 3, action(EventActionKind::Continue)),
        (EventId::TheJoust, 4, action(EventActionKind::Leave)),
        (EventId::TheJoust, 5, action(EventActionKind::Leave)),
    ];

    for (event_id, screen, expected) in cases {
        assert_unique_selector(&event_run(event_id, screen), expected);
    }
}
```

Register it at the bottom of `owner_policy.rs` without moving the existing tests:

```rust
#[cfg(test)]
mod coverage_tests;
```

- [ ] **Step 2: Run the focused test and verify the real failure**

Run:

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::forced_flow_events_select_one_real_option_on_every_screen -- --exact
```

Expected: FAIL at the first uncovered event with `MissingMarkedPolicy(GremlinWheelGame)`; it must not fail from malformed test state.

- [ ] **Step 3: Add the focused forced-flow policies**

Create `src/content/events/owner_policy/coverage_rules.rs`:

```rust
use crate::state::events::EventActionKind;
use crate::state::run::RunState;

use super::{action, event_screen, option_index, EventOwnerOptionSelector};

pub(super) fn gremlin_wheel_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Special),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn lab_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Gain),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn colosseum_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => action(EventActionKind::Fight),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn the_joust_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 | 2 | 3 => action(EventActionKind::Continue),
        1 => option_index(0),
        _ => action(EventActionKind::Leave),
    }
}
```

At the top of `owner_policy.rs`, register and import the focused module:

```rust
mod coverage_rules;

use coverage_rules::{
    colosseum_choice, gremlin_wheel_choice, lab_choice, the_joust_choice,
};
```

Add these explicit arms before the existing wildcard fallback:

```rust
EventId::Colosseum => return Ok(choose(colosseum_choice(run_state))),
EventId::GremlinWheelGame => return Ok(choose(gremlin_wheel_choice(run_state))),
EventId::Lab => return Ok(choose(lab_choice(run_state))),
EventId::TheJoust => return Ok(choose(the_joust_choice(run_state))),
```

- [ ] **Step 4: Run focused mechanics and owner contracts**

Run:

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::forced_flow_events_select_one_real_option_on_every_screen -- --exact
cargo test --lib content::events::colosseum
cargo test --lib content::events::gremlin_wheel
cargo test --lib content::events::lab
cargo test --lib content::events::the_joust
```

Expected: all commands PASS. The owner test must prove one enabled real option per listed screen.

- [ ] **Step 5: Commit the forced-flow slice**

```powershell
git add src/content/events/owner_policy.rs src/content/events/owner_policy/coverage_rules.rs src/content/events/owner_policy/coverage_tests.rs docs/superpowers/specs/2026-07-10-event-owner-coverage-design.md docs/superpowers/plans/2026-07-10-event-owner-coverage.md
git commit -m "feat: cover forced-flow event owners"
```

---

### Task 2: Free and deck-positive event owners

**Files:**
- Modify: `src/content/events/owner_policy/coverage_rules.rs`
- Modify: `src/content/events/owner_policy/coverage_tests.rs`
- Modify: `src/content/events/owner_policy.rs`
- Modify: `src/content/events/fountain.rs`

**Interfaces:**
- Consumes: `has_omamori_charge`, `has_safe_purge_target`, and the public hidden-free `best_duplicate_target_for_shop_v1` premium-target fact.
- Produces: six explicit selectors for `GoldenShrine`, `FountainOfCurseCleansing`, `UpgradeShrine`, `AccursedBlacksmith`, `Duplicator`, and `NoteForYourself`.

- [ ] **Step 1: Add failing state-dependent contract tests**

Append these tests to `coverage_tests.rs` and add imports for `CardId`, `RelicId`, and `RelicState`:

```rust
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};

#[test]
fn golden_shrine_uses_omamori_for_desecrate_and_otherwise_prays() {
    let run_state = event_run(EventId::GoldenShrine, 0);
    assert_unique_selector(&run_state, effect(EventEffect::GainGold(100)));

    let mut protected = event_run(EventId::GoldenShrine, 0);
    protected.relics.push(RelicState::new(RelicId::Omamori));
    assert_unique_selector(
        &protected,
        effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Regret),
        }),
    );
}

#[test]
fn fountain_upgrade_blacksmith_duplicate_and_note_use_real_deck_facts() {
    let empty_fountain = event_run(EventId::FountainOfCurseCleansing, 0);
    assert_unique_selector(&empty_fountain, action(EventActionKind::Leave));

    let mut cursed = event_run(EventId::FountainOfCurseCleansing, 0);
    cursed.add_card_to_deck(CardId::Injury);
    assert_unique_selector(&cursed, action(EventActionKind::DeckOperation));

    let mut empty_upgrade = event_run(EventId::UpgradeShrine, 0);
    empty_upgrade.master_deck.clear();
    assert_unique_selector(&empty_upgrade, action(EventActionKind::Leave));
    let mut upgrade = event_run(EventId::UpgradeShrine, 0);
    upgrade.add_card_to_deck(CardId::Bash);
    assert_unique_selector(&upgrade, action(EventActionKind::DeckOperation));

    let mut forge = event_run(EventId::AccursedBlacksmith, 0);
    forge.add_card_to_deck(CardId::Bash);
    assert_unique_selector(&forge, action(EventActionKind::DeckOperation));
    let mut plain_blacksmith = event_run(EventId::AccursedBlacksmith, 0);
    plain_blacksmith.master_deck.clear();
    assert_unique_selector(&plain_blacksmith, action(EventActionKind::Leave));

    let mut empty_duplicate = event_run(EventId::Duplicator, 0);
    empty_duplicate.master_deck.clear();
    assert_unique_selector(&empty_duplicate, action(EventActionKind::Leave));
    let mut premium_duplicate = event_run(EventId::Duplicator, 0);
    premium_duplicate.add_card_to_deck(CardId::Offering);
    assert_unique_selector(
        &premium_duplicate,
        action(EventActionKind::DeckOperation),
    );

    let default_note = event_run(EventId::NoteForYourself, 1);
    assert_unique_selector(&default_note, action(EventActionKind::Decline));
    let mut useful_note = event_run(EventId::NoteForYourself, 1);
    useful_note.note_for_yourself_card = CardId::Offering;
    useful_note.add_card_to_deck(CardId::Strike);
    assert_unique_selector(&useful_note, action(EventActionKind::DeckOperation));
}

#[test]
fn deck_positive_events_cover_intro_and_completion_screens() {
    let cases = [
        (EventId::GoldenShrine, 1, action(EventActionKind::Leave)),
        (
            EventId::FountainOfCurseCleansing,
            1,
            action(EventActionKind::Leave),
        ),
        (EventId::UpgradeShrine, 1, action(EventActionKind::Leave)),
        (
            EventId::AccursedBlacksmith,
            1,
            action(EventActionKind::Leave),
        ),
        (EventId::Duplicator, 1, action(EventActionKind::Leave)),
        (EventId::NoteForYourself, 0, action(EventActionKind::Continue)),
        (EventId::NoteForYourself, 2, action(EventActionKind::Leave)),
    ];
    for (event_id, screen, expected) in cases {
        assert_unique_selector(&event_run(event_id, screen), expected);
    }
}
```

- [ ] **Step 2: Run the new tests and verify they fail on missing policy**

Run:

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::golden_shrine_uses_omamori_for_desecrate_and_otherwise_prays -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::fountain_upgrade_blacksmith_duplicate_and_note_use_real_deck_facts -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::deck_positive_events_cover_intro_and_completion_screens -- --exact
```

Expected: all three FAIL with `MissingMarkedPolicy` for the first uncovered event in each test.

- [ ] **Step 3: Expose the fountain fact and implement the six policies**

Change the fountain helper signature to:

```rust
pub(crate) fn removable_curse_count(run_state: &RunState) -> usize {
```

Append to `coverage_rules.rs`:

```rust
use crate::ai::deck_mutation_compiler_v1::best_duplicate_target_for_shop_v1;
use crate::content::cards::CardId;
use crate::state::events::{EventCardKind, EventEffect};

use super::{effect, has_omamori_charge, has_safe_purge_target};

pub(super) fn golden_shrine_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_omamori_charge(run_state) {
        return effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Regret),
        });
    }
    let gold = if run_state.ascension_level >= 15 { 50 } else { 100 };
    effect(EventEffect::GainGold(gold))
}

pub(super) fn fountain_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if crate::content::events::fountain::removable_curse_count(run_state) > 0 => {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn upgrade_shrine_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if run_state
            .master_deck
            .iter()
            .any(crate::state::core::master_deck_card_can_upgrade) =>
        {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn accursed_blacksmith_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade)
    {
        action(EventActionKind::DeckOperation)
    } else if has_omamori_charge(run_state) {
        action(EventActionKind::Trade)
    } else {
        action(EventActionKind::Leave)
    }
}

pub(super) fn duplicator_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if best_duplicate_target_for_shop_v1(run_state).is_some() => {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn note_for_yourself_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 if !crate::content::events::note_for_yourself::default_note_is_ignorable(run_state)
            && has_safe_purge_target(run_state) =>
        {
            action(EventActionKind::DeckOperation)
        }
        1 => action(EventActionKind::Decline),
        _ => action(EventActionKind::Leave),
    }
}
```

Extend the `coverage_rules` import and the migration dispatch with exactly:

```rust
use coverage_rules::{
    accursed_blacksmith_choice, colosseum_choice, duplicator_choice,
    fountain_choice, golden_shrine_choice, gremlin_wheel_choice, lab_choice,
    note_for_yourself_choice, the_joust_choice, upgrade_shrine_choice,
};

EventId::AccursedBlacksmith => {
    return Ok(choose(accursed_blacksmith_choice(run_state)))
}
EventId::Duplicator => return Ok(choose(duplicator_choice(run_state))),
EventId::FountainOfCurseCleansing => {
    return Ok(choose(fountain_choice(run_state)))
}
EventId::GoldenShrine => return Ok(choose(golden_shrine_choice(run_state))),
EventId::NoteForYourself => {
    return Ok(choose(note_for_yourself_choice(run_state)))
}
EventId::UpgradeShrine => return Ok(choose(upgrade_shrine_choice(run_state))),
```

- [ ] **Step 4: Run focused owner, compiler, and mechanic tests**

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::golden_shrine_uses_omamori_for_desecrate_and_otherwise_prays -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::fountain_upgrade_blacksmith_duplicate_and_note_use_real_deck_facts -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::deck_positive_events_cover_intro_and_completion_screens -- --exact
cargo test --lib ai::deck_mutation_compiler_v1
cargo test --lib content::events::fountain
cargo test --lib content::events::upgrade_shrine
cargo test --lib content::events::accursed_blacksmith
cargo test --lib content::events::duplicator
cargo test --lib content::events::note_for_yourself
```

Expected: all PASS.

- [ ] **Step 5: Commit the deck-positive slice**

```powershell
git add src/content/events/owner_policy.rs src/content/events/owner_policy/coverage_rules.rs src/content/events/owner_policy/coverage_tests.rs src/content/events/fountain.rs
git commit -m "feat: cover deck-positive event owners"
```

---

### Task 3: Resource and risk event owners

**Files:**
- Modify: `src/content/events/owner_policy/coverage_rules.rs`
- Modify: `src/content/events/owner_policy/coverage_tests.rs`
- Modify: `src/content/events/owner_policy.rs`
- Modify: `src/content/events/knowing_skull.rs`
- Modify: `src/content/events/sensory_stone.rs`

**Interfaces:**
- Consumes: `event_resource_budget_for`, `hp_loss_class`, `spend_reserved_or_worse`, `has_omamori_charge`, and `EventGainClass`.
- Produces: explicit policies for `Ssssserpent`, `Addict`, `KnowingSkull`, `SensoryStone`, and `SecretPortal`.

- [ ] **Step 1: Add failing resource/risk policy tests**

Append to `coverage_tests.rs`:

```rust
#[test]
fn curse_trades_require_omamori_and_addict_preserves_gold_reserve() {
    let serpent = event_run(EventId::Ssssserpent, 0);
    assert_unique_selector(&serpent, action(EventActionKind::Decline));
    let mut protected_serpent = event_run(EventId::Ssssserpent, 0);
    protected_serpent
        .relics
        .push(RelicState::new(RelicId::Omamori));
    assert_unique_selector(&protected_serpent, action(EventActionKind::Accept));

    let poor_addict = event_run(EventId::Addict, 0);
    assert_unique_selector(&poor_addict, action(EventActionKind::Leave));
    let mut protected_addict = event_run(EventId::Addict, 0);
    protected_addict
        .relics
        .push(RelicState::new(RelicId::Omamori));
    assert_unique_selector(&protected_addict, option_index(1));
    let mut funded_addict = event_run(EventId::Addict, 0);
    funded_addict.gold = 300;
    assert_unique_selector(&funded_addict, option_index(0));
}

#[test]
fn knowing_skull_sensory_stone_and_secret_portal_use_survival_gates() {
    let healthy_skull = event_run(EventId::KnowingSkull, 1);
    assert_unique_selector(&healthy_skull, effect(EventEffect::GainGold(90)));
    let mut low_skull = event_run(EventId::KnowingSkull, 1);
    low_skull.current_hp = 12;
    assert_unique_selector(&low_skull, action(EventActionKind::Leave));
    let mut blocked_skull = event_run(EventId::KnowingSkull, 1);
    blocked_skull.relics.push(RelicState::new(RelicId::Ectoplasm));
    assert_unique_selector(&blocked_skull, action(EventActionKind::Leave));

    let high_focus = event_run(EventId::SensoryStone, 1);
    assert_unique_selector(&high_focus, option_index(2));
    let mut low_focus = event_run(EventId::SensoryStone, 1);
    low_focus.current_hp = 20;
    assert_unique_selector(&low_focus, option_index(0));

    assert_unique_selector(
        &event_run(EventId::SecretPortal, 0),
        action(EventActionKind::Decline),
    );
    assert_unique_selector(
        &event_run(EventId::SecretPortal, 1),
        action(EventActionKind::Special),
    );
}

#[test]
fn risk_events_cover_intro_confirmation_and_completion_screens() {
    let cases = [
        (EventId::Ssssserpent, 1, action(EventActionKind::Continue)),
        (EventId::Ssssserpent, 99, action(EventActionKind::Leave)),
        (EventId::Addict, 1, action(EventActionKind::Leave)),
        (EventId::KnowingSkull, 0, action(EventActionKind::Continue)),
        (EventId::KnowingSkull, 2, action(EventActionKind::Leave)),
        (EventId::SensoryStone, 0, action(EventActionKind::Continue)),
        (EventId::SensoryStone, 2, action(EventActionKind::Leave)),
        (EventId::SecretPortal, 2, action(EventActionKind::Leave)),
    ];
    for (event_id, screen, expected) in cases {
        assert_unique_selector(&event_run(event_id, screen), expected);
    }
}
```

- [ ] **Step 2: Run the tests and verify the missing-policy failures**

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::curse_trades_require_omamori_and_addict_preserves_gold_reserve -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::knowing_skull_sensory_stone_and_secret_portal_use_survival_gates -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::risk_events_cover_intro_confirmation_and_completion_screens -- --exact
```

Expected: FAIL because at least `Addict`, `KnowingSkull`, or `SecretPortal` still returns `MissingMarkedPolicy`; `Ssssserpent` and `SensoryStone` may still pass through markers at this stage.

- [ ] **Step 3: Expose narrow event facts and implement the five policies**

Change these private helpers to crate-visible facts:

```rust
// knowing_skull.rs
pub(crate) fn gold_cost(state: i32) -> i32 {
    BASE_COST + gold_n(state)
}

// sensory_stone.rs
pub(crate) fn sensory_focus_choice(run_state: &RunState) -> usize {
```

Append to `coverage_rules.rs`:

```rust
use crate::ai::event_resource_budget::EventGainClass;

use super::{
    event_resource_budget_for, hp_loss_class, spend_reserved_or_worse,
};

pub(super) fn ssssserpent_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if has_omamori_charge(run_state) => action(EventActionKind::Accept),
        0 => action(EventActionKind::Decline),
        1 => action(EventActionKind::Continue),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn addict_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_omamori_charge(run_state) {
        return option_index(1);
    }
    let budget = event_resource_budget_for(run_state);
    let can_pay = run_state.gold >= 85
        && run_state.gold - 85 >= budget.gold.estimated_next_shop_purge_cost
        && !spend_reserved_or_worse(budget.gold.spend_75);
    if can_pay {
        option_index(0)
    } else {
        action(EventActionKind::Leave)
    }
}

pub(super) fn knowing_skull_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => {
            let state = run_state
                .event_state
                .as_ref()
                .map(|event| event.internal_state)
                .unwrap_or_default();
            let cost = crate::content::events::knowing_skull::gold_cost(state);
            let budget = event_resource_budget_for(run_state);
            if budget.gold.gold_gain != EventGainClass::Blocked
                && !spend_reserved_or_worse(hp_loss_class(&budget, cost))
            {
                effect(EventEffect::GainGold(90))
            } else {
                action(EventActionKind::Leave)
            }
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn sensory_stone_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => option_index(crate::content::events::sensory_stone::sensory_focus_choice(
            run_state,
        )),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn secret_portal_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Decline),
        1 => action(EventActionKind::Special),
        _ => action(EventActionKind::Leave),
    }
}
```

Extend the `coverage_rules` import and the migration dispatch with exactly:

```rust
use coverage_rules::{
    accursed_blacksmith_choice, addict_choice, colosseum_choice, duplicator_choice,
    fountain_choice, golden_shrine_choice, gremlin_wheel_choice, knowing_skull_choice,
    lab_choice, note_for_yourself_choice, secret_portal_choice, sensory_stone_choice,
    ssssserpent_choice, the_joust_choice, upgrade_shrine_choice,
};

EventId::Addict => return Ok(choose(addict_choice(run_state))),
EventId::KnowingSkull => return Ok(choose(knowing_skull_choice(run_state))),
EventId::SecretPortal => return Ok(choose(secret_portal_choice(run_state))),
EventId::SensoryStone => return Ok(choose(sensory_stone_choice(run_state))),
EventId::Ssssserpent => return Ok(choose(ssssserpent_choice(run_state))),
```

- [ ] **Step 4: Run the risk contracts and event mechanics**

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::curse_trades_require_omamori_and_addict_preserves_gold_reserve -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::knowing_skull_sensory_stone_and_secret_portal_use_survival_gates -- --exact
cargo test --lib content::events::owner_policy::coverage_tests::risk_events_cover_intro_confirmation_and_completion_screens -- --exact
cargo test --lib content::events::sssserpent
cargo test --lib content::events::addict
cargo test --lib content::events::knowing_skull
cargo test --lib content::events::sensory_stone
cargo test --lib content::events::secret_portal
```

Expected: all PASS.

- [ ] **Step 5: Commit the risk-policy slice**

```powershell
git add src/content/events/owner_policy.rs src/content/events/owner_policy/coverage_rules.rs src/content/events/owner_policy/coverage_tests.rs src/content/events/knowing_skull.rs src/content/events/sensory_stone.rs
git commit -m "feat: cover risk event owners"
```

---

### Task 4: Exhaustive dispatch, marker retirement, and single pending-choice owner

**Files:**
- Modify: `src/content/events/owner_policy.rs`
- Modify: `src/content/events/owner_policy/coverage_tests.rs`
- Modify: `src/runtime/branch/owner_audit/event_owner_bridge.rs`
- Modify: `src/runtime/branch/owner_audit/boundary_router.rs`
- Modify: `src/runtime/branch/owner_audit/run_choice_owner.rs`
- Modify: `src/state/events/mod.rs`
- Modify: `src/content/events/sssserpent.rs`
- Modify: `src/content/events/sensory_stone.rs`
- Modify: `src/content/events/woman_in_blue.rs`
- Modify: `src/ai/event_policy_v1/tests.rs`

**Interfaces:**
- Replaces: `event_owner_policy_action(&EngineState, &RunState) -> Result<EventOwnerAction, EventOwnerPolicyGap>`.
- Produces: `event_owner_policy_selector(&RunState) -> Result<EventOwnerOptionSelector, EventOwnerPolicyGap>` with `EventOwnerPolicyGap::{MissingEventState, NeowOwnedByNeowStart}`.
- Preserves: `RunPendingChoice` is routed and executed only by `Owner::RunChoice`.

- [ ] **Step 1: Add the routing and separate-Neow tests**

Add to `coverage_tests.rs`:

```rust
#[test]
fn neow_is_owned_separately_from_regular_event_policy() {
    let run_state = event_run(EventId::Neow, 0);
    assert_eq!(
        event_owner_policy_selector(&run_state),
        Err(EventOwnerPolicyGap::NeowOwnedByNeowStart)
    );
}
```

Add to `boundary_router.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
    use sts_simulator::state::events::{EventId, EventState};
    use sts_simulator::state::selection::DomainEventSource;

    #[test]
    fn neow_regular_events_and_event_deck_choices_have_distinct_owners() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = Some(EventState::new(EventId::Neow));
        assert!(matches!(owner_for_current_boundary(&session), Some(Owner::NeowStart)));

        session.run_state.event_state = Some(EventState::new(EventId::GoldenShrine));
        assert!(matches!(
            owner_for_current_boundary(&session),
            Some(Owner::Event(EventId::GoldenShrine))
        ));

        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Upgrade,
            source: DomainEventSource::Event(EventId::UpgradeShrine),
            return_state: Box::new(EngineState::EventRoom),
        });
        assert!(matches!(owner_for_current_boundary(&session), Some(Owner::RunChoice)));
    }
}
```

Add to `run_choice_owner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{RunControlCommand, RunControlConfig};
    use sts_simulator::state::events::EventId;
    use sts_simulator::state::selection::{DomainEventSource, SelectionScope};

    #[test]
    fn event_origin_upgrade_produces_one_typed_run_choice_target() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.add_card_to_deck(CardId::Bash);
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Upgrade,
            source: DomainEventSource::Event(EventId::UpgradeShrine),
            return_state: Box::new(EngineState::EventRoom),
        });

        let OwnerDecision::Candidates(choices) = run_choice_owner_decision(&session) else {
            panic!("event-origin upgrade must be owned by RunChoice");
        };
        let [choice] = choices.as_slice() else {
            panic!("RunChoice must produce one committed candidate");
        };
        let RunControlCommand::Input(ClientInput::SubmitSelection(resolution)) = &choice.action
        else {
            panic!("RunChoice candidate must submit a typed selection");
        };
        assert_eq!(resolution.scope, SelectionScope::Deck);
        assert_eq!(resolution.selected_card_uuids().len(), 1);
    }
}
```

- [ ] **Step 2: Run the new tests and verify the API test fails**

```powershell
cargo test --lib content::events::owner_policy::coverage_tests::neow_is_owned_separately_from_regular_event_policy -- --exact
cargo test --lib runtime::branch::owner_audit::boundary_router::tests::neow_regular_events_and_event_deck_choices_have_distinct_owners -- --exact
```

Expected: the coverage test FAILS to compile because `event_owner_policy_selector` and `NeowOwnedByNeowStart` do not exist; the boundary ownership characterization test PASSES.

- [ ] **Step 3: Narrow the API and make event dispatch exhaustive**

Replace the policy gap and entry-point shapes with:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventOwnerPolicyGap {
    MissingEventState,
    NeowOwnedByNeowStart,
}

pub fn event_owner_policy_selector(
    run_state: &RunState,
) -> Result<EventOwnerOptionSelector, EventOwnerPolicyGap> {
    let event_id = run_state
        .event_state
        .as_ref()
        .map(|event| event.id)
        .ok_or(EventOwnerPolicyGap::MissingEventState)?;
    let selector = match event_id {
        EventId::BigFish => big_fish_choice(run_state),
        EventId::Cleric => cleric_choice(run_state),
        EventId::DeadAdventurer => dead_adventurer_choice(run_state),
        EventId::GoldenIdol => golden_idol_choice(run_state),
        EventId::LivingWall => living_wall_choice(run_state),
        EventId::Mushrooms => mushrooms_choice(run_state),
        EventId::ScrapOoze => scrap_ooze_choice(run_state),
        EventId::ShiningLight => shining_light_choice(run_state),
        EventId::Ssssserpent => ssssserpent_choice(run_state),
        EventId::WorldOfGoop => world_of_goop_choice(run_state),
        EventId::GoldenWing => golden_wing_choice(run_state),
        EventId::MatchAndKeep => match_and_keep_choice(run_state),
        EventId::GoldenShrine => golden_shrine_choice(run_state),
        EventId::Addict => addict_choice(run_state),
        EventId::BackTotheBasics => back_to_basics_choice(run_state),
        EventId::Beggar => beggar_choice(run_state),
        EventId::Colosseum => colosseum_choice(run_state),
        EventId::CursedTome => cursed_tome_choice(run_state),
        EventId::DrugDealer => drug_dealer_choice(run_state),
        EventId::ForgottenAltar => forgotten_altar_choice(run_state),
        EventId::Ghosts => ghosts_choice(run_state),
        EventId::KnowingSkull => knowing_skull_choice(run_state),
        EventId::MaskedBandits => masked_bandits_choice(run_state),
        EventId::Mausoleum => mausoleum_choice(run_state),
        EventId::Nest => nest_choice(run_state),
        EventId::Nloth => nloth_choice(run_state),
        EventId::TheJoust => the_joust_choice(run_state),
        EventId::TheLibrary => the_library_choice(run_state),
        EventId::Vampires => vampires_choice(run_state),
        EventId::Falling => super::falling_owner::falling_choice(run_state),
        EventId::MindBloom => mind_bloom_choice(run_state),
        EventId::MoaiHead => moai_head_choice(run_state),
        EventId::MysteriousSphere => mysterious_sphere_choice(run_state),
        EventId::SensoryStone => sensory_stone_choice(run_state),
        EventId::TombRedMask => tomb_red_mask_choice(run_state),
        EventId::WindingHalls => winding_halls_choice(run_state),
        EventId::AccursedBlacksmith => accursed_blacksmith_choice(run_state),
        EventId::BonfireElementals => bonfire_choice(run_state),
        EventId::BonfireSpirits => bonfire_choice(run_state),
        EventId::Designer => designer_choice(run_state),
        EventId::Duplicator => duplicator_choice(run_state),
        EventId::FaceTrader => face_trader_choice(run_state),
        EventId::FountainOfCurseCleansing => fountain_choice(run_state),
        EventId::GremlinWheelGame => gremlin_wheel_choice(run_state),
        EventId::Lab => lab_choice(run_state),
        EventId::NoteForYourself => note_for_yourself_choice(run_state),
        EventId::Purifier => purifier_choice(run_state),
        EventId::SecretPortal => secret_portal_choice(run_state),
        EventId::Transmorgrifier => transmorgrifier_choice(run_state),
        EventId::UpgradeShrine => upgrade_shrine_choice(run_state),
        EventId::WeMeetAgain => we_meet_again_choice(run_state),
        EventId::WomanInBlue => woman_in_blue_choice(run_state),
        EventId::Neow => return Err(EventOwnerPolicyGap::NeowOwnedByNeowStart),
    };
    Ok(selector)
}
```

Delete `EventOwnerAction`, `event_run_choice_policy_action`, `deck_mutation_selection`, `choose`, the `OwnerPolicy` selector variant, and the missing/ambiguous-marker fallback and gap variants. Keep `single_deck_mutation_choice` because coverage rules use it to decide whether an optional event deck operation is worthwhile.

Update `event_owner_bridge.rs` to consume the selector directly:

```rust
match sts_simulator::content::events::owner_policy::event_owner_policy_selector(
    &session.run_state,
) {
    Ok(selector) => visible_event_option_decision(session, surface, &selector),
    Err(err) => OwnerDecision::Gap(format!("{err:?}")),
}
```

Update `assert_unique_selector` in `coverage_tests.rs` to call the new API directly:

```rust
let selector = event_owner_policy_selector(run_state).unwrap();
```

- [ ] **Step 4: Remove the obsolete marker representation**

In `state/events/mod.rs`, remove this field and enum:

```rust
pub owner_policy: EventOwnerPolicyKind,

pub enum EventOwnerPolicyKind {
    None,
    ConservativeAuto,
}
```

Remove all `EventOwnerPolicyKind` imports, `owner_policy:` initializers, and `conservative_auto_if` / `focus_owner_policy` marker helpers from `sssserpent.rs`, `sensory_stone.rs`, and `woman_in_blue.rs`. Preserve `sensory_focus_choice` because the explicit policy calls it. Remove the explicit `owner_policy: EventOwnerPolicyKind::None` fixture field and import from `src/ai/event_policy_v1/tests.rs`.

Verify the retirement is complete:

```powershell
rg -n "EventOwnerPolicyKind|owner_policy:|MissingMarkedPolicy|AmbiguousMarkedPolicy|EventOwnerAction|SubmitSelection" src
```

Expected: no matches for the retired event-owner symbols. Matches for unrelated `SubmitSelection` uses are allowed only if the final query is split and shown to come from other typed owners; `EventOwnerAction::SubmitSelection` must have no match.

- [ ] **Step 5: Run all focused owner and routing contracts**

```powershell
cargo test --lib content::events::owner_policy::coverage_tests
cargo test --lib content::events::owner_policy::tests
cargo test --lib runtime::branch::owner_audit::boundary_router
cargo test --lib runtime::branch::owner_audit::event_owner_bridge
cargo test --lib runtime::branch::owner_audit::run_choice_owner
```

Expected: all PASS. Compilation itself proves the `EventId` match is exhaustive.

- [ ] **Step 6: Commit the exhaustive ownership boundary**

```powershell
git add src/content/events/owner_policy.rs src/content/events/owner_policy/coverage_tests.rs src/runtime/branch/owner_audit/event_owner_bridge.rs src/runtime/branch/owner_audit/boundary_router.rs src/runtime/branch/owner_audit/run_choice_owner.rs src/state/events/mod.rs src/content/events/sssserpent.rs src/content/events/sensory_stone.rs src/content/events/woman_in_blue.rs src/ai/event_policy_v1/tests.rs
git commit -m "refactor: make event owner coverage exhaustive"
```

---

### Task 5: Final verification and maintained-design status

**Files:**
- Modify: `docs/superpowers/specs/2026-07-10-event-owner-coverage-design.md`
- Verify: all files changed by Tasks 1-4

**Interfaces:**
- Consumes: exhaustive selector API and focused owner contract suite.
- Produces: one verified implementation with no seed-panel acceptance dependency.

- [ ] **Step 1: Format and run focused verification**

```powershell
cargo fmt --all
cargo fmt --check
cargo test --lib content::events::owner_policy
cargo test --lib runtime::branch::owner_audit
```

Expected: all commands exit 0.

- [ ] **Step 2: Run the full library suite once**

```powershell
cargo test --lib
```

Expected: all library tests PASS. Do not substitute a panel or fixed-seed run for this command, and do not add one afterward.

- [ ] **Step 3: Check the final diff and retired symbols**

```powershell
git diff --check
rg -n "EventOwnerPolicyKind|MissingMarkedPolicy|AmbiguousMarkedPolicy|EventOwnerAction" src
git status --short
```

Expected: `git diff --check` exits 0; the retired-symbol search returns no matches; status contains only intended event-owner and documentation changes.

- [ ] **Step 4: Mark the approved design implemented and verified**

Replace the design status with:

```markdown
## Status

Implemented and verified on 2026-07-10. Event-owner completeness is enforced
by exhaustive `EventId` dispatch and seed-free typed owner contracts.
```

- [ ] **Step 5: Commit the verification record**

```powershell
git add docs/superpowers/specs/2026-07-10-event-owner-coverage-design.md
git commit -m "docs: record exhaustive event owner coverage"
```
