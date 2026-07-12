# Attack Retaliation Threat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make combat-search facts, ordering, and conservative rollout account for the immediate per-hit HP cost of attacking Thorns targets without adding Spiker-specific weights.

**Architecture:** Add one shared engine-derived target fact that resolves visible `on_attacked` powers for a hypothetical ordinary player damage event. Card-action observation applies that fact once per explicitly resolved damage action and feeds the existing reactive-risk channel; diagnostics project the same fact without changing frontier value.

**Tech Stack:** Rust 2021, existing combat power resolvers, Cargo unit/integration tests, frozen combat-search driver diagnostics.

## Global Constraints

- Work in the stable checkout on the existing local `master`; do not create a worktree or dispatch subagents.
- Do not run `cargo clean`.
- Do not hard-code `EnemyId::Spiker` in the new fact or ordering consumer.
- Do not add a minimum-HP hard rejection or modify terminal/frontier scoring in this slice.
- Do not double count Sharp Hide, which remains an `on_card_played` reaction.
- Treat runtime-expanded actions that are represented as one resolved action as one conservative retaliation hint in this slice; explicit repeated actions such as Twin Strike and Pummel are counted per hit.
- Keep the A3F42 line as ignored diagnostic evidence, not a permanent card-sequence test.

---

### Task 1: Add the engine-derived target retaliation fact

**Files:**
- Create: `src/ai/combat_search_v2/attack_retaliation.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`

**Interfaces:**
- Consumes: `powers_snapshot_for`, `resolve_power_on_attacked`, `MonsterEntity::turn_plan`, and visible `MoveStep::ApplyPower` facts.
- Produces: `AttackRetaliationTargetFactV1`, `attack_retaliation_for_target`, and `attack_retaliation_player_hp_loss_for_event` for Tasks 2 and 3.

- [ ] **Step 1: Add failing target-fact tests**

Create `attack_retaliation.rs` with the type declarations and tests first. The production functions should be referenced but not yet defined so RED is a missing-function compile failure:

```rust
use crate::EntityId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct AttackRetaliationTargetFactV1 {
    pub(super) source_entity_id: EntityId,
    pub(super) power_source_count: usize,
    pub(super) player_hp_loss_per_damage_event: i32,
    pub(super) visible_growth_amount: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::action::{DamageInfo, DamageType};
    use crate::runtime::combat::{CombatState, Power, PowerPayload};
    use crate::runtime::monster_move::{BuffSpec, MonsterMoveSpec};
    use crate::test_support::{blank_test_combat, test_monster};

    fn thorns_combat(amount: i32) -> CombatState {
        let mut combat = blank_test_combat();
        let mut spiker = test_monster(EnemyId::Spiker);
        spiker.id = 7;
        spiker.set_planned_steps(
            MonsterMoveSpec::Buff(BuffSpec {
                power_id: PowerId::Thorns,
                amount: 2,
            })
            .to_steps(),
        );
        combat.entities.monsters = vec![spiker];
        combat.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::Thorns,
                instance_id: None,
                amount,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat
    }

    #[test]
    fn target_fact_reports_engine_resolved_thorns_and_visible_growth() {
        let combat = thorns_combat(3);

        assert_eq!(
            attack_retaliation_for_target(&combat, 7),
            Some(AttackRetaliationTargetFactV1 {
                source_entity_id: 7,
                power_source_count: 1,
                player_hp_loss_per_damage_event: 3,
                visible_growth_amount: 2,
            })
        );
    }

    #[test]
    fn event_loss_uses_actual_damage_source_and_type() {
        let combat = thorns_combat(5);
        let info = DamageInfo {
            source: 0,
            target: 7,
            base: 6,
            output: 6,
            damage_type: DamageType::Normal,
            is_modified: false,
        };

        assert_eq!(
            attack_retaliation_player_hp_loss_for_event(&combat, &info),
            5
        );
        assert_eq!(
            attack_retaliation_player_hp_loss_for_event(
                &combat,
                &DamageInfo {
                    damage_type: DamageType::Thorns,
                    ..info
                }
            ),
            0
        );
    }

    #[test]
    fn target_fact_is_absent_without_player_damage_retaliation() {
        let mut combat = thorns_combat(0);
        combat.entities.power_db.remove(&7);

        assert_eq!(attack_retaliation_for_target(&combat, 7), None);
    }
}
```

