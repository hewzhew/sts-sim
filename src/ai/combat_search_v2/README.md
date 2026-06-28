# Combat Search V2 Module Map

This directory is the combat-search mainline. Before adding a new module, check
this map and extend an existing boundary when one already exists.

## Primary Entry Points

- `search.rs`: whole-combat search loop, frontier expansion, terminal handling,
  transposition/dominance checks, and report finalization.
- `frontier/`: frontier queue, priority, `SearchNode`, and resource dominance
  vectors.
- `types/config.rs`: user-visible policy switches. New experimental behavior
  should be named here before it affects search.
- `types/report/`: JSON report schema. Add fields only when a consumer uses
  them to make an implementation decision.

## Search Behavior Boundaries

- `transition.rs`: legal-action filtering and terminal classification wrappers.
- `action_facts/`, `action_effects/`, `action_priority/`: structured action
  semantics. Put reusable action knowledge here instead of embedding it in
  search, rollout, or reports.
- `action_ordering/`: action child-generation order only. It must not prune or
  merge legal actions.
- `action_equivalence/`: soundness-scoped local action-list deduplication only. Do
  not use it for global state merging.
- `turn_planner/`: exact same-turn enumeration and optional frontier seeding.
  Reuse this for turn-level macro candidates; do not create another turn-plan
  system.
  - `root_frontier_seed` seeds exact current-turn end states from the initial
    search root only.
  - `turn_boundary_frontier_seed` is opt-in and seeds exact current-turn end
    states whenever search reaches a new empty-prefix player-turn boundary,
    with exact source-key de-duplication. It does not prune atomic branches or
    create terminal outcome records by itself.
  - `tactical_enemy_turn_boundary_frontier_seed` is an opt-in experimental
    gated seed. It uses the same exact end-state seeding only at empty-prefix
    turn boundaries where typed enemy-mechanics facts show tactical multi-enemy
    pressure, such as a living Healer/support enemy with another living enemy or
    a Fungi Beast swarm. It should stay out of default hallway search until its
    budget and scheduling behavior are tuned enough not to crowd ordinary
    exact search.
- `turn_local_dominance/`: same-parent same-turn pruning only. Cross-turn or
  cross-parent dominance belongs in `frontier/` resource dominance, not here.

## Value And Rollout

- `value/`, `value.rs`, `value_facts.rs`, `pressure_value.rs`,
  `enemy_phase_value.rs`, `card_pile_value.rs`: state evaluation facts and
  ordering scores. These may guide frontier/rollout priority but do not prove
  terminal outcomes.
- `rollout/`, `rollout_cache.rs`, `rollout_policy.rs`,
  `rollout_pending_choice.rs`, `rollout_scheduler.rs`, `rollout_value.rs`:
  estimate-only rollout behavior. Rollout output must remain labeled as
  estimate evidence unless replayed into an exact search node. The default
  `enemy_mechanics_adaptive_no_potion` rollout currently uses phase-aware
  rollout only for typed Guardian and Bronze Automaton mechanics and otherwise
  stays conservative.

## Game-Mechanics State Facts

- `phase_profile/`, `phase_action_ordering.rs`, `enemy_mechanics_profile/`,
  `enemy_phase_transition/`: enemy and phase facts used by value/ordering.
- `pending_choice_profile/`, `pending_choice_ordering/`,
  `pending_choice_fanout.rs`: pending-choice shape, ordering, and fanout risk.
- `potions/`: potion facts, proposal gates, and potion-specific tests. Potion
  policy should stay explicit and opt-in.

## Diagnostics And Report-Only Code

- `diagnostics/`, `diagnostics_tags.rs`, `target_fanout/`, `turn_sequence/`,
  `turn_sequence_effect/`, `discard_order_shadow_audit/`, `card_identity/`,
  `state_abstraction/`: observation, audit, and boundary classification.
  These modules must not remove exact branches unless the boundary is promoted
  to a prune-safe consumer.
- `decision_microscope/` and `rollout_probe/`: opt-in analysis tools. Do not
  route normal search behavior through them.
- `trajectory_report.rs` and `baseline.rs`: whole-combat outcome reporting and
  baseline comparison. Baselines are comparison evidence, not teacher labels.

## Rules To Avoid Duplicate Systems

1. If the work is about full-turn candidates, start in `turn_planner/`.
2. If the work is about child order, start in `action_ordering/` or
   `phase_action_ordering.rs`.
3. If the work is about pruning, identify the safe-pruning boundary first:
   `action_equivalence/`, `frontier/`, or `turn_local_dominance/`.
4. If the work is about estimates, use `value/` or `rollout/`; do not let it
   claim a terminal outcome.
5. If the work only explains behavior, it belongs in diagnostics and needs a
   concrete consumer before adding more report fields.
6. New top-level files in this directory should be rare. Prefer extending the
   nearest existing submodule and updating this map.
