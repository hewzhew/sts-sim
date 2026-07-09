# Primary Search Recovery Contract V0 Design

## Status

Implementation review draft. This is the narrow first cut before any
feature-prior, NRPA-style adaptation, or ML work.

## Problem

The current project can often prove that a combat is solvable by running
review, rescue, or a heavier lane after primary fails. That is not enough.
Normal run advancement should depend on primary combat search, and primary must
explain its own attempt in structured data.

The immediate failure mode is:

```text
primary fails
  -> a hidden or side-path mechanism finds/executes a line
  -> the run appears to advance
  -> we cannot tell whether primary is reliable
```

V0 fixes the contract, not the search algorithm quality.

## Goal

Make one primary combat attempt readable, typed, and non-ambiguous.

```text
Combat boundary
  -> named primary profile
  -> primary attempt
  -> PrimarySearchOutcome
  -> runner advances only on an accepted primary line
```

After V0, review may still inspect a failed combat case, but review cannot
silently replace the primary attempt.

## Non-Goals

- No feature weight update.
- No NRPA-style online adaptation.
- No neural network or dataset export.
- No new boss playbook.
- No new rescue lane.
- No expansion of dirty-win semantic scoring beyond fields already cheap to
  obtain.
- No large human-readable report system.

## Contract Rule

Primary outcome is the only normal run-advancing combat result.

```text
Accepted primary line:
  runner may advance

No accepted primary line:
  runner records a combat gap or tool error
  review may be suggested as an offline diagnostic
```

Internal no-win rescue must not be mixed into the primary outcome unless the
profile explicitly names it as part of primary behavior. For V0, hidden rescue
is disabled for normal primary profiles.

## PrimarySearchOutcome V0

The first implementation can be a new type or a clearly named report section
inside the existing search report. The required schema is:

```text
PrimarySearchOutcome:
  profile: PrimaryProfileSummary
  status: PrimarySearchStatus
  accepted_line: Option<PrimaryLineSummary>
  best_complete_line: Option<PrimaryLineSummary>
  best_partial_line: Option<PrimaryLineSummary>
  telemetry: PrimarySearchTelemetry
```

### PrimarySearchStatus

```text
AcceptedWin
AcceptedDirtyWin
NoAcceptedLine
BudgetExhausted
FrontierExhausted
SearchInternalError
```

V0 should map existing outcomes conservatively. If a search has no accepted
line and the reason is unclear, prefer `NoAcceptedLine` with telemetry over
inventing a precise diagnosis.

### PrimaryProfileSummary

```text
profile_id
stakes = hallway | elite | boss
max_nodes
wall_ms
potion_policy
max_potions_used
rollout_policy
child_rollout_policy
acceptance_policy
internal_no_win_rescue_enabled
```

This is required because hallway, elite, and boss budgets are not semantically
equivalent. A combat gap without its profile is not interpretable.

### PrimaryLineSummary

```text
line_len
final_player_hp
hp_delta
potions_used
first_action_label
first_action_kind
```

V0 should not add expensive or controversial game semantics here. If dirty-win
flags are already available, include them. If not, defer them.

### PrimarySearchTelemetry

Minimum fields:

```text
elapsed_ms
deadline_hit
expanded_nodes
terminal_wins
terminal_losses
complete_candidates
frontier_remaining

first_win_node
first_win_ms
first_accepted_node
first_accepted_ms

rollout_us
expansion_us
transition_us

legal_root_actions
ordered_root_actions
top_root_actions
selected_first_action
```

Fields may be `null` when not observed. Do not guess.

## Root Action Surface V0

The root action surface must be observable without enabling a separate
microscope tool.

Each top root action row should include:

```text
rank
label
kind
target_label
ordering_score_or_bucket
role_hint_if_already_available
```

V0 does not need full action-feature extraction. The goal is to see what
primary considered first and what first action the accepted or best line used.

## Runner Boundary

Runner behavior after V0:

```text
if primary.status is AcceptedWin or AcceptedDirtyWin:
  commit accepted_line
else:
  emit combat gap with PrimarySearchOutcome attached or referenced
```

Runner must not:

- run review automatically to find a line,
- run post-primary lanes that advance the run,
- reinterpret profile details to decide potion policy after the profile has
  been built,
- parse human summaries to classify primary results.

## Panel Boundary

Panel rows should expose primary outcome facts, not review conclusions.

Required panel/capsule projection:

```text
primary_profile_id
primary_stakes
primary_status
primary_elapsed_ms
primary_expanded_nodes
primary_terminal_wins
primary_complete_candidates
primary_first_win_ms
primary_first_accepted_ms
primary_rollout_us
primary_expansion_us
primary_potion_policy
primary_max_potions_used
primary_internal_no_win_rescue_enabled
```

The panel may include a mechanical `next_recommended_command` for review, but
that command is diagnostic-only.

## Review Boundary

Review remains useful, but it is explicitly offline.

Allowed:

```text
combat_case_review reads a combat case
combat_case_review compares larger budgets or potion profiles
combat_case_review reports whether a line exists under review conditions
```

Not allowed:

```text
normal runner executes review's line
panel treats review success as primary success
primary status is overwritten by review status
```

## Regression Cases

V0 should be validated against the cases that exposed the contract failure.

### Spike Slime L A1F14

Requirement:

```text
Without running review, primary telemetry can show whether the failure was
budget, node count, rollout cost, or no complete candidate under the profile.
```

If the reasonable hallway profile still fails, the row must be interpretable
from `PrimarySearchOutcome`.

### Hexaghost A1F16

Requirement:

```text
Boss primary shows profile id, boss budget, potion policy, max potions, and
accepted/no-accepted status.
```

It must be possible to distinguish:

```text
boss no-potion failure
boss potion-allowed failure
boss primary accepted line
```

without reading review output.

### Hidden Rescue Regression

Requirement:

```text
Normal primary outcome cannot have an implicit turn_pool_rescue source.
```

If a rescue mechanism is ever promoted, its profile id must say so.

## Implementation Boundary

The first code cut should do only these things:

1. Add or expose `PrimarySearchOutcome` V0.
2. Populate the minimum telemetry fields from existing counters where possible.
3. Attach primary profile summary to the outcome.
4. Project primary outcome into capsule/panel summaries.
5. Ensure normal primary does not hide internal no-win rescue.
6. Add regression tests for profile visibility and hidden-rescue exclusion.

Do not implement action-feature weights in this cut.

## Testing

Unit tests should cover:

- boss primary profile summary includes boss budget and potion policy,
- elite primary profile summary includes elite/rescue budget and limited
  potion policy,
- hallway primary profile summary does not enable hidden no-win rescue,
- a failed primary attempt still emits telemetry and profile summary,
- panel/capsule projection uses primary outcome fields rather than review
  output.

Case-level validation should cover:

- Spike Slime L telemetry presence,
- Hexaghost boss profile visibility,
- no `turn_pool_rescue` source in normal primary outcome.

## Success Criteria

V0 is successful when:

- every normal combat attempt has a visible primary profile id,
- every primary attempt has a typed status,
- first-win and first-accepted fields are visible or explicitly null,
- rollout/expansion/transition timing is visible,
- panel rows can show primary failure without running review,
- runner advancement depends only on accepted primary lines,
- hidden rescue cannot be mistaken for primary success.

## Deferred Roadmap

The broader adaptation roadmap remains in
`2026-07-08-primary-search-recovery-adaptation-design.md`.

Deferred until after this V0:

- action feature vector extraction,
- accepted-line feature debug artifact,
- feature-prior action ordering,
- NRPA-style online adaptation,
- ML dataset export.
