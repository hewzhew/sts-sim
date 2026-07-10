# Campfire Owner Strategy Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make owner-audit Rest/Smith decisions consume the existing strategic campfire policy and correct the `Limit Break` upgrade semantic without freezing a temporary deck choice.

**Architecture:** `card_analysis_v1` remains the source of mechanical upgrade facts. `campfire_owner` asks `campfire_policy_v1` for an executable Rest/Smith choice, then retains its local Toke and relic-action fallbacks when the policy stops.

**Tech Stack:** Rust 2021, built-in test harness, Cargo.

## Global Constraints

- Do not add boss search lanes or increase combat budgets.
- Do not redesign Toke, Dig, Lift, or Recall strategy.
- Do not assert a particular upgrade for the Collector seed or another temporary deck snapshot.
- Do not retain the deck-only smith scorer as a second owner policy.
- Do not use subagents for this implementation.

---

### Task 1: Correct the Limit Break upgrade fact

**Files:**
- Modify: `src/ai/card_analysis_v1.rs`

**Interfaces:**
- Consumes: `card_analysis_profile_v1(CardId, u8) -> CardAnalysisProfileV1`.
- Produces: `CardAnalysisProfileV1::is_upgrade_exhaust_removed_delta == true` for `CardId::LimitBreak`.

- [ ] **Step 1: Write the failing semantic test**

Add a local test module that calls the public analysis API and asserts only the runtime-backed mechanical fact:

```rust
#[cfg(test)]
mod tests {
    use super::card_analysis_profile_v1;
    use crate::content::cards::CardId;

    #[test]
    fn limit_break_upgrade_records_exhaust_removal() {
        let profile = card_analysis_profile_v1(CardId::LimitBreak, 0);
        assert!(profile.is_upgrade_exhaust_removed_delta);
    }
}
```

- [ ] **Step 2: Verify the test fails for the missing fact**

Run: `cargo test --lib limit_break_upgrade_records_exhaust_removal`

Expected: FAIL because `is_upgrade_exhaust_removed_delta` is false.

- [ ] **Step 3: Add the minimal semantic correction**

Extend the existing matcher without changing any scores or policy:

```rust
fn is_upgrade_exhaust_removed_delta_v1(card: CardId) -> bool {
    matches!(card, CardId::Havoc | CardId::Armaments | CardId::LimitBreak)
}
```

- [ ] **Step 4: Verify the focused test passes**

Run: `cargo test --lib limit_break_upgrade_records_exhaust_removal`

Expected: one matching test passes.

- [ ] **Step 5: Commit the independently valid semantic fix**

Run:

```text
git add src/ai/card_analysis_v1.rs
git commit -m "fix: record Limit Break exhaust removal upgrade"
```

### Task 2: Route owner Rest/Smith through strategic campfire policy

**Files:**
- Modify: `src/runtime/branch/owner_audit/campfire_owner.rs`

**Interfaces:**
- Consumes: `build_campfire_decision_context_v1(&RunState, Vec<CampfireChoice>)` and `plan_campfire_decision_v1(&CampfireDecisionContextV1, &CampfirePolicyConfigV1)`.
- Produces: a private `strategic_rest_or_smith_choice(&RunState, &[CampfireChoice]) -> Option<CampfireChoice>` boundary used before local non-strategic fallbacks.

- [ ] **Step 1: Write stable owner-boundary tests**

Add tests beside the owner implementation. One compares the owner's visible Rest/Smith command to the action selected by `campfire_policy_v1`, without asserting a hard-coded card. The other constructs a full-HP Fusion Hammer + Shovel campfire and asserts that policy `Stop` falls through to visible `Dig` rather than becoming a gap or forced Rest.

