# Return-Q Linear V0 Negative Baseline

## Observation

The linear return-Q selector is not accepted as a mainline combat policy.

The controlled V0 mixed dataset produced a strict offline failure after adding
pairwise ranking gates:

```text
full_state_plus_candidate pairwise = 0.7196
action_only pairwise              = 0.7725
candidate_only pairwise           = 0.7460
```

Closed-loop evaluation on the 95000 seed range also failed the policy gate:

```text
rule_baseline_v0          reward 21.53
plan_query_v0             reward 17.10
learned_q_direct          reward 15.62
learned_q_selective_1ply  reward 11.80
```

## Decision

Freeze the concat linear return-Q model as a negative baseline. Keep its scripts
and reports for regression comparison, but do not promote it to the training
mainline and do not use it to drive recursive search.

## Reason

A linear concat model has weak state-action interaction. In same-state candidate
ranking, pure state terms cancel, so the model can collapse toward global
candidate/action bias. This matches the observed ablation behavior.

## Next Gate

Before further model work, test whether the short-return target itself has
closed-loop decision value:

```text
short_return_oracle_controlled_H*
short_return_oracle_shielded_vs_rule_H*
```

If those oracle policies do not beat `rule_baseline_v0`, the target/horizon or
continuation protocol must be fixed before training another selector.
