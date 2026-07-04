# Architecture

This file is the maintained architecture contract for current AI, runner, and
artifact work. It replaces the old set of narrow boundary notes.

## Guiding Rule

```text
unified typed representation
  -> explicit phases
  -> pluggable decision owners
  -> execution applies typed decisions without reinterpreting policy
```

Free-form strings are display and provenance only. If a decision needs to be
continued, replayed, compared, or learned from, it needs typed identity first.

## AI Layers

New AI code must choose an owner layer before it is written:

- `domain`: stable game facts and vocabulary. No value judgments.
- `analysis`: profiles derived from public state. No scene choice.
- `strategy`: typed deck facts, package state, deficits, admission rules, and
  small shared evaluators used by policies.
- `policy`: thin scene adapters for reward, shop, campfire, event, route, Neow,
  boss relic, and run-choice decisions.
- `runtime`: branch execution, scheduling, journals, replay, capsules, budgets,
  and artifact writing.
- `legacy`: still-required old code that is not the design target.

The intended flow is:

```text
domain -> analysis -> strategy -> policy -> runtime
```

Do not add another scene-local strategic model when reward, shop, route, and
branch retention need the same concept. Shared concepts belong in `analysis` or
`strategy`; scene-specific button mapping belongs in `policy`; applying a
typed action belongs in `runtime`.

## Non-Combat Automation

Run-control automation reduces manual repetition. It may execute bounded
route, reward, shop, campfire, event, run-choice, and combat-handoff decisions.
It is not a teacher label and not proof that a policy is good.

Every automated non-trivial decision has this role:

```text
label_role = behavior_policy_not_teacher
```

Non-combat decision records must stay hidden-free:

- public observations are allowed,
- declared distributions and beliefs are allowed,
- privileged simulator futures are forbidden.

Automation should stop when the current site lacks a bounded policy answer. Do
not encode stale global rules such as "shops always stop" or "events always
stop"; each high-agency site needs its own owner/compiler boundary.

## Runner And Combat

The runner owns run progression:

- selecting or applying non-combat owner decisions,
- deciding when combat search is allowed,
- setting search budgets and potion policy,
- applying an exact returned combat line,
- saving run capsules, frontier checkpoints, and `CombatCase` artifacts.

Combat search owns only the in-combat problem:

- legal combat action enumeration,
- action ordering, rollout, and search policy,
- exact execution of candidate combat lines,
- combat outcome facts and diagnostics.

Combat search must not decide rewards, shops, events, campfires, routes, branch
retention, or deck-building causes. A combat result can expose a symptom; it is
not by itself a deck-construction verdict.

`CombatCase` is the preferred handoff from runner to combat investigation. If a
branch-tiny combat gap cannot be investigated from a saved case, fix the case
payload or the review entrypoint instead of creating another report format.

Potions are run resources. Combat may consider potion actions only when the
runner explicitly opens a potion policy and budget. A diagnostic fact such as
"potion rescue found a win" does not automatically mean the main runner should
spend that potion.

Accepted combat lines must be exact executable lines from the current combat
state. Frontiers, near misses, rollout samples, and dirty diagnostic lines are
evidence, not runnable campaign actions.

### Combat Search Orchestration

Combat search code should keep these phases separate:

```text
setup -> search runner -> scoring -> line selection -> repair/improvement
      -> execution -> trace/render/rejection
```

`combat_search.rs` is the command orchestrator. It should decide which phase to
call next, not contain search loops, render prose, or apply individual combat
actions.

Complete-line search is split by role:

- `combat_complete_line_search`: one bounded frontier search from one start
  state.
- `combat_complete_line_scoring`: lane classification and position scoring.
- `combat_complete_line_repair`: local improvement of an already found line.
- `combat_complete_line_solver`: base search plus optional repair, producing
  the selected complete-line outcome.

Changing how positions are scored should usually touch scoring. Changing how a
frontier expands should touch search. Changing how a found line is improved
should touch repair. The solver should stay a thin phase coordinator.

## Gap Semantics

Gaps are typed stops, not verdicts:

- `automation_gap`: a non-combat owner boundary has no bounded answer.
- `combat_gap`: current runner/search settings did not produce an acceptable
  executable combat line.
- `budget_gap`: configured wall-clock or slice budget ended.
- `potion_rescue`: diagnostic or retry path found a potion-assisted line.
- `still_no_win_after_review`: review settings still found no accepted line.

None of these proves why the run is bad. The next investigation step must be
explicit: search policy, potion gate, reward/shop choices, deck facts, or owner
coverage.

## Campaign Artifacts

Campaign artifacts are storage and replay surfaces, not strategy authority.
Keep these responsibilities separate:

```text
checkpoint  exact simulator state needed to resume execution
state       scheduler/workset state needed to continue a campaign
journal     append-only decision facts and candidate pools
report      bounded projection for inspection and tools
diagnostic  opt-in sidecar data for large explanations and traces
```

Checkpoint owns exact resume state. State owns scheduling data. Journal owns
decision facts and candidate identity. Report is a cheap projection.
Diagnostics are opt-in sidecars for large or narrow-use explanations.

Default reports should reference state, journal, checkpoint, and diagnostics
instead of inlining large payloads. Compression is not a license to store
unbounded data.

## Journal And Candidate Identity

Decision history belongs in the journal. It records:

- the decision boundary,
- branch and checkpoint context,
- available candidates,
- stable candidate ids and typed summaries,
- candidate admission and disposition,
- selected or applied candidates when a policy chose one.

Every decision needs a stable `decision_id`. Every candidate needs a stable
`candidate_id`. Display labels, command strings, and rendered summaries must
not be parsed for control flow.

Candidate admission is the structured scheduling trace:

- `admission.status`,
- `reason_category`,
- `reason_code`,
- `source`,
- `lane`.

Route, map, reward, shop, event, campfire, boss-relic, and run-choice
candidates should carry typed identity that can be continued without
recovering meaning from text.

## Report Field Admission

Reports, journals, summaries, and learning samples are interfaces. A quick
field can become an accidental policy surface.

Every new output field should be one of:

- `fact`: raw state or candidate data.
- `diagnostic`: intermediate view for debugging a model or scheduler.
- `verdict`: explicit conclusion with a named evaluator and evidence limits.
- `label`: training or evaluation target with a documented source.

If a field does not fit one of these classes, do not add it. Do not present
diagnostic extremes such as `furthest`, `best_hp`, or `cleanest` as winners
unless the evaluator really supports a winner claim.

Tests should protect stable structure, not prose. Avoid tests whose main
assertion is a human-facing adjective.

## Prohibited Crossings

- Do not use strings as decisions when a typed action, candidate key, or case
  field exists.
- Do not let combat review mutate runner policy.
- Do not let combat search choose non-combat owner actions.
- Do not let runner code inspect hidden futures except through explicit
  diagnostic experiments.
- Do not add another summary/report layer when a capsule `summary.json`,
  `CombatCase`, journal event, or existing review output can carry the fact.
- Do not preserve a duplicate module just because migration is uncomfortable.

## Change Rule

Any change that moves behavior across these boundaries must update this file in
the same commit. Small search heuristics, runner retry gates, potion policies,
owner bridges, artifact shapes, and report fields all count when they change
who owns a decision.