- [ ] **Step 2: Register the module and verify RED**

Add to `src/ai/combat_search_v2/mod.rs`:

```rust
mod attack_retaliation;
```

Run:

```powershell
cargo test --lib attack_retaliation -- --nocapture
```

Expected: compilation fails because `attack_retaliation_for_target` and `attack_retaliation_player_hp_loss_for_event` are absent.

- [ ] **Step 3: Implement the fact through engine power resolvers**

Add these functions to `attack_retaliation.rs`:

```rust
use crate::content::powers::{
    resolve_power_on_attacked, store::powers_snapshot_for, PowerId,
};
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::CombatState;
use crate::runtime::monster_move::{MoveStep, MoveTarget};
use crate::EntityId;

pub(super) fn attack_retaliation_player_hp_loss_for_event(
    combat: &CombatState,
    info: &DamageInfo,
) -> i32 {
    if info.source != 0
        || !combat.entities.monsters.iter().any(|monster| {
            monster.id == info.target && monster.is_alive_for_action()
        })
    {
        return 0;
    }
    powers_snapshot_for(combat, info.target)
        .into_iter()
        .flat_map(|power| {
            resolve_power_on_attacked(
                power.power_type,
                combat,
                info.target,
                info.output.max(info.base),
                info.source,
                info.damage_type,
                power.amount,
            )
        })
        .map(player_hp_loss_from_action)
        .sum()
}

pub(super) fn attack_retaliation_for_target(
    combat: &CombatState,
    entity_id: EntityId,
) -> Option<AttackRetaliationTargetFactV1> {
    let owner = combat.entities.monsters.iter().find(|monster| {
        monster.id == entity_id && monster.is_alive_for_action()
    })?;
    let mut source_count = 0usize;
    let mut player_loss = 0i32;
    for power in powers_snapshot_for(combat, entity_id) {
        let loss: i32 = resolve_power_on_attacked(
            power.power_type,
            combat,
            entity_id,
            1,
            0,
            DamageType::Normal,
            power.amount,
        )
        .into_iter()
        .map(player_hp_loss_from_action)
        .sum();
        if loss > 0 {
            source_count += 1;
            player_loss = player_loss.saturating_add(loss);
        }
    }
    (player_loss > 0).then_some(AttackRetaliationTargetFactV1 {
        source_entity_id: entity_id,
        power_source_count: source_count,
        player_hp_loss_per_damage_event: player_loss,
        visible_growth_amount: owner
            .turn_plan()
            .steps
            .iter()
            .filter_map(|step| match step {
                MoveStep::ApplyPower(power)
                    if power.target == MoveTarget::SelfTarget
                        && power.power_id == PowerId::Thorns
                        && power.amount > 0 =>
                {
                    Some(power.amount)
                }
                _ => None,
            })
            .sum(),
    })
}

fn player_hp_loss_from_action(action: Action) -> i32 {
    match action {
        Action::Damage(info) if info.target == 0 => info.output.max(info.base).max(0),
        Action::LoseHp { target: 0, amount, .. } => amount.max(0),
        _ => 0,
    }
}
```

Do not special-case `EnemyId::Spiker`.

- [ ] **Step 4: Run target-fact tests and formatting**

Run:

```powershell
cargo fmt --all
cargo test --lib attack_retaliation -- --nocapture
```

Expected: three retaliation tests pass with no warnings.

- [ ] **Step 5: Commit the shared fact**

```powershell
git add -- src/ai/combat_search_v2/mod.rs src/ai/combat_search_v2/attack_retaliation.rs
git commit -m "feat: expose attack retaliation facts"
```

---

### Task 2: Project retaliation into card effects and shared ordering

**Files:**
- Create: `src/ai/combat_search_v2/action_effects/card_play_effects/attack_retaliation_observation.rs`
- Modify: `src/ai/combat_search_v2/action_effects/card_play_effects.rs`
- Modify: `src/ai/combat_search_v2/action_effects/card_play_effects/observation.rs`
- Modify: `src/ai/combat_search_v2/action_effects/types.rs`
- Modify: `src/ai/combat_search_v2/action_effects/tests.rs`
- Modify: `src/ai/combat_search_v2/action_facts/types.rs`
- Modify: `src/ai/combat_search_v2/action_facts/mechanics.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`
- Modify: `src/ai/combat_search_v2/rollout_action_selector/tests.rs`

