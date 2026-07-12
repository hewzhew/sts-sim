# Cross-Turn Attack Retaliation Debt Design

## Goal

Explain and eventually improve search behavior when damaging an enemy creates optional, repeated player HP loss across several turns. The design must remain engine-derived and generic: no `EnemyId::Spiker` checks, no fixed Spiker weight, and no global "HP always beats progress" reorder.

## Evidence

The first attack-retaliation slice is mechanically correct:

- engine power hooks expose current retaliation and visible growth;
- explicit single-hit and multi-hit card actions report the expected player HP loss;
- action ordering and conservative rollout prefer the lower-retaliation action within the same semantic role;
- the full library and architecture-boundary suites pass.

The frozen A3F42 rerun nevertheless stayed structurally identical: final HP 43, first win at node 452, and the same four explicit attacks into the Spiker. New ordering diagnostics contain lethal Twin Strike candidates with 30 projected player HP loss, proving that the signal reaches action ordering.

The first failing boundary is therefore not power resolution. It is the transition from local action ordering to whole-search coverage:

- semantic ordering changes child generation order only;
- lethal role rank precedes reactive risk;
- after children enter the frontier, stable unresolved rollout values intentionally compare progress before survival and HP;
- state value compares fewer living enemies and enemy effort progress before survival margin and player HP.

Those rules prevent high-HP stalls in ordinary fights, so changing them globally would be broader than this problem.

## Considered Approaches

### 1. Generic retaliation-debt observation followed by coverage diversification (recommended)

Track retaliation attribution explicitly through diagnostics first. A later behavioral slice may preserve representative paths across both progress and optional retaliation-debt bands, rather than converting HP into enemy damage with a magic coefficient.

Benefits:

- separates optional self-inflicted retaliation from unavoidable monster damage;
- preserves the existing anti-stall ordering for ordinary combats;
- supports setup lines that trade immediate progress for fewer future retaliation events;
- can be introduced in shadow/diagnostic mode before affecting the frontier.

Cost: a path ledger requires threading already-computed action-effect facts into child construction without resolving every card a second time.

### 2. Move survival or player HP before enemy progress globally

This is small in code but affects every stable unresolved combat. It risks recreating the high-HP stalled-state problem that the current comparator explicitly avoids. Rejected for this slice.

### 3. Rely only on deeper rollout

Rollout already sees exact HP changes, but the frozen search still selected the same complete line. Increasing rollout depth or budget would be expensive and would not explain which safer path family was absent or suppressed. Rejected as the primary solution.

## Current Slice: Diagnostic Completion

This goal implements only the evidence needed before a behavioral frontier change.

### Action attribution

Extend public ordering action-effect samples with:

- `attack_retaliation_trigger_count_hint`;
- `attack_retaliation_player_hp_loss_hint`.

These values already exist in `CardPlayReactiveEffectDiagnostics`; the missing work is the final JSON report projection.

### Search-wide exposure summary

Extend ordering diagnostics with descriptive aggregates collected from observed actions:

- number of candidate actions with attack retaliation;
- total projected retaliation trigger count across those candidates;
- total projected player HP loss across those candidates;
- maximum projected player HP loss on one candidate.

These counters describe search exposure only. They do not score, prune, merge, or reorder actions.

### Schema policy

The search diagnostic schema is additive. Existing fields retain their meaning. Notes must state that the values are candidate observations, not unique paths and not realized HP loss.

## Deferred Path Ledger

A future behavioral slice may add `AttackRetaliationPathLedgerV1` to search nodes with cumulative projected trigger count and projected HP loss. It must consume the action effects already computed by semantic ordering; it must not resolve cards again during child construction.

That prerequisite likely requires replacing the current `OrderedActionChoice = IndexedActionChoice` alias with an ordered choice that carries the relevant effect attribution. The ledger remains path metadata and must not silently enter exact combat state identity.

Before the ledger affects coverage or frontier selection, diagnostics must demonstrate at least one of these conditions:

1. a lower-retaliation path reaches comparable enemy-progress and survival buckets but is systematically delayed;
2. a setup path reduces later retaliation events but is lost at a documented coverage boundary;
3. the frozen search contains materially different retaliation-debt paths that collapse under an existing bucket or representative cap.

If none is observed, the frontier remains unchanged.

## Future Behavioral Boundary

When evidence supports it, prefer coverage diversification over a scalar score:

- bucket cumulative optional retaliation debt into none, low, and high using structural thresholds derived from observed per-event loss, not an enemy-specific constant;
- retain representatives across progress bucket × survival bucket × retaliation-debt bucket;
- keep exact terminal wins authoritative and continue choosing complete wins by existing outcome evaluation;
- report how many candidates were preserved or displaced solely by the new coverage dimension.

This is not part of the current implementation slice.

## Testing

The diagnostic slice requires focused tests that prove:

- a Thorns single-hit action projects one trigger and its HP loss into ordering JSON;
- a multi-hit action projects multiple triggers;
- Sharp Hide retains total reactive HP loss but zero attack-retaliation attribution;
- search-wide counters aggregate retaliation samples without changing ordered choices;
- a no-retaliation combat leaves all new counters at zero.

Repository completion still requires formatting, the full library suite, and `architecture_runtime_boundaries`. A frozen A3F42 rerun is evidence only and must not become a permanent card-sequence test.

## Non-Goals

- no Spiker-specific policy;
- no minimum-HP hard rejection;
- no global progress/HP comparator reorder;
- no new frontier score or coefficient;
- no duplicate card resolution in child construction;
- no permanent regression test for one frozen trajectory.
