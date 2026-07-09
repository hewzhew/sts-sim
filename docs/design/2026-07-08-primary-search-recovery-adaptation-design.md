# Primary Search Recovery And Adaptation Design

## Status

Review draft. This document is a design proposal for making combat primary
search a trustworthy mainline again. It does not change code by itself.

This document is a roadmap, not the first implementation contract. The first
implementation cut is defined by
`2026-07-08-primary-search-recovery-contract-v0-design.md` and is intentionally
narrower: make primary attempts readable and contract-safe before adding
feature priors, NRPA-style adaptation, or ML export.

## Problem

Recent primary-only runs exposed a structural issue:

```text
The project has often proved that a run can be rescued,
but not that primary combat search is reliable.
```

The most useful symptom was the A1 Spike Slime L case. A short primary budget
failed to find a line in a fight that was not actually unwinnable. Larger
diagnostic/review budgets could find wins. That is not a deck conclusion. It is
a search-contract conclusion.

The older workflow made this hard to see because several mechanisms could hide
the failure:

- post-primary portfolio lanes,
- potion rescue lanes,
- internal no-win rescue,
- heavy review tools,
- repeated reruns from the beginning of the run.

Those tools are not inherently bad. The failure mode is that they turned into
the practical mainline while primary search stayed weak and under-instrumented.

## Current Evidence

The current worktree has already moved in the right direction:

- post-primary portfolio lanes are disabled in the normal plan,
- internal no-win rescue is opt-in rather than hidden in normal primary,
- boss and elite primary profiles are now stake-aware,
- `branch_panel` is the Rust panel entry and the old Python panel wrapper has
  been removed,
- case review remains useful, but should not define normal run behavior.

That still leaves the real search problem:

```text
How should primary search find the first acceptable line,
then improve line quality, without turning every failure into a side lane?
```

## Design Goal

Make primary combat search an anytime solver with explicit telemetry and a
small adaptation loop.

The target shape is:

```text
Combat boundary
  -> PrimarySearchProfile
  -> PrimarySearchAttempt
  -> staged search schedule
  -> stable PrimarySearchOutcome
  -> runner either advances or records a real combat gap
```

Review tools may inspect failed attempts. They must not be the ordinary way a
run gets through combat.

## Non-Goals

- No neural network in the first implementation.
- No generic failure root-cause classifier.
- No new boss-specific playbook as the main fix.
- No hidden rescue inside accepted primary behavior.
- No post-primary lane stack that silently advances the run.
- No large prose report layer.
- No attempt to solve reward/shop/deck strategy in this design.

## External Ideas Considered

These papers and systems inform the design. They are not dependencies.

- Browne et al., "A Survey of Monte Carlo Tree Search Methods":
  MCTS works as asymmetric anytime search, but practical strength comes from
  the selection, rollout, and domain-bias design, not from random simulation
  alone.
  https://repository.essex.ac.uk/4117/1/MCTS-Survey.pdf

- Cazenave, "Nested Monte-Carlo Search":
  keep and improve complete sequences rather than only classifying failures.
  This is close to "find a line, then bias future attempts around it".
  https://www.ijcai.org/Proceedings/09/Papers/083.pdf

- Rosin, "Nested Rollout Policy Adaptation":
  successful sequences can update action preferences online. This is the
  cleanest research analog for lightweight action-feature adaptation before
  training a model.
  https://www.ijcai.org/Proceedings/11/Papers/115.pdf

- Likhachev et al., "Anytime Repairing A*":
  first find a feasible solution, then reuse search effort to improve it under
  a continuing budget.
  https://www.cs.cmu.edu/~maxim/files/ara_nips03.pdf

- Silver et al., "Mastering Chess and Shogi by Self-Play with a General
  Reinforcement Learning Algorithm":
  strong search combines policy priors, value estimates, and tree search. The
  immediate lesson is interface shape, not immediate neural-network training.
  https://arxiv.org/abs/1712.01815

- Schrittwieser et al., "MuZero":
  planning can be paired with learned models, but this should come after the
  project has stable state/action data and outcome labels.
  https://arxiv.org/abs/1911.08265

- Ross et al., "DAgger":
  training data should come from states the current policy actually visits.
  That supports collecting primary-search failures and near-wins from real run
  capsules instead of only hand-built toy cases.
  https://arxiv.org/abs/1011.0686

## Core Principle

Primary search should own the normal combat decision.

```text
review = offline diagnosis
rescue = explicit experiment or manually promoted primary feature
primary = only normal run-advancing combat search
```

If a technique can advance the run, it must either:

1. become part of a named primary profile, or
2. remain diagnostic and not affect runner behavior.

There should be no third category of hidden helper that sometimes advances
combat but is not visible as primary behavior.

## Primary Search Contract

### Input

```text
PrimarySearchRequest:
  combat_state
  run_context
  stakes = hallway | elite | boss
  profile_id
  budget
  artifact_policy
```

