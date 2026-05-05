# Single-Step Q Decision Experiment

This document defines one narrow experiment:

> Should `Q(s, a)` remain on the table as a combat-local interface, or should we stop
> pretending a root-only one-step scorer can carry short-line tactical reasoning?

This is not a fixture collection exercise.
It is a route-decision experiment.

## The Question

We want to separate three different claims:

1. `immediate step score` is enough
2. `action-conditioned short-horizon value` is enough as a root interface
3. `full short-line evaluation` is required before any root grouping makes sense

The experiment is successful only if the result rules out at least one of these.

## What This Experiment Is Not

- It is not a training run
- It is not a search integration test
- It is not a broad oracle validation sweep
- It does not answer whether learned evaluation should be injected into the current tree

It answers only:

> does a deterministic sequencing case already kill myopic one-step scoring, and does
> root grouping by first action still preserve the correct local preference?

## Existing Spec To Use

Start with the existing author spec:

- `data/combat_lab/specs/flex_before_strike_cultist_light_pressure_turn2.json`

Why this one:

- no hidden intent
- no immediate draw uncertainty needed to understand the decision
- the tactical motif is obvious: `Flex` should come before attacks
- it is small enough that a short-line report is readable

## Tooling

This repo now includes a direct author-spec audit entrypoint:

```powershell
cargo run --bin combat_author_audit -- `
  --author-spec data/combat_lab/specs/flex_before_strike_cultist_light_pressure_turn2.json `
  --decision-depth 4 `
  --top-k 3 `
  --branch-cap 6 `
  --json-out tools/artifacts/decision_experiments/flex_before_strike_audit.json
```

The command prints a grouped short-line report and optionally writes the JSON artifact.

## What To Inspect

Read the report in two passes.

### Pass 1: Kill the myopic step-score idea

Check whether the top trajectory under `Play ... Flex` clearly beats the top trajectory
under attack-first openings.

If yes, then a scorer that mainly sees:

- immediate damage
- immediate block
- immediate visible pressure relief

is already dead for this class of sequencing problem.

That does **not** kill `Q(s, a)`.
It only kills shallow step-local scoring.

### Pass 2: Decide whether root grouping still survives

Now ignore the individual line details and ask:

> after grouping trajectories by first move, does the best first-move family still point
> to the right tactical opening?

If the grouped winner is still the `Flex` family, then root action interfaces are still
alive on this case.

If the grouped winner is unstable or wrong, then a root-only `Q(s, a)` interface is
already suspect even before training.

## Decision Table

### Outcome A

- `Flex` family wins clearly
- best `Flex` trajectory is tactically sensible
- attack-first families lose cleanly

Interpretation:

- abandon myopic step scoring
- keep `Q_local(s, a)` alive as a possible root interface
- do not assume it can replace within-turn replanning

### Outcome B

- only full lines make sense
- grouped first-move ranking is noisy, unstable, or wrong

Interpretation:

- do not make root-only `Q(s, a)` the main architecture bet
- move toward line-level local evaluation first

### Outcome C

- even the short-line report itself looks tactically wrong

Interpretation:

- stop talking about Q
- the local judge or horizon definition is not trustworthy enough yet

## Hard Rule For Interpreting This Experiment

Do not overclaim.

This experiment can justify:

- dropping shallow one-step scoring
- keeping or doubting root action-conditioned local value

This experiment cannot justify:

- injecting a learner into the current search
- claiming the teacher is broadly trustworthy
- claiming line-level evaluation is solved

## Immediate Follow-Up If Outcome A Happens

Run the same command on:

- `data/combat_lab/specs/power_through_not_on_cultist_low_pressure_turn2.json`

That second case tests resource preservation pressure instead of pure setup-before-payoff
ordering.

If both cases agree:

- shallow step-local scoring is not enough
- short-horizon local evaluation is worth continuing
- root `Q(s, a)` is still plausible, but only as one layer inside a sequence-aware system

## Immediate Follow-Up If Outcome B Happens

Do not collect more fixtures.

Instead, define the next experiment around:

- whether grouped-by-first-action reporting loses decisive information on purpose-built
  same-opening / different-line cases

That is the point where a true `Q_line(s, line)` question becomes live.
