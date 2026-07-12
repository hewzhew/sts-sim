# Current-Turn Retaliation Protection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explore a payable block Skill before a retaliation-bearing Attack only when that two-card order strictly reduces combined known retaliation and visible-incoming HP loss.

**Architecture:** Add one bounded counterfactual helper beside the existing current-turn setup logic. It evaluates current attack facts, builds a scratch post-block state, reevaluates the same attack with engine-derived retaliation projection, and emits a semantic ordering role without changing legality, pruning, rollout, frontier value, or state identity.

**Tech Stack:** Rust 2021, existing combat-search action facts and ordering, exact engine combat stepper for tests, serde diagnostics, Cargo tests.

## Global Constraints

- Work in the stable checkout on local `master`; do not create a worktree or dispatch subagents.
- Do not run `cargo clean`.
- Do not add `EnemyId::Spiker`, Shapes, or Thorns-specific policy checks.
- Do not globally promote block cards or add a retaliation-to-progress coefficient.
- Do not add a cross-turn path ledger, frontier lane, prune, merge, rollout change, or state-identity field.
- Preserve every legal action and original action identifier; this slice changes child-generation order only.
- Reuse `card_play_effect_facts` and the engine-owned defense-aware retaliation projection.
- Keep the frozen A3F42 trajectory as ignored evidence, not a permanent seed assertion.

---

### Task 1: Characterize the missing protection role

**Files:**
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`

**Interfaces:**
- Consumes: existing `priority_for_input` test helper and `CardPlayEffectDiagnostics` retaliation fields.
- Produces: red tests for `ActionOrderingRole::CurrentTurnRetaliationProtection` and its eligibility boundaries.

- [ ] **Step 1: Add the failing A3F42-shaped ordering test**

Add a local fixture helper that creates a living monster with engine `Thorns`, zero visible incoming damage, four player energy, Heavy Blade, and Flame Barrier. Then add:

```rust
#[test]
fn block_skill_precedes_payable_retaliation_attack_when_order_saves_hp() {
    let combat = retaliation_ordering_combat(4, 13);

    let flame = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let heavy = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
    assert!(flame > heavy);
}
```

Use a generic test monster and insert `PowerId::Thorns`; the production implementation must not inspect that id directly.

- [ ] **Step 2: Add failing boundary tests**

Add this helper and three tests:

```rust
fn flame_priority(combat: &CombatState) -> ActionOrderingPriority {
    priority_for_input(
        &EngineState::CombatPlayerTurn,
        combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    )
}

#[test]
fn retaliation_protection_requires_both_cards_to_be_payable() {
    let combat = retaliation_ordering_combat(3, 13);
    let flame = flame_priority(&combat);
    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}

#[test]
fn block_skill_is_not_retaliation_setup_without_retaliation() {
    let mut combat = retaliation_ordering_combat(4, 0);
    combat.entities.power_db.remove(&1);
    let flame = flame_priority(&combat);
    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}

#[test]
fn retaliation_setup_is_not_promoted_when_visible_damage_makes_orders_equal() {
    let mut combat = retaliation_ordering_combat(4, 13);
    add_visible_attacker(&mut combat, 20);
    let flame = flame_priority(&combat);
    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}
