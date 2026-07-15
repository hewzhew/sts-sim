# Cheap Survival-Access Shop Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when the user explicitly requests delegation. Complete the checkboxes in order and stop on unexplained failures.

**Goal:** Let the normal shop decision pipeline admit a cheap card when one purchase simultaneously supplies immediate survival and a still-missing access role, covering seed006's post-purge Shrug It Off without naming that card or promoting broad multi-step shop search.

**Architecture:** Extend the existing acquisition report with one derived evidence bit and one policy reason. The evidence is computed from existing price, reward-admission mechanics, and deck-role facts; the mainline pipeline consumes the resulting policy exactly as it consumes other acquisition decisions. The shop owner and diagnostic multi-step portfolio remain unchanged.

**Tech stack:** Rust, existing cargo test unit/integration suites, Git.

## Global constraints

- Work in the stable checkout at D:\rust\sts_simulator; do not create a worktree.
- Use test-driven development: add all focused regression tests first, observe RED, then change production code.
- Do not special-case ShrugItOff, seed006, a floor number, or an exact deck list in production code.
- Do not change shop ownership, bundle generation, route policy, combat search, potion policy, relic policy, or shop_policy_v1::portfolio behavior.
- Cheap continues to mean price <= 35; do not reclassify the card as premium.
- Require RewardAdmissionClass::ImmediateWork; burdened or speculative cards do not receive this exception.
- Keep tests about evidence and admission. Do not lock the full seed trajectory or require that every eligible card is always purchased.
- Do not create or update run artifacts for this focused policy change.

## Intended interface change

CardAcquisitionReport gains:

    pub cheap_survival_access_compression: bool,

AcquisitionPolicyReason gains:

    CheapSurvivalAccessCompression,

The new evidence consumes existing facts:

- acquisition source is Shop;
- opportunity cost is Cheap;
- reward admission class is ImmediateWork;
- reward admission provides a survival mechanic;
- the same admission provides either still-needed card draw or still-needed energy.

The evidence produces ContextTake before the final unsupported-shop rejection.

## Task 1: Add RED acquisition-policy regressions

**Files:**

- Modify: src/ai/strategy/acquisition.rs

- [ ] Add CardAcquisitionReport to the acquisition test module imports if it is not already in scope.
- [ ] Add a helper named seed006_post_purge_shop_report that constructs the small, exact diagnostic context but calls only public strategy functions.

Use a deck with four Strike, four Defend, Bash, and Berserk. Build its DeckPlanSnapshot with an Act 1 shop admission context and these strategic facts:

    RunStrategicFacts {
        entering_act: 2,
        starter_basic_count: 8,
        curse_count: 0,
        has_energy_relic: false,
        has_runic_pyramid: false,
    }

Assess candidates through AcquisitionContext::shop(plan, 43, price) so the test represents the state immediately after the 75-gold purge.

- [ ] Add cheap_survival_access_shop_admission_accepts_seed006_shrug.

Assert for ShrugItOff at price 25:

    assert!(report.cheap_survival_access_compression);
    let policy = evaluate_deck_construction_contract(&report);
    assert_eq!(policy.verdict, AcquisitionPolicyVerdict::ContextTake);
    assert_eq!(
        policy.reason,
        AcquisitionPolicyReason::CheapSurvivalAccessCompression
    );

- [ ] Add cheap_survival_access_shop_admission_requires_cheap_price.

Assess the same card at price 36. Assert the evidence bit is false and the result remains Reject with NoPolicySupport.

- [ ] Add cheap_survival_access_shop_admission_requires_both_roles.

Assess IronWave at price 25. Assert the evidence bit is false; survival alone is not enough.

- [ ] Run the focused RED command:

    cargo test --lib cheap_survival_access_shop_admission -- --nocapture

Expected RED: the new report field and policy reason do not exist yet. If it fails for an unrelated compile or fixture problem, fix the test setup before production code.

## Task 2: Add a RED decision-pipeline integration regression

**Files:**

- Modify: src/ai/strategy/decision_pipeline.rs

- [ ] Add cheap_survival_access_shop_admission_reaches_mainline_lane to the decision-pipeline tests.
- [ ] Reuse the same post-purge deck and Act 1 context, then call shop_card_in_context_with_price(..., CardId::ShrugItOff, 0, 25).
- [ ] Assert the decision is executable mainline policy rather than an inspection-only branch:

    assert_eq!(decision.inspect_only_reason(), None);
    assert_eq!(decision.lane, CandidateLane::Mainline);

