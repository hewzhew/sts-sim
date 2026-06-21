# Campaign Journal

`CampaignJournal` is the campaign-level decision event log. It exists because
branch campaign decisions need a stable source of truth that is separate from
compact reports, checkpoint snapshots, and human-readable inspection output.

## Purpose

The journal answers questions such as:

- what decision boundary was reached
- which candidates were available
- which candidates were kept, frozen, pruned, or applied
- which branch and checkpoint state the decision belonged to
- which later outcomes can be linked back to that decision

Reports and inspect commands should be views over the journal when possible.
They should not become independent places that reconstruct decision history.

## Current Scope

The current implementation records these decision event shapes produced by
branch campaign parent expansion:

```text
CampaignJournalV1
  events[]
    reward_candidate_set
      decision_id
      boundary_title
      frontier_key
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    shop_branch_candidate_set
      decision_id
      boundary_title
      frontier_key
      candidates[]
        command
        label
        semantic_class
        disposition: kept
    shop_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_frontier_count
      rollout_head_plan_id
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    campfire_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_option_count
      selected_plan_id
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    event_candidate_pool
      decision_id
      boundary_title
      frontier_key
      game_event_id
      candidate_count
      branch_option_count
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    boss_relic_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_option_count
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    route_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      selected_index
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    route_decision
      decision_id
      route_branch_id
      selected_index
      selected_candidate_id
      target
      move_kind
      safety
      command
      elite_prep_bp
      first_elite
```

`BranchCampaignReportV1` now carries `journal` as a top-level field. The older
`rounds[].decision_observations` field remains as a compatibility summary, but
new inspection should prefer `journal`. The `--inspect-journal` report view
prints journal events directly; `--inspect-decision-observations` remains a
reward-only compatibility view.

Decision-outcome dataset export also prefers `journal` when available. It uses
the journal `decision_id` as the sibling group identity and links an observed
branch outcome to a candidate when the branch command sequence starts with the
journal parent commands plus the candidate command. Older command-prefix
reconstruction remains as a fallback for reports without journal events.

The same export prints `DecisionCandidateCoverageV1`, a report-level diagnostic
for how many journal candidates have any observed descendant branch. This is a
coverage check for campaign scheduling and learning data readiness; it is not a
candidate value estimate.

Coverage gap continuation builds on that diagnostic. It can plan unobserved
journal candidates with `--plan-coverage-gap-continuation --inspect-report ...`,
then execute a bounded set with `--execute-coverage-gap-continuation --resume ...
--resume-checkpoint ...`. Execution creates temporary active branches from the
journal parent commands plus the missing candidate command and then uses the
normal branch campaign runner. This mechanism is for targeted data coverage, not
for saying that the missing candidate is strategically better.

For route/map candidates, continuation targets carry structured
`target_origin` provenance from the journal `MapDecisionPacketV1` when it is
available. That origin records the typed target room, route action, candidate
pool completeness, route projection coverage, visible path summary, and first
elite segment. Coverage-gap tooling should use this provenance to explain and
schedule missing route candidates; it should not parse `go N` commands or
display labels to recover map identity.

## Boundaries

`CampaignJournal` is not a strategy engine. It must not decide what to pick or
rank branches.

`CampaignJournal` is not a combat trace. Combat details should live in combat
capture or combat trace artifacts and be linked by reference if needed.

`CampaignJournal` is not a checkpoint. Checkpoints restore state; journal events
explain decision provenance.

## Intended Data Flow

```text
decision boundary
  -> candidate enumeration / evaluation
  -> CampaignJournal event
  -> branch/report summary view
  -> inspect view
  -> learning/outcome dataset
```

The important direction is one-way: reports and datasets are derived from
journal events, not the other way around.

## Migration Plan

1. Reward candidate sets are the first event source.
2. Shop branch frontier candidates were the second event source.
3. Full shop compiler candidate pools are now captured in
   `BranchExperimentReportV1.shop_plan_candidate_pools` and surfaced as
   `shop_candidate_pool` journal events.
4. Campfire compiler candidate pools are now captured in
   `BranchExperimentReportV1.campfire_plan_candidate_pools` and surfaced as
   `campfire_candidate_pool` journal events.
5. Event branch candidate pools are now captured in
   `BranchExperimentReportV1.event_candidate_pools` and surfaced as
   `event_candidate_pool` journal events.
