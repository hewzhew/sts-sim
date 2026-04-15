# Power Through -> Second Wind Net Value Findings

This note records one narrow result:

> can the current `decision_audit` judge the immediate net value of
> `Power Through -> Second Wind`?

It is not a broad scorer review.
It is a route-decision memo.

## Question

The question was not:

- whether future burden is handled in general
- whether `Second Wind` is a good card
- whether the search stack is good enough

The question was:

> after `Power Through`, can the current audit distinguish
> "immediate garbage conversion is good"
> from
> "immediate conversion wrongly burns valuable non-attacks"?

## Cases

Three author specs were added:

- `data/combat_lab/specs/power_through_second_wind_net_value_pure_gain.json`
- `data/combat_lab/specs/power_through_second_wind_net_value_pure_loss.json`
- `data/combat_lab/specs/power_through_second_wind_net_value_mixed.json`

They were audited with `combat_author_audit`.

## Findings

### Pure Gain

`Power Through -> Second Wind` is ranked correctly.

What the current audit sees:

- immediate safety
- removal of junk / new `Wound`
- no persistent dead-draw burden after immediate cleanup

This means the current local terms are not blind to immediate conversion value.

### Pure Loss

The current audit does **not** properly penalize lines where `Second Wind`
collaterally burns high-value non-attacks such as `Barricade` and `Entrench`.

Observed pattern:

- `Second Wind first` still ranks near the top
- `Power Through -> Second Wind` is not meaningfully pushed down
- current breakdown fields do not express the opportunity cost of losing those cards

### Mixed

The mixed case shows the same blind spot in a noisier form.

The audit can still reason about:

- threat relief
- defense gap
- raw burden

But it does not explicitly represent:

- the collateral opportunity cost of immediate non-attack conversion

## Conclusion

Current `decision_audit` can represent:

- immediate threat / defense consequences
- immediate burden cleanup

Current `decision_audit` cannot represent:

- the opportunity cost of collateral exhaust when an immediate conversion action
  burns valuable non-attacks

## Decision

Do **not** extend `burden` further for this problem.

If a new dimension is added, the next candidate should be:

- `collateral_exhaust_cost_of_immediate_conversion`

Do **not** treat this result as justification for:

- touching the larger search architecture
- re-opening `Q` debates
- broadening into `Evolve` / `Fire Breathing` / generic exhaust-engine scoring

This memo closes one question:

> the current blind spot is not "burden relief";
> it is "collateral exhaust opportunity cost".
