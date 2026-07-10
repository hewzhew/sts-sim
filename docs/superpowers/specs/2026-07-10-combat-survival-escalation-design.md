# Combat Survival Escalation Design

## Goal

Prevent the owner-audit mainline from accepting locally winning combat lines
that leave the run below a basic survival reserve, and give an elite combat one
bounded quality rescue after a time-limited primary gap.

## Evidence

The bounded mainline probe for seed `20260710001` reached Act 2 floor 22 and
stopped at Book of Stabbing with 17/85 HP. Its elite primary search hit the
3,000 ms deadline after expanding 2,225 of 200,000 allowed nodes and found no
terminal win. No internal no-win rescue or post-primary elite lane ran.

The same path had already accepted an Act 2 Byrds win that lost 61 HP, moving
from 80 HP to 19 HP. The owner-audit recipe explicitly selected unlimited HP
loss, so the dangerous win was treated as sufficient and did not enter the
outer search portfolio.

## Decision

Use a survival-aware acceptance gate plus one explicit elite quality rescue.
Do not globally increase search budgets and do not re-enable run-control's
internal no-win rescue.

### Survival Acceptance Gate

Every owner-audit combat search lane receives a finite HP-loss limit derived
from the combat's starting run state:

```text
reserve_hp = max(1, ceil(max_hp / 4))
max_hp_loss = max(0, current_hp - reserve_hp)
```

A complete win is immediately acceptable only when it leaves at least 25% of
maximum HP, rounded up. When current HP is already below that reserve, only a
zero-loss win satisfies the gate. This is intentionally a reliability floor,
not a claim that the accepted line is optimal.

Examples:

- 80/80 HP permits at most 60 HP loss, so the observed 61-loss Byrds line is
  rejected and escalated.
- 54/80 HP permits at most 34 HP loss, so the observed 23-loss Hexaghost line
  remains acceptable.
- 17/85 HP permits no further HP loss; a losing or chip-damage line cannot be
  mistaken for safe mainline progress.

The rule belongs in a focused owner-audit survival-policy module. The generic
run-control API continues to support explicit numeric and unlimited gates for
interactive callers.

### Elite Quality Rescue

The primary elite lane keeps its existing 200,000-node / 3,000-ms profile. If
that lane returns a combat gap, the outer owner-audit portfolio may run exactly
one elite quality rescue lane with these properties:

- at most 300,000 nodes and 5,000 ms, using the existing bounded non-boss
  quality budget calculation;
- immediate child rollout, adaptive enemy-mechanics rollout, and round-robin
  evaluation buckets;
- semantic potion use with at most one potion;
- the same survival acceptance gate as every other owner-audit lane;
- accepted complete lines only; no partial combat state is committed.

If the outer run deadline has already capped the combat budget, the existing
budget-gap path wins and the rescue does not start. Boss behavior is unchanged
in this change.

## Data Flow and Ownership

1. `combat_search_lane_options` reads the visible current/max HP and attaches
   the owner-audit survival gate to the lane options.
2. `run-control` applies the existing HP-loss acceptance and rejection logic.
3. `boundary_router` continues to classify an unsafe or absent complete win as
   a combat gap.
4. `combat_search_portfolio_plan` selects one elite quality rescue after the
   primary gap.
5. `combat_search_orchestrator` keeps its existing deadline check and commits
   only an accepted rescue result.

No layer gets a second source of combat truth: run-control owns candidate
execution and rejection, while owner-audit owns mainline survival policy and
lane orchestration.

## Testing

Add focused unit regressions that prove:

- the survival calculation rounds the 25% reserve up and produces the three
  limits shown above;
- owner-audit lane options no longer request unlimited HP loss;
- an elite primary gap plans exactly one elite quality rescue;
- the elite rescue profile uses the bounded quality budget, one semantic
  potion, and accepted-line-only commit behavior;
- hallway and boss portfolio plans retain their intended lane counts;
- wall-capped combat still stops before post-primary rescue.

Run the focused owner-audit tests first, then `cargo fmt --check`,
`cargo test --lib`, and `git diff --check`.

## Non-Goals

- Do not increase default primary, rescue, or boss budgets.
- Do not add seed panels, checkpoint continuation, or source-replay tests.
- Do not change event owners, route scoring, shop policy, or card acquisition.
- Do not add a boss rescue portfolio in this change.
- Do not remove other currently unused combat lane kinds while fixing this
  blocker.