```

The visible attacker uses `MonsterMoveSpec::Attack(AttackSpec { base_damage: 20, hits: 1, damage_kind: DamageKind::Normal })` so the equality is engine-derived.

- [ ] **Step 3: Run RED**

Run:

```powershell
cargo test --lib current_turn_retaliation_protection -- --nocapture
cargo test --lib block_skill_precedes_payable_retaliation_attack -- --nocapture
```

Expected: compilation fails because `CurrentTurnRetaliationProtection` does not exist.

---

### Task 2: Implement bounded current-turn protection ordering

**Files:**
- Modify: `src/ai/combat_search_v2/action_priority/role.rs`
- Modify: `src/ai/combat_search_v2/action_priority/constants.rs`
- Modify: `src/ai/combat_search_v2/action_priority/play_card/setup.rs`
- Modify: `src/ai/combat_search_v2/action_priority/play_card/mod.rs`
- Test: `src/ai/combat_search_v2/action_priority/tests.rs`

**Interfaces:**
- Produces: `ActionOrderingRole::CurrentTurnRetaliationProtection` with label `current_turn_retaliation_protection`.
- Produces: `current_turn_retaliation_protection_score(combat: &CombatState, setup_card_index: usize, setup_block: i32, setup_cost: i32, visible_incoming_damage: i32) -> i32`.
- Consumes: `card_play_effect_facts`, `cards::can_play_card`, effective card targets, current energy, visible incoming damage, and defense-aware retaliation facts.

- [ ] **Step 1: Add the semantic role and rank**

Add the enum variant after `PreventHpLoss` and its label in `role.rs`:

```rust
ActionOrderingRole::CurrentTurnRetaliationProtection => {
    "current_turn_retaliation_protection"
}
```

Add a named rank in `constants.rs`:

```rust
pub(super) const ROLE_CURRENT_TURN_RETALIATION_PROTECTION: i32 = ROLE_PREVENT_HP_LOSS;
```

This places the role above `DamageProgress` without introducing a damage coefficient.

- [ ] **Step 2: Implement the counterfactual helper**

Import `card_play_effect_facts`, `CardTarget`, and `ClientInput` dependencies into `setup.rs`. Implement the helper with this structure:

```rust
pub(super) fn current_turn_retaliation_protection_score(
    combat: &CombatState,
    setup_card_index: usize,
    setup_block: i32,
    setup_cost: i32,
    visible_incoming_damage: i32,
) -> i32 {
    if setup_block <= 0 || setup_cost < 0 || setup_cost > i32::from(combat.turn.energy) {
        return 0;
    }

    let mut post_setup = combat.clone();
    if setup_card_index >= post_setup.zones.hand.len() {
        return 0;
    }
    post_setup.zones.hand.remove(setup_card_index);
    post_setup.turn.energy = (i32::from(combat.turn.energy) - setup_cost) as u8;
    post_setup.turn.counters.cards_played_this_turn = post_setup
        .turn
        .counters
        .cards_played_this_turn
        .saturating_add(1);
    post_setup.entities.player.block = post_setup
        .entities
        .player
        .block
        .saturating_add(setup_block);

    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, card)| {
            *index != setup_card_index
                && cards::get_card_definition(card.id).card_type == CardType::Attack
                && cards::can_play_card(card, combat).is_ok()
        })
        .filter_map(|(_, current_attack)| {
            let projected_attack = post_setup
                .zones
                .hand
                .iter()
                .find(|card| card.uuid == current_attack.uuid)?;
            if projected_attack.cost_for_turn_java() < 0 && post_setup.turn.energy == 0 {
                return None;
            }
            if cards::can_play_card(projected_attack, &post_setup).is_err() {
                return None;
            }
            Some(
                attack_targets(combat, current_attack)
                    .into_iter()
                    .map(|target| {
                        retaliation_order_hp_benefit(
                            combat,
                            &post_setup,
                            current_attack,
                            projected_attack,
                            target,
                            setup_block,
                            visible_incoming_damage,
                        )
                    })
                    .max()
                    .unwrap_or_default(),
            )
        })
        .max()
        .unwrap_or_default()
        .max(0)
}
```

Implement `attack_targets` as:

```rust
fn attack_targets(combat: &CombatState, card: &CombatCard) -> Vec<Option<usize>> {
    match cards::effective_target(card) {
        CardTarget::Enemy | CardTarget::SelfAndEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| Some(monster.id))
            .collect(),
        CardTarget::AllEnemy | CardTarget::All => vec![None],
        _ => Vec::new(),
    }
}
```

Implement `retaliation_order_hp_benefit` by computing current and post-setup attack facts. Return zero unless current retaliation HP loss is positive. For each order, add retaliation HP loss to `max(visible_incoming_damage - remaining_block_after_both, 0)`. Return attack-first known loss minus block-first known loss.

- [ ] **Step 3: Wire eligibility and ordering role**

In `priority_for_play_card`, compute the setup score only when:

```rust
def.card_type == CardType::Skill
    && block > 0
    && resource_timing.hand_exhaust_target_count == 0
    && !effects.reactive.forced_turn_end
