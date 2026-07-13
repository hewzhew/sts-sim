# Persistent Run Burden Cutpoint Probe Design

## Goal

Explain where retained dirty combat wins first acquire a persistent run burden and whether one
legal action at the last clean player boundary can immediately win, trigger the burden, or change
the burden-triggering enemy plan. The probe is diagnostic evidence only. It must not change search,
acceptance policy, combat execution, or the run.

The first target is the saved Writhing Mass case, but the run-control API observes newly added curse
cards and state transitions generically. The CLI must not contain Writhing Mass, Implant, move-id,
or `Parasite` policy rules.

## Chosen Approach

Add a second opt-in run-control probe to each `combat_case_review --adjudicate` ladder row. It uses
the winning trajectories already retained by that row. For each exactly replayable dirty line, it
finds the input whose application first adds a curse to the projected master deck, preserves the
session immediately before that input, and groups equivalent cutpoints with a diagnostic identity
that combines the existing exact combat-state hash with the persistent run snapshot.

At most 16 unique cutpoints are examined in report order. At each cutpoint, the probe enumerates
the currently legal inputs and applies each input once to a cloned session. It records mechanical
effects only:

- clean combat victory after the input;
- a new curse added by the input;
- a living enemy's planned move changed without adding a curse;
- no immediate burden, victory, or planned-move change;
- input application failed.

Changing a planned move is evidence of an available plan-changing affordance, not proof that the
whole combat now has a clean solution. Likewise, observing no one-action escape does not prove a
clean route is impossible; the necessary divergence may occur earlier.

Two alternatives are deferred:

- Extending search scoring with pending persistent burden now would be premature because the last
  actionable state and available avoidance channels have not yet been measured.
- Increasing retained-candidate capacity or search budgets would add cost even though the global
  best-win score already prefers an observed clean terminal over a dirty one.

## Interface and Output

Add a public run-control API:

```text
probe_combat_case_persistent_burden_cutpoints_v1(
    source_review,
    case,
    config,
    report,
) -> CombatCasePersistentBurdenCutpointProbeV1
```

Each ladder `SearchReview` receives an optional `persistent_burden_cutpoint_probe` field beside its
existing `candidate_adjudication_census`. Both are absent unless `--adjudicate` is enabled.

The typed probe reports:

- retained candidates inspected and dirty candidates with a located cutpoint;
- total unique cutpoints observed, cutpoints examined, and whether the 16-cutpoint cap was hit;
- replay or input-application failures without suppressing successful evidence;
- one summary per examined cutpoint: cutpoint-state hash, candidate frequency, original trigger
  step and action, compact player/enemy state, and one-action outcomes;
- aggregate counts for clean immediate wins, burden-triggering actions, plan-changing actions,
  neutral actions, and failed actions;
- a typed conclusion: `clean_terminal_win_available`, `burden_trigger_plan_change_available`,
  `no_one_action_escape_observed`, `no_dirty_candidate_cutpoint`, or
  `incomplete_due_to_probe_failures`.

Conclusion precedence is deterministic: a clean immediate win wins first; otherwise a clean
plan-changing action; otherwise failures make the result incomplete; otherwise the probe reports no
one-action escape. An unexamined cutpoint caused by the cap is disclosed but does not by itself turn
successfully examined evidence into a failure.

## Data Flow and Ownership

1. The existing bounded ladder searches once and returns its retained winning trajectories.
2. Run-control projects the combat case using the existing trusted projection owner.
3. For each unique retained trajectory, run-control replays inputs through a cloned
   `RunControlSession`. Before and after every input it compares newly added master-deck curse UUIDs.
4. The first input that adds a curse defines the cutpoint. Its diagnostic identity combines the
   exact combat-state hash with current HP/max HP, gold, master-deck card identity and growth values,
   relics, potions, and run RNG counters. Only fully equivalent combat and persistent contexts are
   grouped; the preserved cloned session is used for counterfactual application.
5. For the first 16 unique cutpoints, run-control obtains legal inputs from the current combat
   position, respects the reviewed search configuration's potion affordance, applies each once to a
   cloned session, and observes actual run and combat deltas.
6. The CLI only attaches the typed result to the matching ladder row. It does not inspect
   `meta_changes`, count curse cards, identify monster move ids, or execute another search.

The original candidate trigger input is included among the legal-action outcomes when still legal.
This makes the observed burden transition directly comparable with alternative actions from the
same state.

## Error Handling and Limits

- No retained dirty candidate or no located burden transition produces
  `no_dirty_candidate_cutpoint`.
- Candidate replay drift is recorded with candidate index, action count, and error.
- A legal input that fails in the cloned session is recorded for that input; other inputs and
  cutpoints continue.
- A case projection failure is a typed top-level failure and leaves the rest of the review intact.
- The probe never follows an alternative beyond one stable input and never calls
  `run_combat_search_v2`.
- The 16-cutpoint cap and any omitted unique-cutpoint count are serialized explicitly.

## Verification

- A library test proves the first newly gained curse input captures the preceding clean session,
  not the already-dirty state.
- A library test proves equivalent combat and persistent contexts group while different planned
  moves, master-deck growth, gold, or RNG states remain distinct.
- A Writhing Mass fixture proves a legal nonlethal attack can be reported as a clean planned-move
  change through the existing Reactive mechanic without monster-specific probe logic.
- A library test proves replay or input failures prevent a false
  `no_one_action_escape_observed` conclusion.
- CLI tests prove the new field is absent without `--adjudicate`, attaches only to its matching
  ladder row, and serializes the fixed cap and typed conclusion.
- Run the saved Writhing Mass case once with the existing fast/slow ladder and report the observed
  cutpoint families and one-action affordances without claiming a full clean solution.
- At completion, run the full library, `combat_case_review`, and architecture-boundary suites.

## Non-Goals

- No full-seed replay, new combat search, suffix replay, or multi-action counterfactual.
- No search score, frontier ordering, timed-threat model, candidate retention, or budget change.
- No automatic dirty-win acceptance or clean-only policy relaxation.
- No Writhing Mass special case in the CLI or run-control probe.
- No proof that a clean combat route exists or is impossible.
- No historical artifact rewrite or new standalone diagnostic binary.
