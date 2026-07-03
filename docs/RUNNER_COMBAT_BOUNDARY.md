# Runner / Combat Boundary

This document defines the maintained contract between run-level automation and
combat search. It is a boundary rule, not a search strategy guide.

## Responsibility Split

### Runner

The runner owns run progression:

- selecting or applying non-combat owner decisions,
- deciding when a combat search attempt is allowed,
- setting search budgets and potion policy,
- applying a returned combat line to the run state,
- saving run capsules, frontier checkpoints, and `CombatCase` artifacts.

The runner must not reinterpret combat internals. If combat search reports an
unresolved combat, the runner may save a case, continue another branch, or stop
with a gap. It must not infer that the fight is unwinnable.

### Combat Search

Combat search owns only the in-combat problem:

- legal combat action enumeration,
- action ordering and rollout/search policy,
- exact execution of candidate combat lines,
- combat outcome facts and diagnostics.

Combat search does not decide card rewards, shops, events, campfires, route
choices, branch retention, or deck-building causes. A combat result can expose a
symptom, but it is not by itself a deck-construction verdict.

## Handoff Objects

`CombatCase` is the preferred handoff from runner to combat investigation. It
captures the exact combat state plus enough run context to reproduce and review
the combat without rerunning the campaign.

Use `combat_case_review` for saved combat gaps:

```powershell
cargo run --bin combat_case_review -- --case <case.json> --ladder
```

Do not revive old report/probe readers for branch-tiny combat gaps. If a combat
cannot be investigated from a `CombatCase`, fix the case payload or the review
entrypoint instead of creating another report format.

## Potion Boundary

Potions are run resources. Combat search may consider potion actions only when
the runner explicitly permits a potion policy and budget.

Diagnostic facts such as "potion rescue found a win" do not automatically mean
the main runner should spend that potion. The runner owns the resource gate; the
combat layer owns the line evidence once the gate is open.

## Applying Combat Lines

A combat line may be applied to a run only when it is an exact executable line
from the current combat state and the dry run still produces the accepted
outcome. `best frontier`, `near miss`, rollout samples, and dirty diagnostic
lines are evidence, not executable run actions.

Acceptance gates such as `max_hp_loss` are practical budget controls. They do
not prove optimality, and they must not be presented as exhaustive best-line
claims.

## Gap Semantics

Combat gaps are typed stops:

- `combat_gap`: current runner/search settings did not produce an acceptable
  executable line.
- `budget_gap`: the run stopped because the configured wall-clock or slice
  budget ended.
- `potion_rescue`: a diagnostic or retry path found a potion-assisted line.
- `still_no_win_after_review`: current review settings still found no accepted
  line.

None of these is a direct strategy verdict. A gap can motivate investigation of
search policy, potion gates, reward/shop choices, or deck facts, but that next
step must be explicit.

## Prohibited Crossings

- Do not use string labels as combat or runner decisions when a typed action,
  candidate key, or case field exists.
- Do not let `combat_case_review` mutate runner policy.
- Do not let combat search choose non-combat owner actions.
- Do not let runner code inspect hidden combat futures except through explicit
  diagnostic experiments.
- Do not add another summary/report layer when a `CombatCase`, capsule
  `summary.json`, or existing review output can carry the fact.

## Change Rule

Any change that moves behavior across this boundary must update this document in
the same commit. Small search heuristics, runner retry gates, potion policies,
and combat gap artifacts all count as boundary-affecting when they change who
owns a decision.
