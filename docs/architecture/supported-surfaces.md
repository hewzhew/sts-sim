# Supported Surfaces

This is the current map of maintained Rust and learning entry points. It is not
a freeze report or retirement history. Use git history for archaeology.

## Ownership Map

| Surface | Owner | Purpose |
| --- | --- | --- |
| Simulator core | `sts_simulator` | Game content, exact state transitions, legal actions, and stable domain facts. |
| Complete-turn planner | `sts_combat_planner` | Exact complete-turn generation plus local-turn-graph and policy-discrepancy witness kernels. |
| Combat evaluation | `sts_oracle_eval` | Combat evaluation, fixed-root exact-search orchestration, and `CombatCaseCoreV1`. |
| Run control | `sts_oracle_run_control` | Exact run sessions, non-combat decisions, combat application, and run evidence. |
| Learning environment | `sts_oracle_learning_env` | Exact single-episode environments and opaque combat-root artifacts. |
| Learning adapter | `sts_oracle_learning` | Model-facing views and batched pools; it owns neither policy objectives nor teacher semantics. |
| Branch runtime | `sts_oracle_runtime` | Analysis workspace, branch execution, panels, capsules, persistence, and resident services. |
| Atomic search frontend | `sts_combat_search_driver` | Lightweight CLI plus optimized worker for `AtomicExactV2`. |
| Exact combat contract CLI | `sts_combat_contract` | Typed fixed-root combat contract adapter. |
| Oracle laboratory | `sts_oracle_lab` | Current `ol.cmd` contract, artifact, case, workspace, and diagnostic commands. |
| Legacy command adapters | `sts_oracle_tools` | Thin retained binaries such as `branch_tiny`, `branch_panel`, and `combat_case_review`. |
| Python learning bridge | `bindings/python_learning` | Standalone PyO3/NumPy bridge over the typed learning pools. |

Cargo dependencies are the executable ownership boundary. Do not recreate a
source-scanning architecture test suite to freeze file names, module placement,
private delegation, or retired identifiers.

## Combat Search Identities

There is no version-number ranking of search engines:

- `AtomicExactV2` is the atomic-action fixed-root diagnostic and challenger.
- `TurnGraphPortfolioV1` is the resident witness portfolio over
  `LocalTurnGraphWitnessSession` and `PolicyDiscrepancySession`.

Both currently produce replayable witnesses and diagnostics. Neither is the
learned agent or an authoritative teacher. The canonical search details live in
[Combat Search Ownership](combat-search.md); the learned-agent direction lives
in [Public-Belief Agent Learning System](../design/2026-08-12-public-belief-agent-learning-system.md).

## Human Entry Points

| Command | Use |
| --- | --- |
| `ol.cmd contract ...` | Fresh typed exact-combat contract artifact. |
| `ol.cmd artifact ...` | Inspect or branch an existing current contract artifact. |
| `ol.cmd case ...` | Import and inspect current exact combat cases. |
| `cs.cmd ...` | Repeated fixed-root `AtomicExactV2` work without rebuilding the command host. |
| `branch_tiny` | One bounded owner-audit run or capsule continuation. |
| `branch_panel` | Small multi-seed capsule scheduling and inspection. |
| `combat_case_review` | Saved-case diagnostic review only. |
| `learning/dev.ps1` | Maintained Python learning test and verification entry point. |

Exact syntax and rebuild commands live in [the runbook](../RUNBOOK.md).

## Current Artifact Boundaries

- Routine exact-combat evidence uses the breaking V2 contract artifact. Older
  manifests are not promoted by compatibility inference.
- `CombatCase` accepts `combat_case_v1`; its witness budget has an explicit
  engine identity or `not_run`.
- Branch capsules use `branch_tiny_run_capsule_v5` and
  `branch_tiny_frontier_checkpoint_v3`. A capsule resume requires the V5
  manifest and V3 frontier to agree on run contract and source identity.
- Reports, traces, panels, witnesses, and best action sequences are evidence,
  not policy targets.

## Test Retention

Keep tests that protect game mechanics, exact transitions, public observation
and legality, search-kernel behavior, replayability, or an interface consumed
by a maintained command or learner. Delete migration guards when their
migration lands. Do not retain a test merely because it documents an old
architecture shape.
