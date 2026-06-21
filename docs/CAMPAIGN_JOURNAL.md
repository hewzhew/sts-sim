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

The current implementation records two decision event shapes produced by branch
campaign parent expansion:

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
```

`BranchCampaignReportV1` now carries `journal` as a top-level field. The older
`rounds[].decision_observations` field remains as a compatibility summary, but
new inspection should prefer `journal`. The `--inspect-journal` report view
prints journal events directly; `--inspect-decision-observations` remains a
reward-only compatibility view.

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
5. Move event, route, and boss relic decisions after campfire.
6. Link milestone outcomes to prior `decision_id` values.
7. Gradually remove report-only decision attachments once views read from the
   journal directly.

## Design Rules

- Give every decision a stable `decision_id`.
- Give every candidate a stable `candidate_id`.
- Keep display labels separate from machine identity.
- Store public boundary identity and candidate structure at decision time.
- Prefer structured fields over parsing strings from rendered reports.
- Keep old report fields only as compatibility views, not as new sources of
  truth.

## Current Caveats

- `shop_branch_candidate_set` exists for reports generated during the first
  shop-journal migration. New reports should prefer `shop_candidate_pool`.
- Shop candidate pools are the compiler candidate pool, not a raw shop inventory
  dump. They include single-action plans, stop/leave plans, and portfolio plans
  that the compiler generated for the active compile mode.
- Campfire candidate pools are the campfire compiler candidate pool, including
  rest/smith/stop plans and deck-mutation-derived target metadata where
  available.
- Candidate semantics still include legacy `semantic_class` strings from branch
  retention; these are provenance, not proof of strategic correctness.
- Outcome links are not implemented yet.
- Existing `decision observations` output is still reward-compatible legacy
  terminology. Prefer `--inspect-journal` for new debugging.
