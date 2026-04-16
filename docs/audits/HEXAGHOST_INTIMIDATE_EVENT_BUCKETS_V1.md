# Hexaghost Intimidate Event Buckets v0.1

This document is the second concrete instance of
[Key Resource Event Bucket Template v0.1](../templates/KEY_RESOURCE_EVENT_BUCKET_TEMPLATE.md).

It is intentionally parallel to the `Disarm` instance, but it tests a
different mitigation shape:

- `Disarm` = persistent mitigation tied to the future attack script
- `Intimidate` = short-lived mitigation tied to a near-term attack window

The point is not to build an `Intimidate` rulebook.
The point is to test whether the same v0.1 language can describe a mitigation
resource whose value is expected to be much more immediate.

## 1. Resource Role

- resource: `Intimidate`
- role: `mitigation`

Engine fact:

- `Intimidate` is a `0`-cost Ironclad skill
- it applies `Weak 1` to all enemies
- upgraded, it applies `Weak 2`

Source:

- [intimidate.rs](/d:/rust/sts_simulator/src/content/cards/ironclad/intimidate.rs:1)

## 2. Windows

### `window_1`

- first `Divider`

Why:

- it is the first major multihit attack window
- it is the earliest attack window where short-lived mitigation is likely to
  matter in a visible way

This choice is fixed before looking at outcome analysis.

### `window_2`

- first `Tackle` after that first `Divider`

Why:

- it is the next major attack window after `window_1`
- it helps test whether `Intimidate` behaves like an immediate-only mitigation
  tool or whether some value still carries into a later window

This choice is also fixed up front.

This v0.1 document does not elevate `Inferno` into the template window set.

## 3. Layer A: Opportunity State

For this motif:

- `unavailable_before_window_1`
  means `Intimidate` is not reachable before the first `Divider`
- `available_but_unplayable_before_window_1`
  means `Intimidate` is reachable but cannot be physically played before the
  first `Divider`
- `available_and_playable_before_window_1`
  means `Intimidate` is in hand before the first `Divider` and there is enough
  energy plus a legal action opportunity to play it

As in the template:

- this is only about physical playability
- it does not yet answer whether playing it is strategy-optimal

## 4. Layer B: Value Timing

For this motif, the expected timing hypotheses are different from `Disarm`.

Working hypotheses:

- `immediate_by_window_1`
  is the leading candidate label
- `delayed_to_window_2`
  is possible but should be treated as something to verify, not assume
- `mostly_tail_risk_by_window_2`
  is still allowed, but is not the default hypothesis for a short-lived weak
  application

Why this differs from `Disarm`:

- `Disarm` changes the enemy script’s effective damage over many future windows
- `Intimidate` is expected to concentrate more of its value in the first
  relevant attack window

## 5. Why This Is A Useful Second Mitigation Motif

`Disarm + Hexaghost` tested whether the template could describe:

- persistent mitigation
- delayed value
- script-layer value that may not show up in early mean hp-loss

`Intimidate + Hexaghost` is useful because it should test a different corner:

- short-horizon mitigation
- likely immediate value
- a resource whose value may be easier to see at `window_1`

If the same template cannot describe both motifs, that is useful information.

## 6. What We Do Not Yet Claim

Unlike the `Disarm` motif document, this file does **not** yet carry completed
audit results.

So this document does **not** currently claim:

- that `Intimidate` definitely separates mean hp-loss by `window_1`
- that `window_2` is irrelevant
- that `Intimidate` is generally better or worse than `Disarm`
- that weak timing is already fully characterized in the current harness

This is a motif-definition document first, not an outcome report.

## 7. Observed v0.1 Outcome

A first minimal event-bucket audit was run on:

- [start_spec.json](/d:/rust/sts_simulator/data/boss_validation/hexaghost_intimidate_v1/start_spec.json:1)
- fixed eval seed set: `2009..2016`

Summary source:

- [hexaghost_intimidate_v1_bucket_summary.json](/d:/rust/sts_simulator/tools/artifacts/learning_dataset/hexaghost_intimidate_v1_bucket_summary.json:1)

Observed Layer A outcome:

- `P(reachable_before_window_1) = 0.75`
- `P(playable_when_reached_before_window_1) = 0.75`
- opportunity-state counts:
  - `available_and_playable_before_window_1 = 6`
  - `unavailable_before_window_1 = 2`

Observed Layer B outcome:

- against the same-prefix `SharedPolicy` comparator, `Intimidate` already
  separates mean hp-loss by `window_1`
  - `Intimidate mean_hp_loss_to_window_1 = 14.0`
  - `SharedPolicy mean_hp_loss_to_window_1 = 20.0`
- by `window_2`, this local seed set no longer shows useful separation
  - both branches converge to `mean_hp_loss_to_window_2 = 45.0`
  - `worst_20p_hp_loss_to_window_2` also stays at `45.0`

Current local reading:

- for this motif and this seed set, the best-fitting Layer B label is
  `immediate_by_window_1`
- but that label should be read narrowly:
  - it only says the first useful separation appears by `window_1`
  - it does **not** imply that `Intimidate` remains dominant by `window_2`
  - it does **not** imply broad generality beyond this deck / encounter / seed
    set

Guardrail:

- `Bash` and `Defend` comparison rows exist in the summary, but their branch
  subsets are smaller / less aligned
- so the main v0.1 timing read should come from the `Intimidate` vs
  `SharedPolicy` comparison, not from over-reading the auxiliary comparator rows

## 8. v0.1 Metrics To Use

The same v0.1 metrics apply here:

- `P(reachable_before_window_1)`
- `P(playable_when_reached_before_window_1)`
- `mean_hp_loss_to_window_1`
- `mean_hp_loss_to_window_2`
- `worst_20p_hp_loss_to_window_2`

Optional supporting metric:

- `catastrophe_rate_to_window_2`

Not included yet:

- `block_budget_saved`
- `lethal_timing_delta`
- late-script kill-speed analysis

## 9. Initial Questions For The Audit

The first audit using this motif should answer only these questions:

1. Is `Intimidate` usually reachable before the first `Divider` in the chosen
   natural-start deck / seed set?
2. If reachable, is it physically playable before that `Divider`?
3. Does its value mostly show up by `window_1`, or is meaningful separation only
   visible by `window_2`?
4. Does `Intimidate` mostly improve mean hp-loss, or does it mainly improve the
   bad tail by `window_2`?

## 10. What Transfers Beyond Intimidate

The reusable part is not:

- a special rule for weak cards

The reusable part is:

- can the template describe both persistent mitigation and short-lived
  mitigation?
- does the same opportunity-state language still work?
- does the same value-timing language still work?

If it does, that is a stronger test of the template than staying inside only
the `Disarm` case.

## 11. Immediate Next Use

This document should be used as:

- the second mitigation motif under the same template
- a check that v0.1 is not secretly just a `Disarm`-shaped abstraction

The immediate next step is not RL work.
It is to run a small event-bucket audit and see whether the resulting labels
still make sense for a short-lived mitigation resource.