### Profile

```text
PrimarySearchProfile:
  profile_id
  budget
  potion_policy
  max_potions_used
  objective_schedule
  frontier_policy
  action_prior_stack
  rollout_schedule
  acceptance_policy
  telemetry_level
```

The profile is the only place where hallway, elite, and boss behavior should
diverge. Runner code should not reinterpret combat type after building the
profile.

### Output

```text
PrimarySearchOutcome:
  status
  accepted_line
  best_complete_line
  best_partial_line
  telemetry
  artifact_refs
```

`status` should be an enum:

```text
AcceptedWin
AcceptedDirtyWin
NoAcceptedLine
BudgetExhausted
FrontierExhausted
SearchInternalError
```

The runner only advances combat on `AcceptedWin` or `AcceptedDirtyWin`.
Everything else becomes a combat gap or tool error. Review can inspect it, but
review cannot silently replace it.

## Telemetry Contract

Primary must report enough information to explain whether a failure is:

- too little budget,
- too few expanded nodes,
- expensive rollout,
- expensive step/expansion,
- bad action ordering,
- no complete line under current profile,
- accepted-line rejection due to quality/resource rules.

Minimum telemetry:

```text
budget:
  max_nodes
  wall_ms
  elapsed_ms
  deadline_hit

node_stats:
  expanded_nodes
  terminal_wins
  terminal_losses
  complete_candidates
  frontier_remaining

first_win:
  first_win_node
  first_win_ms
  first_accepted_node
  first_accepted_ms

timing:
  expansion_us
  rollout_us
  transition_us
  diagnostics_us

root_surface:
  legal_action_count
  ordered_action_count
  top_actions
  selected_first_action_for_best_line

outcome_quality:
  hp_delta
  potions_used
  dirty_win_flags
  curses_or_bad_cards_added
  key_resource_changes
```

This is deliberately structured data. It is not a human summary.

## Objective Schedule

Primary search should be staged, not replaced by separate rescue lanes.

### Stage 1: Feasibility

Goal:

```text
find any replayable win under the profile's legal policy
```

This stage optimizes for first accepted line. It should be fast and should
avoid heavy diagnostics.

### Stage 2: Quality Improvement

Goal:

```text
improve the accepted line while sharing the same attempt context
```

Quality means:

- higher final HP,
- fewer or no potions used,
- avoiding dirty outcomes such as curse pollution,
- preserving important run resources.

Stage 2 should not rerun from scratch if Stage 1 found a win. It should reuse
available trajectory/frontier evidence where practical.

### Stage 3: Adaptive Reprioritization

Goal:

```text
use the best complete/winning trajectories to bias remaining search
```

This is the lightweight NRPA-inspired layer. It is not a neural network. It is
a feature-weight update over action facts observed in successful or promising
lines.

## Adaptation Model V0

The first adaptation model should be transparent and small.

```text
ActionFeature:
  play_attack
  play_block
  play_draw
  play_key_setup
  play_scaling
  clear_minion
  target_low_hp_enemy
  block_lethal_or_high_incoming
  use_damage_potion
  use_block_or_defense_potion
  use_scaling_potion
  end_turn
  add_dirty_card_or_curse
  spend_high_value_resource
```

Each ordered action receives a feature vector:

```text
score = existing_order_score + dot(weights, action_features)
```

V0 update rule:

```text
When a line is accepted:
  increase weights for features that appear in the accepted line,
  with a small decay by turn depth.

When a line is a high-quality improvement:
  increase weights more strongly.

When a line is dirty or resource-expensive:
  do not punish globally yet; record evidence only.
```

Do not update from arbitrary losses in V0. Losses are too ambiguous and can
easily teach the wrong lesson.

## Why This Is Better Than Failure Classification

The project has repeatedly hit mixed failures:

- deck weakness,
- low HP from earlier decisions,
- action ordering,
- potion timing,
- boss phase pressure,
- rollout cost,
- insufficient budget.

Trying to classify every failed case into exactly one cause would produce a
large and brittle rule system. Adaptation avoids that trap. It asks a smaller
question:

```text
Given a line that worked or nearly worked, what action features should become
easier to revisit?
```

This does not remove the need for analysis, but it reduces the pressure to
write a complete taxonomy of failure.

## Rollout Schedule

Rollout must be owned by primary search, not scattered as a side effect.

V0 schedule:

```text
root:
  cheap root estimate only

early expansion:
  lazy rollout on pop for ordinary children

high-uncertainty nodes:
  spend rollout only when the node is near the frontier cutoff or when root
  action priors disagree strongly

after first accepted win:
  prioritize exact improvement before more rollout-heavy exploration
```

The important change is not one policy name. The important change is that
rollout is budgeted as part of the primary schedule and reported in telemetry.

## Budget Model

Budgets must be stake-aware and profile-owned.