**Interfaces:**
- Consumes: `attack_retaliation_player_hp_loss_for_event` from Task 1 and resolved card `Action` values.
- Produces: retaliation attribution inside `ReactiveCardPlayEffectFacts`, public action diagnostics, and behavior through the existing `reactive_risk_score`.

- [ ] **Step 1: Add failing single-hit, multi-hit, and Sharp Hide tests**

Extend `action_effects/tests.rs` using its existing `insert_power` helper:

```rust
#[test]
fn thorns_reports_per_hit_attack_retaliation_without_double_counting_sharp_hide() {
    let mut combat = blank_test_combat();
    let mut target = test_monster(EnemyId::Spiker);
    target.id = 1;
    combat.entities.monsters = vec![target];
    insert_power(&mut combat, 1, PowerId::Thorns, 3);

    let strike = card_play_effect_facts(
        &combat,
        &CombatCard::new(CardId::Strike, 10),
        Some(1),
    );
    let twin = card_play_effect_facts(
        &combat,
        &CombatCard::new(CardId::TwinStrike, 11),
        Some(1),
    );

    assert_eq!(strike.reactive.attack_retaliation_trigger_count_hint, 1);
    assert_eq!(strike.reactive.attack_retaliation_player_hp_loss_hint, 3);
    assert_eq!(strike.reactive.player_hp_loss, 3);
    assert_eq!(twin.reactive.attack_retaliation_trigger_count_hint, 2);
    assert_eq!(twin.reactive.attack_retaliation_player_hp_loss_hint, 6);
    assert_eq!(twin.reactive.player_hp_loss, 6);

    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::SharpHide,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    let sharp_hide = card_play_effect_facts(
        &combat,
        &CombatCard::new(CardId::Strike, 12),
        Some(1),
    );
    assert_eq!(sharp_hide.reactive.player_hp_loss, 3);
    assert_eq!(
        sharp_hide.reactive.attack_retaliation_player_hp_loss_hint,
        0
    );
}

#[test]
fn non_attack_does_not_trigger_thorns_retaliation() {
    let mut combat = blank_test_combat();
    let mut target = test_monster(EnemyId::Spiker);
    target.id = 1;
    combat.entities.monsters = vec![target];
    insert_power(&mut combat, 1, PowerId::Thorns, 3);

    let defend = card_play_effect_facts(
        &combat,
        &CombatCard::new(CardId::Defend, 10),
        None,
    );

    assert_eq!(defend.reactive.attack_retaliation_trigger_count_hint, 0);
    assert_eq!(defend.reactive.attack_retaliation_player_hp_loss_hint, 0);
}
```

- [ ] **Step 2: Add failing consumer tests**

In `action_priority/tests.rs`, add a test with `[TwinStrike, Strike]` targeting the same 40-HP Thorns owner. Require both actions to retain `DamageProgress`, but require Strike to have less reactive risk and compare greater than Twin Strike:

```rust
assert_eq!(strike.role, ActionOrderingRole::DamageProgress);
assert_eq!(twin.role, ActionOrderingRole::DamageProgress);
assert_eq!(strike.effects.reactive.attack_retaliation_player_hp_loss_hint, 3);
assert_eq!(twin.effects.reactive.attack_retaliation_player_hp_loss_hint, 6);
assert!(strike > twin);
```

Add the same setup without Thorns and assert both retaliation hints are zero; existing damage progress remains the deciding signal.

In `rollout_action_selector/tests.rs`, add `conservative_rollout_reuses_attack_retaliation_ordering`: put Twin Strike first and Strike second in `legal`, use the existing `ProbeWinStepper`, and assert the selected input plays the Strike card index.

- [ ] **Step 3: Run the retaliation tests and verify RED**

Run:

```powershell
cargo test --lib attack_retaliation -- --nocapture
cargo test --lib conservative_rollout_reuses_attack_retaliation_ordering -- --nocapture
```

Expected: compilation fails because the reactive attribution fields are absent.

- [ ] **Step 4: Add internal and public attribution fields**

Add to `ReactiveCardPlayEffectFacts` and `CardPlayReactiveEffectDiagnostics` in `action_effects/types.rs`:

