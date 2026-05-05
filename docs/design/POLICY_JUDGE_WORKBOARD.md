# Policy / Judge Workboard

This document is the working boundary for the combat strategy data loop.

It is intentionally narrower than the full engine/parity effort.

## Scope

This line owns:

- combat decision audit
- preference export
- seed-set curation
- motif summary
- root-search diagnosis
- small search-facing policy priors

This line does **not** own:

- Java parity fixes
- protocol truth expansion
- importer debt cleanup
- noncombat truth correctness

Those belong to the separate `Engine / Truth` line.

## Current Artifacts

Primary code paths:

- `src/bot/search/decision_audit.rs`
- `src/bot/search/mcts.rs`
- `src/bot/search/root_policy.rs`
- `src/bot/search/root_rollout.rs`
- `src/bot/search/sequence_judge.rs`
- `src/bin/combat_decision_audit.rs`
- `tests/combat_decision_audit_cli.rs`

Current seed sets:

- `data/combat_lab/policy_seed_set_20260413_122703.jsonl`
- `data/combat_lab/policy_seed_set_20260412_214122.jsonl`

## What Exists Now

The current loop is:

`live_comm_raw -> replay reconstruction -> audit fixture -> offline branch search -> preference samples -> motif summary -> targeted search regression`

Implemented commands:

- `audit-frame`
- `audit-fixture`
- `extract-fixture`
- `export-preferences`
- `export-preference-seed-set`
- `summarize-preferences`
- `diagnose-search-frame`

Recent structure cleanup:

- root-search pressure and action semantics moved out of `mcts.rs`
- `root_policy.rs` now owns:
  - `StatePressureFeatures`
  - `TransitionPressureDelta`
  - `ActionSemanticTags`
  - root transition policy bonuses used by search ordering
- `sequence_judge.rs` now owns root sequence reranking templates:
  - `survive_now`
  - `play_setup_now` via `setup_then_survive`
  - `defer_setup_to_safe_window`
  - `potion_bridge`
- `root_rollout.rs` now owns:
  - advance to next decision point
  - project turn-close state
  - shared terminal / enemy-total helpers

Implemented sample provenance fields:

- `state_source = reconstructed_live_replay_state`
- `chosen_source = live_replay`
- `preferred_source = offline_audit_search`
- `preferred_search_kind = offline_counterfactual_branch_search`
- `chosen_action_observed`
- `preferred_action_observed`
- audit config fields: `decision_depth`, `top_k`, `branch_cap`

## Verified Regressions

These are currently locked in tests:

- frame `178` search diagnosis now prefers `Play #4 Sword Boomerang+`
- high-pressure search prefers `Defend` over blind attack
- passive setup stays below immediate survival lines
- `Block Potion` is used when it is the only survival bridge

These are tactical regressions, not parity proofs.

## Current Findings

From the current seed sets, the dominant motifs are:

- `survival_window_missed`
- `potion_bridge_available`
- `undervalued_block`

Secondary motifs:

- `attack_over_block`
- `better_outcome_available`
- `heuristic_power_timing`
- `overgreedy_setup`

Interpretation:

- the current search is still strongest at immediate survival gating
- the next missing layer is not “more single-card bonuses”
- the next missing layer is short sequence judgment near the root

## Rules For New Strategy Code

Do not add strategy code that is only expressible as:

- `if card.id == X { +N }`
- `if card.id == Y && hp < Z { -N }`

Unless it is game-rule fact plumbing.

Prefer this layering:

1. state-pressure features
2. action semantic tags
3. reusable sequence templates
4. root-level sequence judge

Current status:

- layer `1` exists in `root_policy.rs`
- layer `2` exists in `root_policy.rs`
- layer `3` exists as initial templates in `sequence_judge.rs`
- layer `4` exists as initial root reranker in `sequence_judge.rs`
- `setup_then_survive` now includes a shallow local follow-up search
- setup timing is now compared as `play now` vs `defer to safe window`
- deferred setup windows now include:
  - later this turn
  - next turn after `EndTurn` rolls into the next decision point
- current follow-up budget is intentionally small: `2` decisions, width `3`

Any new strategy addition should answer:

- is this a rule fact or a policy heuristic?
- does it generalize across multiple cards?
- does it belong at root-level only?
- do we have a regression frame for it?

## Immediate Next Steps

1. Build `StatePressureFeatures`

- unblocked incoming
- lethal pressure
- potion dependence
- near-turn survival margin

2. Build `ActionSemanticTags`

- immediate block
- incoming reduction
- persistent payoff setup
- exhaust outlet
- exhaust fuel
- potion bridge
- damage push

3. Add a root-level `SequenceJudge`

Evaluate only a few candidate first moves:

- best defense move
- best incoming-reduction move
- suspicious setup move
- potion move
- `EndTurn`

4. Use it to rerank root candidates instead of expanding card-specific bonuses.

## Commands

Export a focused seed set:

```powershell
cargo run --bin combat_decision_audit -- export-preference-seed-set --raw d:\rust\sts_simulator\logs\raw\live_comm_raw_20260412_214122.jsonl --out d:\rust\sts_simulator\data\combat_lab\policy_seed_set_20260412_214122.jsonl --summary-out d:\rust\sts_simulator\data\combat_lab\policy_seed_set_20260412_214122.summary.json --frames 153,178,202
```

Summarize current preference motifs:

```powershell
cargo run --bin combat_decision_audit -- summarize-preferences --in d:\rust\sts_simulator\data\combat_lab\policy_seed_set_20260413_122703.jsonl,d:\rust\sts_simulator\data\combat_lab\policy_seed_set_20260412_214122.jsonl --top-examples 8
```

Inspect root-search behavior on a frame:

```powershell
cargo run --bin combat_decision_audit -- diagnose-search-frame --raw d:\rust\sts_simulator\logs\raw\live_comm_raw_20260412_214122.jsonl --frame 178 --depth-limit 5 --top-k 5
```

## Explicit Deferrals

Not part of the next step:

- full MCTS rewrite
- end-to-end learned ranker integration
- same-RNG certification for every preference sample
- card-by-card tactical bonus expansion

Those can come later after the root sequence judge exists.