```text
hallway primary:
  small wall budget, no hidden rescue, low potion access

elite primary:
  larger budget, limited semantic potion access

boss primary:
  boss budget, explicit potion policy, quality-aware accepted-line criteria
```

The profile may use different numbers over time. The invariant is that the
runner should not pretend that one `search_ms` value is semantically equivalent
for hallway, elite, and boss.

## Review Tool Role

Review tools should answer:

```text
What did primary fail to see?
Was the case solvable under a larger budget?
Which telemetry field suggests the bottleneck?
```

They should not answer:

```text
What line should the runner execute right now?
```

If review repeatedly discovers a useful technique, that technique must be
promoted into a named primary profile or an action-prior plugin. Until then it
remains offline evidence.

## Panel Role

Panels should measure primary behavior, not rescue behavior.

Panel rows should capture:

```text
seed
floor
combat subject
primary profile id
primary outcome status
first_win_ms
first_win_nodes
expanded_nodes
rollout_us
expansion_us
accepted_line_quality
```

Five-seed panels are not statistically meaningful. They are workflow tests and
regression smoke tests. The real value is that they expose whether primary is
getting stronger or merely being bypassed.

## Case Policy

Frozen cases are still useful, but only as regression samples.

They should not be treated as the source of truth for the whole project.
Because reward/shop changes alter future decks, an old case may become stale.
The correct response is not to delete all cases, but to label them:

```text
case_kind:
  primary_regression
  boss_pressure
  potion_timing
  dirty_win
  obsolete_snapshot
```

Only `primary_regression` cases should block primary-search changes.

## Implementation Phases

### Phase 1: Primary Outcome And Telemetry

Add stable primary attempt output before adding adaptation.

Deliverables:

- `PrimarySearchOutcome` or equivalent report section.
- first-win and accepted-win timing.
- rollout/expansion/transition timing.
- root action surface summary.
- explicit `accepted_line` vs `best_complete_line` distinction.

Success:

```text
The Spike Slime L case can be discussed from primary telemetry alone,
without running review first.
```

### Phase 2: Review Demotion

Make normal panel/run output primary-only by default.

Deliverables:

- review commands remain available,
- review output is not used to advance a run,
- panel summaries show primary outcome status,
- no hidden rescue inside primary unless the profile explicitly names it.

Success:

```text
When primary fails, the run reports a combat gap instead of silently rescuing.
```

### Phase 3: Feature-Vector Action Prior

Introduce a transparent feature-score layer for action ordering.

Deliverables:

- action facts -> small action feature vector,
- profile-owned prior weights,
- stable telemetry showing top action features,
- no pruning from the feature layer.

Success:

```text
Changing feature weights changes action order but never legality.
```

### Phase 4: NRPA-Like Online Adaptation

Use accepted lines to update feature weights within the same attempt or across
nearby retries.

Deliverables:

- line-to-feature extraction,
- small positive-only update from accepted lines,
- decay by turn depth,
- telemetry showing before/after top root actions.

Success:

```text
On cases with an early win, a second adaptive pass finds an equal or better
line with less search effort often enough to justify keeping the mechanism.
```

### Phase 5: Dataset Boundary For Later ML

Only after the above exists, export data for Python/ML.

Deliverables:

- state/action feature rows,
- outcome labels,
- source profile id,
- case/run provenance.

Success:

```text
Python can train or inspect priors without becoming the runtime control layer.
```

## Rejection Criteria

Stop and redesign if any of these happen:

- adaptation requires boss-specific card names to work,
- feature weights become pruning rules,
- review again becomes the normal path to advance combat,
- panel success improves only because more budget is hidden in a side path,
- telemetry cannot explain why primary found or missed its first win,
- implementation needs large prose reports to understand normal operation.

## Success Criteria

This design is working when:

- primary-only panels no longer stop at obviously solvable early hallway fights
  under reasonable hallway budgets,
- boss/elite/hallway budgets are visibly profile-owned,
- review is used after a combat gap, not before every conclusion,
- first-win time and node count are visible for every combat attempt,
- accepted-line quality can improve without rerunning from Neow,
- successful lines feed future action ordering through transparent feature
  weights,
- no run advancement depends on a hidden rescue path.

## Open Questions

These questions should be answered during implementation, not by adding more
prose policy:

1. Should adaptation reset per combat, per run slice, or per panel profile?
2. Which action features are stable enough to include in V0?
3. How much telemetry can be emitted without slowing primary itself?
4. Can the existing action facts cover draw/setup/potion semantics well enough
   for V0 feature vectors?
5. Which frozen cases remain valid primary regression cases after recent
   reward/shop changes?

## First Implementation Boundary

The first code cut should not implement adaptation.

It should only add the primary telemetry contract and make the current primary
attempt readable without review. If that cut is not clean, adaptation will only
make the system harder to reason about.
