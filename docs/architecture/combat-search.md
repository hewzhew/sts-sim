# Combat Search Ownership

This is the canonical identity and ownership map for current combat witness
engines. Detailed
acceptance, potion staging, checkpoint, and replay rules remain in
[`../ARCHITECTURE.md`](../ARCHITECTURE.md#runner-and-combat). Do not infer
authority from a version suffix, directory size, CLI name, or the word
"planner".

## There Is No Single "Highest" Search

The repository keeps two exact witness-engine families because they answer different
questions:

| Engine identity | Search unit | Configuration owner | Orchestration owner | Maintained role |
| --- | --- | --- | --- | --- |
| `AtomicExactV2` | One legal atomic combat action | `CombatSearchV2Config` and `AtomicCombatSearchOptionsV2` | Fixed-root driver and the legacy `branch_tiny` owner-audit session | Fixed-root diagnostic, challenger, benchmark, and compatibility owner-audit search |
| `TurnGraphPortfolioV1` | Exact complete-turn boundaries plus policy-discrepancy trajectories | `OracleCombatWitnessOptionsV1` | `OracleResidentCombatWitnessJobV1`, backed privately by `OracleRunCombatWitnessWorkV1` | Current resident oracle witness producer used by run exploration and analysis |

`AtomicExactV2` is not a lower-quality alias for the resident portfolio, and
the resident portfolio is not a profile of `combat_search_v2`. Results are
comparable only when they share the same exact root, legality/potion contract,
acceptance target, work accounting, and replay verification.

## Production Resident Flow

```text
Oracle run / analysis owner
  -> OracleCombatWitnessOptionsV1
  -> OracleResidentCombatWitnessJobV1
  -> private OracleRunCombatWitnessWorkV1 portfolio scheduler
       -> LocalTurnGraphWitnessSession
       -> PolicyDiscrepancySession
  -> typed portfolio evidence + replay-exact incumbent
  -> run-control acceptance
  -> atomic combat-resolution transaction
```

`LocalTurnGraphWitnessSession` lazily generates exact complete-turn options and
shares exact boundary states among its anchor, proposal, and guide services.
`PolicyDiscrepancySession` explores atomic trajectories by accumulated
departure from the same typed action policy. `OracleRunCombatWitnessWorkV1` decides
which member receives the next bounded service grant and retains the best
verified incumbent under the run-control contract.

The two member sessions live in `sts_combat_planner`. That crate owns the exact
complete-turn generator and solver kernels; it does not own campaign budgets,
potion-stage promotion, run continuation, or final application. Those belong
to run control.

## Atomic Exact V2 Flow

```text
fixed CombatPosition / CombatCase
  -> AtomicCombatSearchOptionsV2
  -> CombatSearchV2Config
  -> CombatSearchV2Session
  -> exact atomic-action report
  -> replay/adjudication adapter
  -> optional atomic combat-resolution transaction
```

The implementation lives under `src/ai/combat_search_v2`. Its rollout,
turn-plan seeding, frontier plugins, priority ablations, and report controls
are engine-private. The dependency-light `sts_combat_search_driver` frontend
and optimized worker expose this engine for fixed-root work. They do not invoke
the resident portfolio.

The `turn_planner/` directory inside `combat_search_v2` is an atomic-v2 macro
candidate generator. It is not `LocalTurnGraphWitnessSession` and does not
make atomic-v2 the production resident witness engine.

`branch_tiny` owner-audit still runs an atomic-v2 staged session. Its Rust
modules and V5 artifacts therefore use explicit `atomic_combat_search_*`
names. V5 intentionally breaks V4: old capsules and cases are not silently
loaded, renamed, or upgraded into current evidence. Re-import an exact root
and regenerate a fresh artifact under the current schema.

## Configuration Boundary

Only controls an engine reads may appear in its request type.

`OracleCombatWitnessOptionsV1` owns:

- complete-turn generation work;
- maximum engine steps per generated transition;
- wall allowance and satisfaction;
- potion admission, slot mask, and discard admission.

It must not accept or inherit atomic-v2 rollout, turn-plan, frontier, plugin,
priority-ablation, or node-budget fields. In particular, resident construction
does not inherit the older `RunControlSession.search_*` atomic tuning fields.

`AtomicCombatSearchOptionsV2` owns the fixed-root atomic adapter and may project
the full `CombatSearchV2Config` surface. Production resident code must not
import that type or the `combat_search_v2` implementation namespace.

Shared vocabulary lives in `src/ai/combat_witness_contract.rs`:

- `CombatWitnessEngineV1` gives durable engine identity;
- `CombatWitnessSatisfactionV1` names cross-owner acceptance requests;
- `CombatWitnessPotionPolicyV1` names potion action admission;
- the high-stakes potion default is owner policy shared by both engines, not
  an atomic-v2 capability.

The materialized persistent-payoff compatibility score lives with
`CombatPersistentOutcomeV1`. It preserves already-realized max HP, recovered
gold, and card growth comparisons. It is explicitly not a prediction of
future run value.

## Evidence Boundary

New action trajectories must identify their producer as `AtomicExactV2` or
`TurnGraphPortfolioV1`. `SearchCombat` is read-only legacy provenance and new
code must not emit it.

Engine identity does not make a trajectory a teacher label. A search report,
frontier sample, rollout estimate, stage trace, or accepted action sequence is
evidence with its own root and budget contract. Learning code may consume it
only through an explicitly designed target contract.

No current engine implements or passes a certified improvement operator. The
maintained specification for the missing public-information, chance-particle,
fair-root-allocation, independent-evaluation, and qualification boundary is
[`CombatSearchImprovementContractV1`](../design/2026-08-11-combat-search-improvement-contract-v1.md).
Until that contract is implemented and qualified, best witnesses remain
debug/replay provenance and must not be projected into teacher labels.

## Where To Start

- Resident oracle scheduling or potion stages: `oracle_combat_witness_work.rs`,
  `oracle_run_explorer.rs`, and `OracleCombatWitnessOptionsV1`.
- Complete-turn generation or local/discrepancy kernels:
  `crates/sts_combat_planner`.
- Fixed-root atomic search behavior: `src/ai/combat_search_v2`.
- Fixed-root CLI performance: `crates/sts_combat_search_driver` and `cs.cmd`.
- Result application and rejection: the engine-named run-control adapter, then
  the shared combat-resolution transaction.
- Policy-improvement teacher work: start with
  `CombatSearchImprovementContractV1`; do not add teacher semantics to either
  witness engine.

When a new control appears, first name which row in the engine table consumes
it. If neither engine reads it, it is not a supported search option.
