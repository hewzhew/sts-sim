# Hexaghost Disarm Event Buckets v0.1

This document is the first concrete instance of
[Key Resource Event Bucket Template v0.1](/d:/rust/sts_simulator/docs/templates/KEY_RESOURCE_EVENT_BUCKET_TEMPLATE.md:1).

It stays inside the narrow scope:

- mitigation-type key resource only
- exactly 2 windows
- 2-layer labels
- only 5 primary metrics

## 1. Resource Role

- resource: `Disarm`
- role: `mitigation`

This document is **not** trying to define all of `Disarm`.
It is using `Disarm + Hexaghost` as a calibration case for:

- key-window reachability
- playability
- delayed mitigation value

## 2. Windows

### `window_1`

- first `Divider`

Why:

- it is the first major multihit attack window
- it is the earliest place where a mitigation resource might matter materially

This choice is fixed before reading the outcome data.
It is not chosen after the fact based on where `Disarm` happens to look best.

### `window_2`

- first `Tackle` after that first `Divider`

Why:

- it is the next major attack window after `window_1`
- it lets us test whether value is immediate or delayed

This choice is also fixed up front.

This v0.1 document does **not** elevate `Inferno` to an official template
window, even though it remains useful background context.

## 3. Layer A: Opportunity State

For this motif:

- `unavailable_before_window_1`
  means `Disarm` is not reachable before the first `Divider`
- `available_but_unplayable_before_window_1`
  means `Disarm` is reachable but not playable before the first `Divider`
- `available_and_playable_before_window_1`
  means `Disarm` is reachable and can be legally played before the first
  `Divider`

For v0.1, “playable” still means only:

- in hand before `window_1`
- enough energy and legal action opportunity to play it

It does **not** yet include “is it worth the opportunity cost”.

So in this document, `available_and_playable_before_window_1` means only:

- `Disarm` is in hand before the first `Divider`
- there is enough energy and a legal chance to play it

It does not yet mean that playing it is already strategy-optimal.

## 4. Layer B: Value Timing

For this motif:

- `immediate_by_window_1`
  means mean hp-loss already separates by the first `Divider`
- `delayed_to_window_2`
  means it does not separate by `window_1`, but does by the following `Tackle`
- `mostly_tail_risk_by_window_2`
  means means stay close, but bad-tail outcomes separate by `window_2`

## 5. What Existing Data Already Says

### Script-layer value is real

From the Rust-side `hexaghost_v1` audit:

- `avg_script_future_raw_damage_prevented_total = 38`
- `avg_script_future_multihit_damage_prevented_total = 32`

Source:

- [ironclad_hexaghost_disarm_v1_script_value_summary.json](/d:/rust/sts_simulator/tools/artifacts/learning_dataset/ironclad_hexaghost_disarm_v1_script_value_summary.json:1)

This is not a bucket label.
It is supporting evidence that the mitigation resource has genuine persistent
**enemy-script-layer** value.

Guardrail:

- script-layer prevented damage does not by itself prove that player-level
  value has already been realized
- it does not automatically imply lower hp-loss, lower catastrophe rate, lower
  block usage, or better immediate timing

### Early mean hp-loss did not separate in the toy deck

In that same `v1` toy-deck audit:

- `Disarm`, `Bash`, and `Defend` stayed almost identical on mean hp-loss through
  windows 1 and 2

Interpretation:

- for this motif, “mean hp-loss through window 1” is not enough by itself
- the likely value timing is **not immediate**
- it may be `delayed_to_window_2` or `mostly_tail_risk_by_window_2`
- for mitigation motifs, `mostly_tail_risk_by_window_2` should be treated as a
  normal candidate value form, not as a rare exception

### The original 5-card deck was only a diagnostic toy

That deck proved:

- persistent script value exists

But it was too small and too unrealistic to serve as the main RL task.

That is why the more realistic `hexaghost_v2` natural-start deck was added.

## 6. Current v0.1 Metrics

### Primary metrics

For this motif, v0.1 keeps only:

- `P(reachable_before_window_1)`
- `P(playable_when_reached_before_window_1)`
- `mean_hp_loss_to_window_1`
- `mean_hp_loss_to_window_2`
- `worst_20p_hp_loss_to_window_2`

Optional supporting metric:

- `catastrophe_rate_to_window_2`

Probability-space note:

- these probabilities and rates are local statistics over the explicitly chosen
  seed sets
- if the seed set changes, the conclusion may also change

### Not included yet

These are intentionally deferred:

- `block_budget_saved`
- `lethal_timing_delta`
- full late-script analysis as a primary template metric

## 7. Appendix: Current RL-Facing Probe

On the more realistic `hexaghost_v2` natural-start deck:

`baseline`:

- `avg_damage_taken = 42.375`
- `disarm_played_on_first_seen_turn_rate = 0.5`
- `disarm_played_on_opening_turn_rate = 0.25`

Source:

- [ironclad_hexaghost_disarm_v2_rl_metrics.json](/d:/rust/sts_simulator/tools/artifacts/learning_dataset/ironclad_hexaghost_disarm_v2_rl_metrics.json:1)

`script-credit`:

- `avg_damage_taken = 39.625`
- `disarm_played_on_first_seen_turn_rate = 0.375`
- `disarm_played_on_opening_turn_rate = 0.125`

Source:

- [ironclad_hexaghost_disarm_v2_script_credit_rl_metrics.json](/d:/rust/sts_simulator/tools/artifacts/learning_dataset/ironclad_hexaghost_disarm_v2_script_credit_rl_metrics.json:1)

Interpretation:

- the current script-aware reward can improve safety a bit
- but it does not automatically create earlier `Disarm` play timing
- this is consistent with the v0.1 template:
  the right question is not “did it play `Disarm` as soon as possible?”
  but rather:
  - was it reachable before `window_1`?
  - if reachable, was it playable?
  - did value show up by `window_1` or only later?

This section is an RL-facing probe appendix, not part of the core template
definition.

## 8. What Transfers Beyond Disarm

The reusable part is not:

- a special `Disarm` rule

The reusable part is:

- mitigation resource reachable before the first key window?
- reachable but not playable?
- playable, but value delayed?
- value visible in means or mainly in bad-tail outcomes?

This is the part that should later transfer to other mitigation resources.

## 9. Immediate Next Use

This v0.1 document is meant to support:

- event-bucket audits instead of seed-by-seed arguments
- separating:
  - `Disarm available` states
  - `Disarm unavailable` states
- and avoiding the bad habit of treating “first action on one seed” as the main
  object of study
