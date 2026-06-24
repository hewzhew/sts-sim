# Campaign V2 Migration Plan

This plan implements
[Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md). It is not a
list of small cleanup chores. A migration step counts only if it transfers
semantic ownership to the right Rust component, deletes a misleading surface, or
enforces an artifact boundary.

## Migration Rule

Do not ask "what is the next small patch?"

Ask:

```text
Which wrong owner still controls a campaign concept?
```

Then move that concept to the correct owner and delete or quarantine the old
surface.

Changes that do not count:

- splitting one PowerShell file into several PowerShell files
- renaming active/frozen without changing the experiment model
- compressing JSON while keeping the same boundary error
- adding report fields because an inspect tool needs data
- adding tests for human wording or one uncertain strategic choice
- preserving an alias because it is convenient

## Target Ownership Matrix

| Concept | Owner | Current tolerated compatibility | Forbidden new work |
| --- | --- | --- | --- |
| source selector | `ArtifactStore` | wrapper forwards selector | wrapper resolves paths by convention |
| output allocation | `ArtifactStore` | wrapper asks Rust for refs | wrapper constructs run/scratch paths |
| latest pointers | `ArtifactStore` | old readers for archaeology | wrapper writes pointers |
| manifests | `ArtifactStore` | wrapper may display returned manifest | wrapper creates maintained manifest truth |
| continuation rounds | `CampaignEngine` | wrapper passes typed budget | wrapper interprets total vs additional rounds |
| milestone continuation | `CampaignEngine` | wrapper passes `--until` flags | wrapper loops and polls reports |
| candidate coverage | `ExperimentPlanner` + `CampaignJournal` | executor queues remain internal | active/frozen as public coverage model |
| route/shop/event/reward candidates | `CampaignJournal` | legacy labels as display | labels or command strings as identity |
| inspect views | `InspectRenderer` | wrapper forwards view request | wrapper parses report JSON as workflow control |
| learning rows | `Exporter` | old scripts for archaeology | report/journal grows training-only fields |

## Phase 1: Rust Campaign Request Boundary

Goal: every maintained workflow has a direct Rust request.

Deliverables:

- `campaign run`
- `campaign continue`
- `campaign coverage plan`
- `campaign coverage execute`
- `campaign inspect`
- `campaign artifacts`
- `campaign export`

Acceptance:

```powershell
cargo check --bin branch_campaign_driver
branch_campaign_driver campaign run --random-seed --mode quick --dry-run
branch_campaign_driver campaign continue --from latest --rounds 1 --dry-run
branch_campaign_driver campaign coverage plan --from latest --budget key --dry-run
branch_campaign_driver campaign inspect --from latest --view summary --dry-run
branch_campaign_driver campaign artifacts prune --dry-run
```

The output must show typed request data. It must not require the PowerShell
wrapper to construct a command sequence.

Exit criteria:

- every maintained wrapper path is a forwarder to one Rust request
- wrapper help says it is compatibility launch code
- no new workflow can be added only in PowerShell

## Phase 2: ArtifactStore Ownership

Goal: artifact paths, pointers, manifests, command provenance, and size metadata
are all Rust-owned.

Deliverables:

- Rust resolves `latest`, `scratch-latest`, `run:<id>`, `scratch:<id>`, and
  explicit `path:<path>`
- Rust allocates `run` and `scratch` outputs
- Rust writes manifests, command provenance, and latest pointers
- Rust list/show/prune commands cover maintained artifact lifecycle

