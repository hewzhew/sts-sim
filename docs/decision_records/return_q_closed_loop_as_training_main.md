# Return-Q Closed Loop as Training Main

## Decision

Use return-based `Q(state, candidate)` learning as the main combat-control
training route. Local plan-query labels, recursive rollout pairwise utilities,
and bounded objective labels remain diagnostics unless converted into actual
engine return targets and validated in closed-loop play.

## Rationale

Hand-written candidate ordering such as "full block before damage" is a hidden
bot. It can be useful for debugging but should not define training truth. The
model should learn from engine transitions and rewards:

```text
Q(s, a) ~= r + gamma * V(s')
```

Engineering should compress and evaluate candidates without writing value
preferences. Value must come from observed return and closed-loop performance.

## Initial V0

- Extend `full_run_env_driver` with clone-based `evaluate_candidates`.
- Collect `return_q_transition_v0` JSONL rows from real engine rewards.
- Train dependency-free linear Q ablations.
- Evaluate `learned_q_direct` and `learned_q_selective_1ply` in engine.

## Non-goals

- Do not promote `recursive_rollout_pairwise_labels` to main training labels.
- Do not tune `CanLethal` / `CanFullBlock` weights as policy truth.
- Do not claim success from offline accuracy without closed-loop improvement.
