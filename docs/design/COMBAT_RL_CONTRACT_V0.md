# Combat RL Contract V0

## Scope
- This contract is for `combat-only`, `local spec`, `Rust simulator + Python RL`.
- It does not cover runtime/live inference, run-level planning, or MCTS.
- `candidate_index` is not a public action ontology. It is only a legality and execution bridge.

## Source Of Truth
- Rust harness and driver own combat truth:
  - simulator state
  - legal actions
  - reward breakdown
  - structured observation payload
- Python bridge owns transport and tensorization:
  - padded object tensors
  - per-head masks
  - canonical action decoding
- Policy owns action priors and value estimation:
  - no candidate scoring core
  - no heuristic target picker core

## Observation V0

### Top-level layout
- `meta`
  - `contract_version`
  - `spec_name`
  - `seed`
  - `turn_count`
  - `phase`
  - `engine_state`
- `player`
  - `hp`
  - `max_hp`
  - `block`
  - `energy`
- `player_powers`
  - structured power objects with masks
- `hand_cards`
  - structured card objects with masks
- `potions`
  - structured potion objects with masks
- `monsters`
  - structured monster objects with masks
- `turn_prefix`
  - per-turn history summary
- `pending_choice`
  - choice kind, options, selection bounds, cancelability
- `available`
  - legality masks for policy heads
- `diagnostics`
  - heuristic summaries for logs and ablations only

### Hidden zone rule
- `draw_count`, `discard_count`, and `exhaust_count` are public observation
  fields.
- Draw/discard/exhaust card identities and draw pile order are not public
  policy observations.
- Replay/live snapshots may contain full hidden engine truth for validation,
  but `CombatObservation` and Python gym tensorization must not expose it.
- `combat_env_driver` reset supports `draw_order_variant=reshuffle_draw` for
  local robustness checks where the same visible start state is evaluated under
  alternate hidden draw orders.

### Hand card fields
- `slot`
- `uuid`
- `card_id`
- `card_type`
- `target_mode`
- `cost_for_turn`
- `upgraded`
- `playable`
- `exhausts_when_played`
- `ethereal`
- `retain`

### Potion fields
- `slot`
- `potion_id`
- `target_mode`
- `usable`

### Monster fields
- `slot`
- `entity_id`
- `monster_id`
- `hp`
- `max_hp`
- `block`
- `alive`
- `targetable`
- `visible_intent`
- `intent_payload`
- `powers`
- `mechanic_state`

### Turn prefix fields
- `cards_played_this_turn`
- `attacks_played_this_turn`
- `skills_played_this_turn`
- `powers_played_this_turn`
- `energy_spent_this_turn`
- `damage_dealt_this_turn`
- `damage_taken_this_turn`
- `last_action_family`
- `last_card_id`

### Pending choice fields
- `kind`
- `min_select`
- `max_select`
- `can_cancel`
- `options`

### Choice option fields
- `option_index`
- `label`
- `card_id`
- `card_uuid`
- `source_pile`
- `selection_indices`
- `selection_uuids`

## Explicit Truth vs Diagnostics

### Must be explicit truth
- Monster phase and counters that are rule-critical.
- Monster powers and player powers.
- Choice options and selection bounds.
- Potion availability and target modes.
- Per-turn action prefix.

### Mechanic state V0 minimum
- Lagavulin
  - `sleeping`
  - `idle_count`
  - `debuff_turn_count`
  - `wake_triggered`
- Guardian
  - `guardian_threshold`
  - `guardian_damage_taken`
  - `guardian_open`
  - `close_up_triggered`
- Slime Boss and split-driven enemies
  - `split_threshold`
  - `split_ready`
- Darkling
  - `half_dead`
  - `regrow_counter`
- Generic support
  - `planned_move_id`
  - `move_history`

### Diagnostics only
- `pressure`
- `belief`
- `hexaghost_future_script`
- heuristic summaries equivalent to action advice
- any direct tactical recommendation field such as:
  - `best_target`
  - `survival_guard`
  - `lethal_pressure`

Diagnostics may be logged or used in controlled ablations. They are excluded from the main structured policy tensors.

## Canonical Action V0

### Public semantic action
- `EndTurn`
- `PlayCard { card_slot, target_slot? }`
- `UsePotion { potion_slot, target_slot? }`
- `SubmitChoice { choice_kind, selection }`
- `Proceed`
- `Cancel`

### Choice semantics
- `selection` means the chosen option set for the current pending choice.
- Single-choice branches such as discovery, stance choice, and card reward use one-element selection.
- Multi-select branches such as hand/grid select use set semantics.
- V0 transport may still carry a local `choice_index` inside the pending-choice subproblem, but this is an adapter detail, not the public ontology.

### Driver bridge rule
- Policy outputs canonical action semantics.
- Python bridge decodes canonical action into exactly one legal driver candidate.
- If decoding fails or is ambiguous, it is a contract bug, not exploration.

## Available Masks
- `action_type_mask`
- `play_card_mask`
- `play_card_target_mask[card_slot]`
- `use_potion_mask`
- `use_potion_target_mask[potion_slot]`
- `choice_option_mask`
- `choice_can_cancel`

## Training Transport V0
- Python structured env exposes `spaces.Dict`.
- Transport fields are:
  - `action_type`
  - `card_slot`
  - `target_slot`
  - `potion_slot`
  - `choice_index`
- These transport fields are not the semantic contract.
- `candidate_index` never leaves the bridge boundary.

## Policy Contract V0
- Shared encoder with object inputs for:
  - player/global
  - hand
  - potions
  - monsters
  - pending choice options
- Tactical latent bottleneck plus probe heads.
- Multi-head actor:
  - action type
  - card slot
  - card target
  - potion slot
  - potion target
  - choice option
- Explicitly out of scope:
  - candidate reranking as policy core
  - full-turn autoregressive combo generation
  - planner-in-the-loop
  - runtime/live inference

## Reference Implementation
- Rust structured truth:
  - [src/bot/harness/combat_env.rs](/d:/rust/sts_simulator/src/bot/harness/combat_env.rs:1)
- Driver payload and candidate bridge:
  - [src/bin/combat_env_driver/main.rs](/d:/rust/sts_simulator/src/bin/combat_env_driver/main.rs:1)
- Structured Python bridge:
  - [tools/learning/structured_combat_env.py](/d:/rust/sts_simulator/tools/learning/structured_combat_env.py:1)
- Structured policy:
  - [tools/learning/structured_policy.py](/d:/rust/sts_simulator/tools/learning/structured_policy.py:1)
- Structured PPO trainer:
  - [tools/learning/train_structured_combat_ppo.py](/d:/rust/sts_simulator/tools/learning/train_structured_combat_ppo.py:1)
