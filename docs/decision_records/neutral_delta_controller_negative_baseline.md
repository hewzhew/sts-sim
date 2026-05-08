# Neutral Delta Controller Negative Baseline

Date: 2026-05-08

## Observation

`NeutralCompressedPolicyRunner` was tested as a one-step / generic engine-effect dominance selector over neutral engine query evidence. A 20-seed shadow audit produced:

```text
selected_count = 0 after demotion
short_horizon_signal_count = 1323
signal_disagrees_with_behavior_count = 208
order_only_disagreement_count = 129
suffix_replay_summary_equal_count = 129
damage_delta_only disagreements = 141
isolated enemy-response hp-loss-worse = 141
aligned enemy-response hp-loss-worse = 3
damage_delta_only aligned hp-loss-worse = 2
trainable_disagreement_label_count = 0
non_none_action_label_count = 0
```

Before demotion, the same one-step signal path could be mistaken for a policy selector. The audit probes showed that many apparent action disagreements were only order artifacts, and that isolated first-action enemy-response comparison penalizes order-only cases incorrectly. The aligned enemy-response probe reduced the apparent HP-loss-worse count from 141 to 3 by replaying compatible same-turn suffixes before the enemy response.

## Decision

Reject neutral one-step / generic effect dominance as:

```text
policy
teacher
action label
takeover controller
```

Keep the neutral query substrate as diagnostics/search evidence:

```text
NeutralEngineQueryService
commutation_probe
reference suffix replay
isolated enemy-response probe
aligned enemy-response probe
public redaction checks
resource contamination checks
reason/relation audit summaries
```

`NeutralProbeEvaluator` must abstain at the decision layer. It may emit `short_horizon_signal_candidate_id`, but that id is diagnostic/search-allocation evidence only and must not be executed or trained as an action label.

## Reason

Generic immediate damage/resource deltas are not a valid value function. They collapse unlike domains such as:

```text
damage vs block
damage vs debuff
damage vs draw
damage vs exhaust cost
single action vs same-turn plan order
```

The probes are valuable because they exposed the failure mode. The selector is not valuable because it turns those short-horizon signals into an apparent action preference.

## Consequences

- `selected_action_id` remains `None` for neutral evaluation traces.
- `damage_delta_only` is `SearchSignalOnly`, not a tactical label.
- Isolated enemy-response probes are diagnostic only.
- Aligned enemy-response probes are required before judging order-sensitive disagreements.
- Typed comparability is a routing contract, not a value rule. It may emit `OrderEquivalent`, `TerminalComparable`, `SurvivalComparable`, `DiagnosticOnlyDamageDelta`, or `Incomparable*`, but `action_label` remains `none`.
- Future work should move from action-level delta comparison to plan-level probes, typed comparability, and certificate gates.
