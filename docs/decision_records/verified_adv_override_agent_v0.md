# Verified Advantage Override Agent V0

## Observation

`learned_adv_shielded` did not reliably reproduce the shielded oracle. It made
few overrides, but too many of those overrides had negative verified advantage.

The engine-verified shielded policy did improve the rule baseline:

```text
rule_baseline_v0:
  reward = 25.129
  combat wins = 6.03

verified_adv_override_agent_v0_H4:
  reward = 26.224
  combat wins = 6.30
```

Run configuration:

```text
seeds = 98100..98199
max_steps = 160
candidate_scope = controlled_v1
horizon_decisions = 4
margin = 0.5
continuation_policy = rule_baseline_v0
```

## Decision

Promote runtime verified improvement to the next control baseline:

```text
default = rule_baseline_v0
override only when engine evaluation verifies:
  return(candidate) > return(rule_action) + margin
```

The learned model is not allowed to directly select actions. A learned model may
later be used as a proposer, but the engine verifier remains the decision gate.

## Rationale

This keeps value judgment downstream of actual engine return. The agent does not
encode tactical preferences such as block-before-damage or specific card rules.
It only compares cloned environment returns under the same continuation policy.

## Current Validation

The 100-seed validation produced:

```text
verified decisions = 8955
verified overrides = 283
override rate = 3.16%
mean verified adv on overrides = 2.808
harmful verified overrides = 0
candidate evaluations = 40743
crashes = 0
```

Pending decision states are now in scope, but naturally reached pending states
remain rare:

```text
combat = 8926
combat_grid_select = 15
combat_hand_select = 14
```

## Next Gate

Do not train another direct selector until counterfactual pending decision-state
sampling improves coverage for choices that rule rarely reaches, such as
Headbutt and similar card-selection effects.