```rust
pub(in crate::ai::combat_search_v2) attack_retaliation_trigger_count_hint: usize,
pub(in crate::ai::combat_search_v2) attack_retaliation_player_hp_loss_hint: i32,
```

Project both fields in `CardPlayEffectFacts::diagnostics`.

Add the same public fields to `CombatSearchV2ActionReactiveMechanicsFacts` in `action_facts/types.rs`, and map them from `effects.diagnostics().reactive` in `action_facts/mechanics.rs`.

- [ ] **Step 5: Observe explicit resolved damage actions**

Create `attack_retaliation_observation.rs` with a helper that accepts `&Action`. Match every `DamageInfo`-carrying action already handled by `action_facts/payload.rs`, plus `DamageAllEnemies` and `VampireDamageAllEnemies`. For each explicit event call:

```rust
use crate::ai::combat_search_v2::attack_retaliation::
    attack_retaliation_player_hp_loss_for_event;
use crate::runtime::action::{Action, DamageInfo};
use crate::runtime::combat::CombatState;

use super::super::ReactiveCardPlayEffectFacts;

pub(super) fn observe_attack_retaliation_action(
    combat: &CombatState,
    reactive: &mut ReactiveCardPlayEffectFacts,
    action: &Action,
) {
    match action {
        Action::Damage(info)
        | Action::PummelDamage(info)
        | Action::BaneDamage(info)
        | Action::WallopDamage(info)
        | Action::DamagePerAttackPlayed(info)
        | Action::HeelHook(info)
        | Action::Flechettes(info)
        | Action::DropkickDamageAndEffect {
            damage_info: info, ..
        }
        | Action::Ftl {
            damage_info: info, ..
        }
        | Action::Skewer {
            damage_info: info, ..
        }
        | Action::Sunder {
            damage_info: info, ..
        }
        | Action::FearNoEvil {
            damage_info: info, ..
        }
        | Action::FiendFire {
            damage_info: info, ..
        }
        | Action::Feed {
            damage_info: info, ..
        }
        | Action::LessonLearned {
            damage_info: info, ..
        }
        | Action::HandOfGreed {
            damage_info: info, ..
        }
        | Action::RitualDagger {
            damage_info: info, ..
        }
        | Action::VampireDamage(info)
        | Action::Barrage { damage: info } => observe_damage_event(combat, reactive, info),
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            ..
        }
        | Action::VampireDamageAllEnemies {
            source,
            damages,
            damage_type,
        } => {
            for (slot, damage) in damages.iter().copied().enumerate() {
                let Some(target) = combat.entities.monsters.get(slot) else {
                    continue;
                };
                if !target.is_alive_for_action() {
                    continue;
                }
                observe_damage_event(
                    combat,
                    reactive,
                    &DamageInfo {
                        source: *source,
                        target: target.id,
                        base: damage,
                        output: damage,
                        damage_type: *damage_type,
                        is_modified: false,
                    },
                );
            }
        }
        Action::Whirlwind {
            damages,
            damage_type,
            ..
        } => {
            for (slot, damage) in damages.iter().copied().enumerate() {
                let Some(target) = combat.entities.monsters.get(slot) else {
                    continue;
                };
                if !target.is_alive_for_action() {
                    continue;
                }
                observe_damage_event(
                    combat,
                    reactive,
                    &DamageInfo {
                        source: 0,
                        target: target.id,
                        base: damage,
                        output: damage,
                        damage_type: *damage_type,
                        is_modified: false,
                    },
                );
            }
        }
        _ => {}
    }
}

fn observe_damage_event(
    combat: &CombatState,
    reactive: &mut ReactiveCardPlayEffectFacts,
    info: &DamageInfo,
) {
    let hp_loss = attack_retaliation_player_hp_loss_for_event(combat, info);
    if hp_loss <= 0 {
        return;
    }
    reactive.attack_retaliation_trigger_count_hint = reactive
        .attack_retaliation_trigger_count_hint
        .saturating_add(1);
    reactive.attack_retaliation_player_hp_loss_hint = reactive
        .attack_retaliation_player_hp_loss_hint
        .saturating_add(hp_loss);
    reactive.player_hp_loss = reactive.player_hp_loss.saturating_add(hp_loss);
}
```