6. Boss relic candidate pools are now captured in
   `BranchExperimentReportV1.boss_relic_candidate_pools` and surfaced as
   `boss_relic_candidate_pool` journal events.
7. Route planner candidate pools are now captured in
   `BranchExperimentReportV1.route_candidate_pools` and surfaced as
   `route_candidate_pool` journal events. New route pools carry a typed
   `MapDecisionPacketV1` with `RouteMoveCandidateV1` entries; legacy candidate
   labels and summaries are compatibility/display views only. Route packets
   include candidate-pool provenance (`legal_candidate_count`,
   `complete_legal_pool`, ordering) and per-candidate projection metadata
   (`path_budget`, `observed_path_count`, coverage). `possibly_truncated`
   coverage is conservative: it means the visible-map DFS reached its configured
   path budget, not that a route is good or bad.
8. Route planner selections remain surfaced as `route_decision` journal events
   for compatibility and selected-action evidence. New route decisions also
   carry `selected_index` and `selected_candidate_id` so the selected move can
   be linked back to the surrounding `route_candidate_pool` without parsing the
   display label or `go N` command.
9. Link milestone outcomes to prior `decision_id` values.
10. Gradually remove report-only decision attachments once views read from the
   journal directly.

## Design Rules

- Give every decision a stable `decision_id`.
- Give every candidate a stable `candidate_id`.
- Keep display labels separate from machine identity.
- Store public boundary identity and candidate structure at decision time.
- Prefer structured fields over parsing strings from rendered reports.
- For map choices, consume `MapDecisionPacketV1` / `RouteMoveCandidateV1`
  (`target`, `action`, `features`, `projection`, `needs`, `evaluation`) rather
  than `RoutePlannerCandidateSummaryV1` strings. Use route projection coverage
  to distinguish complete visible projections from budget-limited projections.
- Keep old report fields only as compatibility views, not as new sources of
  truth.
- Treat candidate `admission` as the structured scheduling trace. Legacy
  `disposition` (`kept`/`pruned`) is still serialized for compatibility, but
  new continuation and replay tooling should prefer `admission.status`,
  `reason_category`, `reason_code`, `source`, and `lane`. The legacy
  `admission.reason` string remains compatibility/debug text, not a field to
  parse for control flow.
- Interpret branch `commands` relative to the report/checkpoint
  `run_prelude`, not relative to a fresh process start. New reports record the
  replay root and prefix commands explicitly; continuation tools should consume
  that prelude instead of reconstructing Neow or CLI prefix state.

## Current Caveats

- `shop_branch_candidate_set` exists for reports generated during the first
  shop-journal migration. New reports should prefer `shop_candidate_pool`.
- Shop candidate pools are the compiler candidate pool, not a raw shop inventory
  dump. They include single-action plans, stop/leave plans, and portfolio plans
  that the compiler generated for the active compile mode.
- Campfire candidate pools are the campfire compiler candidate pool, including
  rest/smith/stop plans and deck-mutation-derived target metadata where
  available.
- Event candidate pools are generated event branch candidates after public event
  semantics and policy/deck-mutation annotations are attached, with final branch
  admission marked separately.
- Boss relic candidate pools are complete boss relic choice sets with projected
  run debt and policy class metadata; all options remain branch candidates.
- Route candidate pools are full route planner option sets emitted while
  expanding a parent branch. They are eligible for journal coverage diagnostics
  and targeted coverage-gap continuation.
- Route decisions are the selected route planner actions emitted while
  expanding a parent branch. They record selected target and safety evidence
  for compatibility; new continuation and learning paths should prefer
  `route_candidate_pool`.
- Decision-outcome samples only include candidates that have an observed
  descendant branch in the report. A candidate that was recorded in the journal
  but never continued by campaign scheduling is still visible in
  `--inspect-journal`, but it does not yet get a synthetic zero-observation
  outcome row.
- Candidate semantics still include legacy `semantic_class` strings from branch
  retention; these are provenance, not proof of strategic correctness.
- Decision-outcome dataset export now links observed branch outcomes back to
  journal decision ids. Milestone outcome events are not yet stored directly in
  the journal.
- Reports written before `run_prelude` still need compatibility fallback when
  running continuation tools. New reports/checkpoints should not infer replay
  prefix from CLI arguments.
- Existing `decision observations` output is still reward-compatible legacy
  terminology. Prefer `--inspect-journal` for new debugging.
