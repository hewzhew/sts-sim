# Boss Preference Representation Audit (2026-04-15)

This note records why the first two boss-preference learner attempts do not
generalize across packs yet.

It is a representation audit, not a scorer audit.

## Scope

Artifacts examined:

- v1 ledger: `tools/artifacts/boss_validation/full_ledger.jsonl`
- v1 dataset: `tools/artifacts/boss_validation/boss_preference_dataset.jsonl`
- v1 report: `tools/artifacts/boss_validation/boss_preference_baseline_report.json`
- v2 ledger: `tools/artifacts/boss_validation/full_ledger_v2.jsonl`
- v2 dataset: `tools/artifacts/boss_validation/boss_preference_dataset_v2.jsonl`
- v2 report: `tools/artifacts/boss_validation/boss_preference_baseline_report_v2.json`

Validation packs:

- `hexaghost_v1`
- `guardian_v1`

## Result Summary

Both v1 and v2 baselines:

- fit the tiny in-pack training set perfectly
- fail to transfer across held-out packs
- collapse to `close_enough` on held-out evaluation

Observed leave-one-pack-out accuracy:

- v1: `0.3333 / 0.3333`
- v2: `0.3333 / 0.3333`

This means the bottleneck is no longer obvious feature leakage alone.

## What V1 Was Doing

V1 input still leaned heavily on judge-side and pack-side signals:

- `boss__*`
- `pack__*`
- `pass_flag`
- `state_seed_mod_10`
- `a_score / b_score`
- `a_outcome_rank / b_outcome_rank`
- record-level and candidate-level rationale/tag counts

Interpretation:

- the learner could mostly imitate local pack patterns and existing judge scale
- it was not forced to learn transferable tactical structure

## What Changed In V2

Removed:

- `boss__*`
- `pack__*`
- `pass_flag`
- `state_seed_mod_10`
- `a_score / b_score`
- `a_outcome_rank / b_outcome_rank`
- tag-count and rationale-count style features

Added:

- state summary
  - player HP / block / energy
  - enemy total HP
  - visible incoming
  - hand size
  - playable card count
  - attack/skill/power counts in hand
- line summary
  - play count
  - total energy spend
  - attack/skill/power plays
  - enemy-targeted vs untargeted plays
  - end-turn count
- retained preference-style deltas
  - `delta_hp_loss`
  - `delta_final_monster_hp`
  - `delta_threat_relief`
  - `delta_defense_gap`
  - `delta_dead_draw_burden`
  - `delta_collateral_exhaust_cost`
  - `delta_steps`

Interpretation:

- v2 is much cleaner than v1
- but the held-out failure remained unchanged

## Why V2 Still Fails

### 1. The dataset is extremely small and symmetric

Current label counts:

- `prefer_a = 3`
- `prefer_b = 1`
- `close_enough = 2`

Additional structure problems:

- `guardian_g1` and `guardian_g2` are mirrored
- `close_enough` cases are near-threshold neutral points
- two packs provide very little shared directional evidence

Effect:

- a linear 3-way classifier sees too little cross-pack signal
- when uncertain on held-out packs, it falls back to the neutral class

### 2. The line summary is still too shallow

Current line summary answers:

- how many cards were played
- how much energy was spent
- whether the line skewed attack/skill/power
- whether actions were targeted

It still does **not** answer:

- did the line play a key pressure-relief card
- did the line do setup or payoff
- did the line spend an irreversible key resource
- did the line reduce danger before the relevant window

Effect:

- the learner can see coarse play statistics
- it still cannot see the tactical meaning of the line

### 3. The state summary is still not window-aware enough

Current state summary is a static snapshot.

It does not explicitly encode:

- what kind of danger window this is
- whether the danger is single-hit vs multi-hit pressure
- whether the state rewards immediate relief vs setup
- whether the hand contains a unique critical resource

Effect:

- the same `delta_threat_relief` can mean different things across bosses
- the learner lacks the context to interpret that delta consistently

### 4. `close_enough` is functioning as a default uncertainty sink

Held-out confusion shows:

- true `prefer_a` cases collapse to `close_enough`
- true `prefer_b` cases also collapse to `close_enough`
- true `close_enough` stays `close_enough`

Interpretation:

- the model is not learning a rich `close_enough` concept
- it is using `close_enough` as the safest default when directional evidence is weak

## Hard Conclusion

The current bottleneck is no longer:

- boss/pack leakage
- raw score imitation
- obviously dirty features

The bottleneck is now:

- too few samples
- too little shared supervision across packs
- line representation that is not yet tactical enough
- state representation that is not yet window-aware enough

In short:

`state summary + coarse line statistics + risk deltas` is still not enough to
express why one line is better than another across different bosses.

## What This Means

This is not evidence that the learner path is wrong.

It is evidence that:

- the validation chain works
- the data chain works
- the learning chain works
- but the current representation has not reached the tactical-causal level yet

That is a better failure mode than v1.

## Decision

Treat v2 as a successful representation audit, not as a failed learner attempt.

What v2 established:

- cleaning obvious leakage was necessary
- cleaning obvious leakage was not sufficient
- the next missing layer is tactical/window semantics, not more denoising
