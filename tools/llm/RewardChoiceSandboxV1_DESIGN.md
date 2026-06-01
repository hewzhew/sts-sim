# RewardChoiceSandboxV1 Design Contract

This document is a pre-implementation contract. Do not implement a reward
choice sandbox until this contract has a saved `DecisionCaseV1` acceptance case.

## Role

`RewardChoiceSandboxV1` is an evidence tool, not a teacher, policy, or final
controller.

```json
{
  "decision_authority": "evidence_only",
  "not_final_action": true,
  "label_role": "not_a_label",
  "trainable_as_action_label": false,
  "policy_quality_claim": false
}
```

## Required input

The sandbox must start from a saved `DecisionCaseV1`.

Required case fields:

- `public_state_before`
- `legal_actions_all`
- `decision_candidates`
- `reward_card_choices`
- `map_route_context`
- `llm_response`
- `selected_action`
- `run_metadata.seed`

No saved case means no sandbox run.

## Counterfactual patch

Each candidate must be represented as an explicit patch:

```json
{
  "patch_role": "reward_candidate_counterfactual",
  "not_actual_run_state": true,
  "candidate": "Shrug It Off",
  "deck_patch": {
    "operation": "add_card",
    "card_id": "ShrugItOff",
    "upgrades": 0
  }
}
```

`Skip` is a candidate:

```json
{
  "candidate": "skip",
  "deck_patch": {
    "operation": "no_change"
  }
}
```

## Scenario suite

The sandbox must not mean "continue the run and see what happens." It must run
declared scenarios with explicit scope.

Initial allowed scenario families:

- `next_hallway_combat_sample`: near-horizon survival and damage taken.
- `early_elite_risk_sample`: elite exposure under fixed continuation policy.
- `act1_boss_prep_sample`: boss preparation indicators, not win-rate claims.
- `draw_consistency_micro_sample`: draw/playability and deck-cycle effects.

Each scenario must declare:

- what it measures
- what it does not measure
- why it is relevant to the saved decision case
- budget
- randomness source

## Continuation policy

Every sandbox result must declare the continuation policy:

```json
{
  "continuation_policy": "rule_baseline_v0",
  "policy_bias_warning": "results reflect this continuation policy and are not intrinsic card value"
}
```

No continuation policy means no result.

## Metrics

Sandbox output must be vector/distribution oriented:

- survival rate within scenario
- hp delta distribution
- damage taken distribution
- turn count distribution
- potion consumption
- deck-cycle / draw consistency metrics
- failure mode clusters
- variance / tail risk

It must not output a single winner unless the mode is explicitly named
`unsafe_debug_rank_for_human_inspection`, and that mode must not feed prompts.

## Inconclusive results

The sandbox must output `inconclusive` when:

- scenario count is too small
- candidates differ mostly through continuation-policy bias
- confidence intervals / variance overlap heavily
- required simulator hooks are missing
- horizon is too short for the proposed claim

## Budget

Every run must declare:

- max candidates
- max scenarios
- max branches per scenario
- max engine steps per branch
- max wall-clock seconds

Budget exhaustion must appear in output and must lower confidence.

## Truth warnings

All results must include:

- `counterfactual_not_actual_run`
- `continuation_policy_dependent`
- `not_teacher_label`
- `not_policy_quality_claim`
- `scenario_suite_limited`

## First acceptance case

The first implementation must be tested on a saved reward decision case, not on
a synthetic prompt. The acceptance target is:

- case loads by `case_id`
- all reward candidates plus `skip` get explicit patches
- no sandbox output claims a winner
- every metric names scenario and continuation policy
- output can be inspected by `tools/llm/case_tool.py show`

