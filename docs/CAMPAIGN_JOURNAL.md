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
2. Shop branch frontier candidates are the second event source.
3. Move full shop compiler candidate pools into journal events when the
   compiler exposes stable plan identities for all candidates at decision time.
4. Move campfire, event, route, and boss relic decisions after shop.
5. Link milestone outcomes to prior `decision_id` values.
6. Gradually remove report-only decision attachments once views read from the
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

- Shop journal events currently record the branch frontier options that were
  exposed to branch campaign. They are not yet the full shop inventory or full
  compiler candidate pool.
- Candidate semantics still include legacy `semantic_class` strings from branch
  retention; these are provenance, not proof of strategic correctness.
- Outcome links are not implemented yet.
- Existing `decision observations` output is still reward-compatible legacy
  terminology. Prefer `--inspect-journal` for new debugging.