For all-enemy actions, construct one `DamageInfo` per living slot using the action's source, damage, and damage type. For `Whirlwind`, conservatively observe one event per living target in this slice. Do not expand `FiendFire`, `Skewer`, or other runtime-compressed repetitions here.

Declare the module in `card_play_effects.rs`:

```rust
mod attack_retaliation_observation;
```

In `observation::observe_card_play_effects`, call the new observer with `&action` before moving `action` into `observe_direct_action`:

```rust
for action in actions {
    super::attack_retaliation_observation::observe_attack_retaliation_action(
        combat,
        &mut accumulator.reactive,
        &action,
    );
    observe_direct_action(combat, &mut accumulator.direct, action);
}
```

Do not change `reactive_risk_score`; it already includes `reactive.player_hp_loss`.

- [ ] **Step 6: Run focused effect, ordering, and rollout tests**

Run:

```powershell
cargo fmt --all
cargo test --lib thorns_reports_per_hit_attack_retaliation -- --nocapture
cargo test --lib non_attack_does_not_trigger_thorns_retaliation -- --nocapture
cargo test --lib action_priority -- --nocapture
cargo test --lib conservative_rollout_reuses_attack_retaliation_ordering -- --nocapture
```

Expected: all focused tests pass; the existing Sharp Hide test remains green and reports no attack-retaliation attribution.

- [ ] **Step 7: Commit the action consumer**

```powershell
git add -- src/ai/combat_search_v2/action_effects src/ai/combat_search_v2/action_facts src/ai/combat_search_v2/action_priority/tests.rs src/ai/combat_search_v2/rollout_action_selector/tests.rs
git commit -m "feat: price attack retaliation in action ordering"
```

---

### Task 3: Project target and aggregate retaliation diagnostics

**Files:**
- Modify: `src/ai/combat_search_v2/action_facts/types.rs`
- Modify: `src/ai/combat_search_v2/action_facts/target.rs`
- Modify: `src/ai/combat_search_v2/action_facts/mod.rs`
- Modify: `src/ai/combat_search_v2/action_facts/tests.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`
- Modify: `src/ai/combat_search_v2/types/report/frontier.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Modify: `src/eval/combat_search_v2/turn_plan_guidance_lab.rs`

**Interfaces:**
- Consumes: `attack_retaliation_for_target` from Task 1.
- Produces: serializable per-target retaliation facts and descriptive enemy-mechanics aggregates; no frontier value changes.

- [ ] **Step 1: Add failing action-target and aggregate tests**

In `action_facts/tests.rs`, add this complete test using the file's existing imports plus `BuffSpec` and `MonsterMoveSpec`:

```rust
#[test]
fn facts_report_attack_retaliation_on_target_and_action() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 1;
    spiker.set_planned_steps(
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Thorns,
            amount: 2,
        })
        .to_steps(),
    );
    combat.entities.monsters = vec![spiker];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Thorns,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        &EngineCombatStepper,
        250,
    );

let retaliation = facts
    .target
    .and_then(|target| target.attack_retaliation)
    .expect("Thorns target should expose attack retaliation");
assert_eq!(retaliation.power_source_count, 1);
assert_eq!(retaliation.player_hp_loss_per_damage_event, 3);
assert_eq!(retaliation.visible_growth_amount, 2);
assert_eq!(
    facts.mechanics.reactive.attack_retaliation_player_hp_loss_hint,
    3
);
}
```

In `enemy_mechanics_profile/tests.rs`, add the same Thorns-3/pending-growth-2 monster setup and require:

```rust
#[test]
fn profile_reports_attack_retaliation_aggregates() {
    let mut combat = blank_test_combat();
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 7;
    spiker.set_planned_steps(
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Thorns,
            amount: 2,
        })
        .to_steps(),
    );
    combat.entities.monsters = vec![spiker];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Thorns,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);

assert_eq!(profile.attack_retaliation_target_count, 1);
assert_eq!(profile.attack_retaliation_total_per_event, 3);
assert_eq!(profile.attack_retaliation_visible_growth_target_count, 1);
assert_eq!(profile.attack_retaliation_visible_growth_total, 2);
}
```

- [ ] **Step 2: Run diagnostic tests and verify RED**

Run:

```powershell
cargo test --lib facts_report_attack_retaliation -- --nocapture
cargo test --lib profile_reports_attack_retaliation -- --nocapture
```

Expected: compilation fails because public target and aggregate fields are absent.

- [ ] **Step 3: Add the serializable target projection**

Add to `action_facts/types.rs`:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2AttackRetaliationTargetFacts {
    pub power_source_count: usize,
    pub player_hp_loss_per_damage_event: i32,
    pub visible_growth_amount: i32,
}
```