Acceptance:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
branch_campaign_driver campaign artifacts show --from latest
branch_campaign_driver campaign artifacts prune --dry-run
rg -n --glob '*.ps1' "latest\\.campaign|latest\\.checkpoint|Write-CampaignWrapperManifest|New-CampaignOutputArtifactViaDriver" tools
```

Remaining PowerShell hits must be compatibility forwarders or explicit legacy
archaeology.

Exit criteria:

- a maintained writing workflow never writes artifact metadata from PowerShell
- old loose root artifacts can be read only through explicit legacy paths

## Phase 3: Engine Continuation Ownership

Goal: continuation is one Rust engine concept.

Deliverables:

- `rounds` means additional rounds in all maintained paths
- `until` milestone execution is a Rust-owned engine loop
- run, continue, and coverage execute share the same budget semantics
- source progress is computed by Rust

Acceptance:

```powershell
branch_campaign_driver campaign continue --from latest --rounds 1 --dry-run
branch_campaign_driver campaign continue --from latest --until Act1Boss --dry-run
rg -n --glob '*.ps1' "MilestoneLoop|Invoke-CampaignUntilMilestone|while.*Milestone|rounds_completed" tools
```

Remaining hits must not implement a loop or read reports as control flow.

Exit criteria:

- no PowerShell command calls the driver repeatedly to implement milestone
  behavior
- no path treats `--rounds` as total rounds

## Phase 4: ExperimentPlanner Replaces Active/Frozen Coverage

Goal: candidate coverage comes from journaled decision candidates.

Deliverables:

- journal entries carry typed decision ids and candidate ids
- route, reward, shop, event, boss relic, and deck mutation candidate pools can
  be addressed by ids
- coverage plan produces `CoverageTarget`s
- coverage execute runs `ContinuationJob`s with target provenance
- outcome summaries classify candidate progress as:

```text
unobserved
target_only
continued
terminal
censored
combat_budget_blocked
invalid
superseded
```

Acceptance:

```powershell
branch_campaign_driver campaign coverage plan --from latest --budget key
branch_campaign_driver campaign coverage execute --from latest --until Act2Start --out scratch
branch_campaign_driver campaign inspect --from scratch-latest --view coverage
rg -n "active/frozen|active branch|frozen branch" README.md docs tools src
```

Remaining active/frozen hits must be internal executor notes or explicit
deprecation text.

Exit criteria:

- coverage planning does not select work by thawing frozen branches
- reports answer "which historical candidates have not been observed?"
- active/frozen cannot appear as winner-like output

## Phase 5: InspectRenderer Ownership

Goal: inspect is read-only, typed, and independent of wrapper state.

Deliverables:

- typed inspect views for summary, artifact, state, journal, coverage, lineage,
  decision, route, shop, combat, and final-boss
- each inspect view declares which artifacts it reads
- no inspect path mutates latest or allocates scratch
- no inspect path parses display labels as ids

Acceptance:

```powershell
branch_campaign_driver campaign inspect --from latest --view summary
branch_campaign_driver campaign inspect --from latest --view artifact
branch_campaign_driver campaign inspect --from latest --view coverage
rg -n --glob '*.ps1' "Inspect|Read-CampaignJsonArtifact|ConvertFrom-Json" tools
```

Remaining PowerShell JSON reads must be legacy archaeology, smoke tests, or
temporary forwarder display only.

Exit criteria:

- maintained inspect does not depend on `tools/campaign_artifact_summary.ps1`
- default inspect output is layered: status, refs, coverage, blockers, optional
  drilldowns

## Phase 6: Artifact Boundary Enforcement

Goal: report, checkpoint, state, journal, diagnostics, and export stop
collapsing into one payload.

Deliverables:

- checkpoint contains exact resume state only
- state contains scheduler/executor bookkeeping only
- journal contains decision facts and candidate pools only
- report contains bounded projections and refs
- diagnostics contain optional large explanations and traces
- exports contain learning/analysis rows explicitly
- manifests record refs, schema versions, encodings, and sizes

Acceptance:

```powershell
branch_campaign_driver campaign artifacts show --from latest
branch_campaign_driver campaign inspect --from latest --view artifact
```

The artifact view must show size and ownership by artifact kind without loading
the report as a database.

Exit criteria:

- default report remains bounded for long runs
- large evidence tables and combat traces are opt-in diagnostics
- learning exports never require adding training-only fields to report/journal

## Phase 7: Compatibility Retirement

Goal: remove the old public surface from normal use.

Deliverables:

- wrapper help only shows supported launch aliases
- old aliases either fail loudly or call the Rust request without changing
  semantics
- old artifact readers are only archaeology
- docs and README show the Rust campaign model first

Acceptance:

```powershell
rg -n "-More|FromScratchLatest|InspectScratchLatest|latest\\.campaign|latest\\.checkpoint|selected_plan|active/frozen" README.md docs tools src
```

Allowed hits:

- explicit deprecation notes
- legacy readers
- tests proving retired aliases fail
- internal executor implementation names that do not drive public coverage

Exit criteria:

- a new contributor can run, continue, inspect, and plan coverage without
  learning old wrapper semantics

## Deletion Policy

Delete or quarantine code when it meets any of these conditions:

- it implements campaign semantics in PowerShell
- it parses report JSON to decide workflow control
- it preserves retired aliases as normal behavior
- it exists only to satisfy tests that protect wording or old shortcuts
- it duplicates a Rust-owned artifact command
- it is a one-off inspect/probe path that cannot be described as a typed view

Do not preserve bad code because it once helped debug an old failure.

## Documentation Policy

Every campaign doc must answer one of these questions:

- What is the architecture?
- What is the CLI contract?
- What owns artifact fields?
- What does the journal store?
- How do I run the maintained workflow today?

If a doc only explains a retired workaround, delete it or move the relevant
warning into a maintained doc. Search results must not be full of abandoned
interfaces.

## Current Priority Order

1. Finish retiring PowerShell-owned campaign semantics.
2. Establish the Rust `campaign <command>` request boundary.
3. Move remaining wrapper artifact writes and inspect JSON reads behind Rust
   commands.
4. Make coverage planning and execution use journal candidate ids as the normal
   exploration interface.
5. Enforce artifact boundaries in writers.
6. Return to strategy quality only after lifecycle and experiment surfaces stop
   fighting each other.
