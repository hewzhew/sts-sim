# Accepted High-Loss Combat Diagnostics Design

## Context

Bounded seed `20260711004` reached A2F22 Book of Stabbing at 20/74 HP. The
existing `combat_search_history.best_win` initially appeared to show consecutive
35 and 24 HP losses against Centurion + Mystic and Snake Plant. Comparing those
reports with the following reward boundaries showed that the Centurion combat
actually ended at 44 HP, not 24 HP: line repair improved the original search
candidate by about 20 HP before execution. Snake Plant still ended at 20 HP.

The current trace therefore conflates two different facts:

1. the best line in the original search report; and
2. the line that line selection and repair ultimately executed.

Combat gaps save a replayable `CombatCase`, but accepted high-loss wins do not.
Once the combat has been applied, the exact start position is gone. We can prove
that Snake Plant spent 24 HP, but cannot inspect its draws and actions to decide
whether the loss came from early search acceptance, Offering self-damage, Weak
timing, Malleable attack ordering, or an unavoidable draw.

## Considered Approaches

### Save every accepted combat

This gives complete coverage but recreates the artifact-volume and inspection
burden that the project has been reducing. Most zero- or low-loss combats do not
need a frozen replay case.

### Reuse `CombatCase.gap` for accepted wins

This makes `combat_case_review` immediately reusable, but an accepted victory is
not a gap. Encoding it as one would preserve the exact ambiguity that caused the
current misdiagnosis and would make the schema name disagree with its contents.

### Save an independent accepted-high-loss diagnostic artifact

This is the selected approach. Keep gap cases unchanged. When a committed combat
win crosses a high-loss trigger, save an exact `CombatCaptureV1` start position
and a typed evidence sidecar that distinguishes the original search candidate
from the selected executable line. The capture can be replayed by the existing
combat search driver without changing the generic `CombatCase` schema.

## Decision

Add a branch-local diagnostic record for committed high-loss combat wins. A
record is created when either the original best winning search line or the final
selected line loses at least 25% of maximum HP:

```text
hp_loss * 4 >= max_hp
```

Considering both values deliberately captures successful repairs as well as
unrepaired losses. For the current seed, the Centurion combat is retained because
its original line was severe, while the evidence also records that repair reduced
the actual loss. Snake Plant is retained because the selected line itself remains
severe.

The diagnostic contains:

- the exact stable combat start as `CombatCaptureV1`;
- act, floor, lane, enemies, start HP, and maximum HP;
- the configured hard HP-loss gate;
- the original search line summary;
- the final selected line summary after repair and clean-line selection;
- typed repair delta (`original_hp_loss - selected_hp_loss`);
- search nodes, terminal win count, and coverage status;
- the selected executable combat automation trajectory.

The accepted-line evidence must be emitted at the run-control line-selection
boundary, where both original and selected lines are still available. The owner
audit lane captures the pre-search combat position, accepts the evidence only
from a committed lane, and carries the resulting diagnostic through the branch.
The capsule artifact store writes paired capture and evidence files under an
accepted-high-loss diagnostics directory and lists them in the capsule result.

At most one record is retained per committed combat identity (act, floor, combat
turn, and enemy set). Rejected lanes and combat gaps do not create accepted-win
records; combat gaps continue using their existing artifact path.

## Evidence Semantics

The original search result and the selected executable result remain separate
typed fields. `combat_search_history` must not present the original line as if it
were the applied outcome. Existing search performance counters remain attached
to the original report, while the accepted-line annotation supplies the actual
selected outcome.

The trigger is diagnostic, not a policy judgment. Crossing it does not mean the
deck needs more Block, that the line is wrong, or that the combat must be
rejected. It only means the run spent enough HP to justify preserving evidence.

## Boundaries

- Do not change combat search ordering, budgets, acceptance gates, or repair
  behavior in this pass.
- Do not change reward, shop, route, campfire, or deck-deficit policy.
- Do not feed recent attrition into owners yet.
- Do not special-case Snake Plant, Centurion, Book of Stabbing, Offering,
  Uppercut, or seed `20260711004`.
- Do not save every successful combat.
- Do not reinterpret or migrate the existing `CombatCase.gap` schema.
- Do not treat diagnostic artifacts as teacher labels.

## Verification

Use test-driven development to prove:

1. selected-line evidence preserves different original and repaired HP losses;
2. the 25% trigger fires when either original or selected loss crosses it and
   remains off when both are below it;
3. only committed accepted wins produce diagnostic drafts;
4. capsule writing produces a replayable `CombatCaptureV1`, a typed evidence
   sidecar, and discoverable artifact references without changing gap cases;
5. selected-line history reports the applied outcome while original search
   performance remains available separately.

Run focused run-control and owner-audit tests, formatting, the full library
suite, and architecture-boundary tests. Then run seed `20260711004` once into a
fresh capsule. The run must emit inspectable Centurion and Snake Plant high-loss
diagnostics. Replay the Snake Plant capture with a quality search that does not
stop at the first merely survivable win and compare its best selected HP loss,
Offering usage, Weak timing, attack order, and terminal-win count with the
original line.

## Success Criteria

- A future investigation can reproduce the exact Snake Plant start without
  rerunning the preceding floors.
- Original, repaired, and actually executed HP-loss claims can no longer be
  confused.
- The diagnostic identifies whether a materially lower-loss Snake Plant line
  exists before any combat or deck policy is changed.
- Normal successful combats do not create persistent artifacts.
- Existing combat-gap artifacts and consumers remain compatible.