Add to `CombatSearchV2ActionTargetFacts`:

```rust
pub attack_retaliation: Option<CombatSearchV2AttackRetaliationTargetFacts>,
```

Re-export the type from `action_facts/mod.rs` and `combat_search_v2/mod.rs`. In `target.rs`, project `attack_retaliation_for_target` without checking an enemy ID.

Add `attack_retaliation: None` to the manual target-fact fixture in `src/eval/combat_search_v2/turn_plan_guidance_lab.rs`.

- [ ] **Step 4: Add descriptive enemy-mechanics aggregates**

Add these fields to internal `EnemyMechanicsProfileV1` and public `CombatSearchV2EnemyMechanicsReport`:

```rust
attack_retaliation_target_count: usize,
attack_retaliation_total_per_event: i32,
attack_retaliation_visible_growth_target_count: usize,
attack_retaliation_visible_growth_total: i32,
```

At the start of `enemy_mechanics_profile`, collect the target facts once for living enemies and initialize the four fields from that vector. Project them in `enemy_mechanics_profile_report`. Keep `tracked_monsters` semantics and the existing policy label unchanged.

- [ ] **Step 5: Run focused diagnostic suites**

Run:

```powershell
cargo fmt --all
cargo test --lib facts_report_attack_retaliation -- --nocapture
cargo test --lib profile_reports_attack_retaliation -- --nocapture
cargo test --lib action_facts -- --nocapture
cargo test --lib enemy_mechanics_profile -- --nocapture
```

Expected: all tests pass and JSON-facing report types compile.

- [ ] **Step 6: Commit diagnostics**

```powershell
git add -- src/ai/combat_search_v2 src/eval/combat_search_v2/turn_plan_guidance_lab.rs
git commit -m "feat: report attack retaliation threats"
```

---

### Task 4: Verify repository and frozen A3F42 evidence

**Files:**
- Inspect: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/accepted_high_loss_combat/seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json`
- Create ignored diagnostic: `artifacts/runs/bounded-mainline-seed-20260712001-timed-enemy-threat/diagnostics/a3f42_lazy_all_8s_attack_retaliation.json`

**Interfaces:**
- Consumes: completed facts, shared ordering, rollout reuse, and diagnostics.
- Produces: evidence deciding whether a separate cross-turn/frontier design is required.

- [ ] **Step 1: Run complete repository verification**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: formatting is clean, 2700+ library tests pass, and all seven architecture-boundary tests pass.

- [ ] **Step 2: Rerun the frozen A3F42 search without an early HP stop**

Run:

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\accepted_high_loss_combat\seed20260712001_g39_b0039_a3f42t0_repulsor_exploder_spiker_exploder.capture.json" --max-nodes 800000 --wall-ms 8000 --potion-policy all --max-potions-used 3 --child-rollout-policy lazy-on-pop --output "artifacts\runs\bounded-mainline-seed-20260712001-timed-enemy-threat\diagnostics\a3f42_lazy_all_8s_attack_retaliation.json"
```

Record complete-win status, final HP, number of explicit damage actions aimed at the Spiker, estimated retaliation paid, first-win node, and elapsed time. Compare with the pre-change evidence: final HP 43, four explicit single-hit Spiker attacks, and roughly 50 gross Thorns damage in the accepted line.

- [ ] **Step 3: Apply the evidence boundary**

If the selected trajectory reduces Spiker attack events or improves final HP, report the exact change and stop. If facts and focused ordering are correct but the frozen line remains structurally unchanged, do not add a Spiker weight; write a separate design for cross-turn retaliation debt/setup value before modifying frontier scoring.

- [ ] **Step 4: Confirm local repository state**

Run:

```powershell
git status --short --branch
git log -6 --oneline
```

Expected: tracked working tree is clean, diagnostics remain ignored, and the design, plan, and implementation commits are present on local `master` without pushing.