```rust
#[cfg(test)]
mod tests {
    use super::campfire_owner_decision;
    use super::super::owner_model::{OwnerDecision, OwnerRoutine};
    use sts_simulator::ai::campfire_policy_v1::{
        build_campfire_decision_context_v1, plan_campfire_decision_v1,
        CampfirePolicyActionV1, CampfirePolicyConfigV1,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::{RelicId, RelicState};
    use sts_simulator::eval::run_control::{
        build_decision_surface, RunControlCommand, RunControlConfig,
        RunControlSession,
    };
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

    #[test]
    fn owner_rest_or_smith_choice_matches_strategic_policy() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session.run_state.current_hp = session.run_state.max_hp;
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::LimitBreak, 1),
            CombatCard::new(CardId::Inflame, 2),
            CombatCard::new(CardId::FiendFire, 3),
        ];
        let options = sts_simulator::engine::campfire_handler::get_available_options(
            &session.run_state,
        );
        let context = build_campfire_decision_context_v1(&session.run_state, options);
        let expected = match plan_campfire_decision_v1(
            &context,
            &CampfirePolicyConfigV1::default(),
        )
        .action
        {
            CampfirePolicyActionV1::Rest { .. } => CampfireChoice::Rest,
            CampfirePolicyActionV1::Smith { deck_index, .. } => {
                CampfireChoice::Smith(deck_index)
            }
            CampfirePolicyActionV1::Stop { reason } => {
                panic!("test requires an executable strategic action: {reason}")
            }
        };

        assert_eq!(owner_choice(&session), expected);
    }

    #[test]
    fn policy_stop_preserves_visible_owner_fallback() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session.run_state.current_hp = session.run_state.max_hp;
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::FusionHammer));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::Shovel));

        assert_eq!(owner_choice(&session), CampfireChoice::Dig);
    }

    fn owner_choice(session: &RunControlSession) -> CampfireChoice {
        let surface = build_decision_surface(session);
        match campfire_owner_decision(session, &surface) {
            OwnerDecision::Routine(OwnerRoutine::Command(RunControlCommand::Input(
                ClientInput::CampfireOption(choice),
            ))) => choice,
            _ => panic!("expected visible campfire input"),
        }
    }
}
```

- [ ] **Step 2: Verify the owner tests fail for the old boundary**

Run: `cargo test --lib campfire_owner::tests::`

Expected: the strategic-policy comparison or Dig fallback fails because the old owner uses deck-local smith/rest rules.

- [ ] **Step 3: Add the strategic Rest/Smith adapter**

Import the campfire policy API, remove the deck-only ranking import, and add:

```rust
fn strategic_rest_or_smith_choice(
    run_state: &RunState,
    options: &[CampfireChoice],
) -> Option<CampfireChoice> {
    let context = build_campfire_decision_context_v1(run_state, options.to_vec());
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());
    match decision.action {
        CampfirePolicyActionV1::Rest { .. } => Some(CampfireChoice::Rest),
        CampfirePolicyActionV1::Smith { deck_index, .. } => {
            Some(CampfireChoice::Smith(deck_index))
        }
        CampfirePolicyActionV1::Stop { .. } => None,
    }
}
```

Call this adapter first in `choose_campfire_owner_action`. On `None`, preserve Toke and the existing Dig/Lift/Recall/Rest fallback order. Delete the old `should_rest_before_smith` and `rank_campfire_upgrades` path.

- [ ] **Step 4: Verify the owner tests pass**

Run: `cargo test --lib campfire_owner::tests::`

Expected: both boundary tests pass.

- [ ] **Step 5: Run proportional verification**

Run:

```text
cargo fmt --check
cargo test --lib campfire_policy_v1
cargo test --lib upgrade_planner_v1
cargo test --lib
git diff --check
```

Expected: formatting succeeds, all library tests pass, and the diff has no whitespace errors.

- [ ] **Step 6: Commit the owner boundary fix**

Run:

```text
git add src/runtime/branch/owner_audit/campfire_owner.rs docs/superpowers/plans/2026-07-10-campfire-owner-strategy-boundary.md
git commit -m "fix: route campfire owner through strategic policy"
```