- [ ] Rerun the focused RED command:

    cargo test --lib cheap_survival_access_shop_admission -- --nocapture

Expected RED remains the missing production interface. Keep the common substring in all four test names so one command covers the policy and integration layers.

## Task 3: Implement the smallest generic evidence rule

**Files:**

- Modify: src/ai/strategy/acquisition.rs
- Verify only: src/ai/shop_policy_v1/portfolio.rs

- [ ] Import RewardAdmissionClass beside RewardAdmission and RewardAdmissionReason.
- [ ] Add cheap_survival_access_compression to CardAcquisitionReport.
- [ ] Add CheapSurvivalAccessCompression to AcquisitionPolicyReason.
- [ ] Compute the evidence in assess_card_acquisition and include it in the report.
- [ ] Add a private helper with this shape:

    fn cheap_survival_access_compression(
        deck_plan: DeckPlanSnapshot,
        source: AcquisitionSource,
        opportunity_cost: AcquisitionOpportunityCost,
        admission: &RewardAdmission,
    ) -> bool {
        source == AcquisitionSource::Shop
            && opportunity_cost == AcquisitionOpportunityCost::Cheap
            && admission.class == RewardAdmissionClass::ImmediateWork
            && admission_survival_tool(admission)
            && ((admission_provides(admission, Mechanic::CardDraw)
                && deck_plan.roles.draw_units < 2)
                || (admission_provides(admission, Mechanic::Energy)
                    && deck_plan.roles.energy_units == 0))
    }

If the actual role-counter types require a harmless comparison adjustment, preserve the semantics above.

- [ ] In acquisition-policy selection, insert this arm after existing shop hard-gap admissions and before the final shop Reject/NoPolicySupport arm:

    AcquisitionSource::Shop if report.cheap_survival_access_compression => acquisition_policy(
        AcquisitionPolicyDecision::ContextTake,
        AcquisitionPolicyReason::CheapSurvivalAccessCompression,
    ),

- [ ] Give the reason this diagnostic label:

    cheap shop card compresses survival and still-needed access

- [ ] Do not edit shop_policy_v1::portfolio.rs; inspect its diff/status only to confirm the diagnostic multi-step surface stayed unchanged.

## Task 4: Prove GREEN at focused and neighboring layers

- [ ] Format the changed Rust code:

    cargo fmt

- [ ] Run the focused regressions:

    cargo test --lib cheap_survival_access_shop_admission -- --nocapture

- [ ] Run the complete acquisition test module:

    cargo test --lib ai::strategy::acquisition::tests -- --nocapture

- [ ] Run the complete decision-pipeline test module:

    cargo test --lib ai::strategy::decision_pipeline::tests -- --nocapture

- [ ] Verify the diagnostic portfolio still retains its existing multi-step plans:

    cargo test --lib ai::shop_policy_v1::tests::compiled_shop_portfolio_retains_multiple_multi_step_plans -- --exact --nocapture

- [ ] Inspect the diff. Confirm there is no card-ID branch in production and no change to the shop owner or portfolio:

    git diff -- src/ai/strategy/acquisition.rs src/ai/strategy/decision_pipeline.rs src/ai/shop_policy_v1/portfolio.rs
    git status --short

## Task 5: Run completion verification and commit

- [ ] Run the repository completion gates from fresh command output:

    cargo fmt -- --check
    cargo test --lib
    cargo test --test architecture_runtime_boundaries
    git diff --check
    git status --short

- [ ] If any unrelated failure appears, diagnose it and report it instead of weakening the test or claiming completion.
- [ ] Commit only the two source files after verification:

    git add src/ai/strategy/acquisition.rs src/ai/strategy/decision_pipeline.rs
    git commit -m "fix: admit cheap survival access shop cards"

- [ ] Confirm the checkout is clean:

    git status --short

## Done means

- The exact seed006 post-purge Shrug It Off example reaches the mainline policy lane.
- Price 36 is not admitted by the new rule.
- A cheap survival-only card is not admitted by the new rule.
- Production logic is generic and uses only existing strategic facts.
- Shop owner and multi-step portfolio behavior are unchanged.
- Focused, neighboring, full library, and architecture-boundary tests pass.
- The implementation is locally committed and the stable checkout is clean.