```

Insert the new role after key setup and before current-turn attack setup:

```rust
} else if current_turn_retaliation_protection > 0 {
    (
        ActionOrderingRole::CurrentTurnRetaliationProtection,
        ROLE_CURRENT_TURN_RETALIATION_PROTECTION,
    )
```

Add the positive score to the existing `phase_setup` tie-break alongside `current_turn_attack_setup`. Do not alter legal actions or any frontier code.

- [ ] **Step 4: Run GREEN and action-ordering neighbors**

Run:

```powershell
cargo fmt --all
cargo test --lib block_skill_precedes_payable_retaliation_attack -- --nocapture
cargo test --lib retaliation_protection_requires_both_cards_to_be_payable -- --nocapture
cargo test --lib block_skill_is_not_retaliation_setup_without_retaliation -- --nocapture
cargo test --lib retaliation_setup_is_not_promoted_when_visible_damage_makes_orders_equal -- --nocapture
cargo test --lib ai::combat_search_v2::action_priority::tests -- --nocapture
cargo test --lib ai::combat_search_v2::action_ordering::tests -- --nocapture
```

Expected: all focused and neighboring ordering tests pass; no action count changes.

---

### Task 3: Verify exact two-action behavior and multi-hit reuse

**Files:**
- Modify: `src/ai/combat_search_v2/action_ordering/tests/core.rs`

**Interfaces:**
- Consumes: Task 2 ordering role and the real `EngineCombatStepper`.
- Produces: synthetic exact evidence that reordering preserves actions and saves HP; proves multi-hit uses sequential projection.

- [ ] **Step 1: Add exact two-action execution test**

Construct a stable player-turn combat with enough energy, Flame Barrier, Heavy Blade, and a 13-retaliation target. Execute both permutations using `EngineCombatStepper::apply_to_stable`, locating the second card by UUID after the first play. Assert:

```rust
assert_eq!(attack_then_block.entities.player.current_hp, initial_hp - 13);
assert_eq!(block_then_attack.entities.player.current_hp, initial_hp);
assert_eq!(attack_then_block.entities.player.block, 16);
assert_eq!(block_then_attack.entities.player.block, 3);
```

Before execution, order the same complete legal action list and assert its length and original action-id set are unchanged.

- [ ] **Step 2: Add multi-hit protection test**

Use Defend, Twin Strike, two energy, five block from setup, and Thorns 3. Assert Defend receives the protection role because sequential projection changes Twin Strike from six HP loss to one HP loss. Do not calculate `hits * thorns` in production code.

- [ ] **Step 3: Run exact and multi-hit tests**

```powershell
cargo fmt --all
cargo test --lib retaliation_protection_exact_order -- --nocapture
cargo test --lib retaliation_protection_uses_sequential_multi_hit_projection -- --nocapture
```

Expected: exact state deltas and semantic role assertions pass.

- [ ] **Step 4: Commit the behavior slice**

```powershell
git add -- src/ai/combat_search_v2/action_priority src/ai/combat_search_v2/action_ordering/tests/core.rs
git commit -m "feat: prioritize current-turn retaliation protection"
```

---

### Task 4: Verify repository and frozen A3F42 evidence

**Files:**
- Create ignored evidence: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/diagnostics/a3f42_lazy_all_8s_retaliation_protection.json`

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: complete regression evidence and a frozen diagnostic comparison; no push and no permanent seed assertion.

- [ ] **Step 1: Run repository verification**

```powershell
cargo fmt --all -- --check
git diff --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: all library and all seven architecture-boundary tests pass.

- [ ] **Step 2: Rerun frozen A3F42**

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\accepted_high_loss_combat\seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json" --max-nodes 800000 --wall-ms 8000 --potion-policy all --max-potions-used 3 --child-rollout-policy lazy-on-pop --output "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\diagnostics\a3f42_lazy_all_8s_retaliation_protection.json"
```

Record:

- best win final HP and HP loss;
- whether Flame Barrier precedes Heavy Blade on turn 6;
- first-win node and 8-second node coverage;
- count of `current_turn_retaliation_protection` roles in ordering diagnostics;
- raw/block/HP retaliation aggregates.

An improved HP result is expected from the reproduced counterfactual but is evidence, not a permanent pass condition.

- [ ] **Step 3: Inspect final local state**

```powershell
git status --short --branch
git log -8 --oneline
```

Expected: tracked work is committed on local `master`, ignored evidence does not dirty the tree, and nothing is pushed.
