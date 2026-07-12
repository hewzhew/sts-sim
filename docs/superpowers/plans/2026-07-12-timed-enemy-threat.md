# Timed Enemy Threat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract exact enemy-owned timed forced-damage facts and use them to order otherwise comparable single-target damage toward the more urgent cancelable threat.

**Architecture:** A focused `timed_enemy_threat` module reads simulator power state and returns policy-free per-target facts. Diagnostics project those facts into public reports, while action ordering consumes three lexicographic fields; conservative rollout inherits the behavior through its existing shared ordering path. Frontier value, pruning, acceptance, and terminal scoring remain unchanged.

**Tech Stack:** Rust, Serde reports, Cargo library tests, existing combat-search capture diagnostics.

## Global Constraints

- Work in `D:\rust\sts_simulator`; do not create a worktree or dispatch subagents.
- Use `PowerId::Explosive`, not `EnemyId::Exploder` or move history, as countdown truth.
- Do not modify frontier value, pruning, dominance, transposition, acceptance, terminal scoring, run-control, or out-of-combat policy.
- Do not add the complete A3F35 result as a unit test or teacher label.
- Use test-first red/green development for each behavior change.

---

### Task 1: Add exact timed-threat facts and a shared damage constant

**Files:**
- Create: `src/ai/combat_search_v2/timed_enemy_threat.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Modify: `src/content/powers/core/explosive.rs`

**Interfaces:**
- Produces: `TimedEnemyThreatKind`, `TimedEnemyThreatV1`, `timed_enemy_threats(&CombatState) -> Vec<TimedEnemyThreatV1>`, and `timed_enemy_threat_for_target(&CombatState, EntityId) -> Option<TimedEnemyThreatV1>`.
- Produces: `content::powers::core::explosive::EXPLOSION_DAMAGE` as the one source constant used by resolution and AI facts.

- [ ] **Step 1: Add the module declaration and failing fact tests**

Add `mod timed_enemy_threat;` beside the other combat-mechanics modules in `src/ai/combat_search_v2/mod.rs`. Create `src/ai/combat_search_v2/timed_enemy_threat.rs` with only this test scaffold:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatState, Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    fn combat_with_explosive(amount: i32) -> CombatState {
        let mut combat = blank_test_combat();
        let mut owner = test_monster(EnemyId::Exploder);
        owner.id = 7;
        combat.entities.monsters = vec![owner];
        combat.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::Explosive,
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
    fn timed_enemy_threat_reads_positive_explosive_power() {
        let combat = combat_with_explosive(3);

        assert_eq!(
            timed_enemy_threats(&combat),
            vec![TimedEnemyThreatV1 {
                source_entity_id: 7,
                kind: TimedEnemyThreatKind::ForcedPlayerDamage,
                owner_turns_until_trigger: 3,
                raw_player_damage: crate::content::powers::core::explosive::EXPLOSION_DAMAGE,
                canceled_by_owner_death: true,
            }]
        );
    }

    #[test]
    fn timed_enemy_threat_tracks_urgency_and_ignores_inactive_owners() {
        let amount_one = combat_with_explosive(1);
        assert_eq!(
            timed_enemy_threat_for_target(&amount_one, 7)
                .map(|fact| fact.owner_turns_until_trigger),
            Some(1)
        );

        let powerless = combat_with_explosive(0);
        assert!(timed_enemy_threats(&powerless).is_empty());

        let mut dead = combat_with_explosive(3);
        dead.entities.monsters[0].is_dying = true;
        assert!(timed_enemy_threats(&dead).is_empty());
    }
}
```

- [ ] **Step 2: Run the fact tests and verify the red state**

Run:

```powershell
cargo test --lib timed_enemy_threat -- --nocapture
```

Expected: compilation fails because the types, extractors, and shared `EXPLOSION_DAMAGE` constant do not exist. The failure must originate from those missing interfaces, not from the test setup.

- [ ] **Step 3: Implement the shared constant and policy-free fact extractor**

In `src/content/powers/core/explosive.rs`, add and use:

```rust
pub const EXPLOSION_DAMAGE: i32 = 30;
```

Replace both hard-coded `30` fields in `DamageInfo` with `EXPLOSION_DAMAGE`.

In `src/ai/combat_search_v2/timed_enemy_threat.rs`, add above the test module:

```rust
use serde::Serialize;

use crate::content::powers::{store, PowerId};
use crate::runtime::combat::CombatState;
use crate::EntityId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TimedEnemyThreatKind {
    ForcedPlayerDamage,
}

impl TimedEnemyThreatKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ForcedPlayerDamage => "forced_player_damage",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct TimedEnemyThreatV1 {
    pub(super) source_entity_id: EntityId,
    pub(super) kind: TimedEnemyThreatKind,
    pub(super) owner_turns_until_trigger: u32,
    pub(super) raw_player_damage: i32,
    pub(super) canceled_by_owner_death: bool,
}

pub(super) fn timed_enemy_threats(combat: &CombatState) -> Vec<TimedEnemyThreatV1> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .filter_map(|monster| timed_enemy_threat_for_target(combat, monster.id))
        .collect()
}

pub(super) fn timed_enemy_threat_for_target(
    combat: &CombatState,
    entity_id: EntityId,
) -> Option<TimedEnemyThreatV1> {
    let owner = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())?;
    let amount = store::power_amount(combat, owner.id, PowerId::Explosive);
    (amount > 0).then_some(TimedEnemyThreatV1 {
        source_entity_id: owner.id,
        kind: TimedEnemyThreatKind::ForcedPlayerDamage,
        owner_turns_until_trigger: amount as u32,
        raw_player_damage: crate::content::powers::core::explosive::EXPLOSION_DAMAGE,
        canceled_by_owner_death: true,
    })
}
```

- [ ] **Step 4: Run focused power and fact tests**

Run:

```powershell
cargo test --lib timed_enemy_threat -- --nocapture
cargo test --lib explosive -- --nocapture
```

Expected: timed-threat tests pass; existing Explosive Power and Exploder mechanism tests still pass using the shared constant.

- [ ] **Step 5: Commit the exact fact layer**

Run:

```powershell
git add -- src/ai/combat_search_v2/mod.rs src/ai/combat_search_v2/timed_enemy_threat.rs src/content/powers/core/explosive.rs
git commit -m "feat: expose timed enemy threat facts"
```

---

### Task 2: Project timed threats into diagnostics

**Files:**
- Modify: `src/ai/combat_search_v2/action_facts/types.rs`
- Modify: `src/ai/combat_search_v2/action_facts/target.rs`
- Modify: `src/ai/combat_search_v2/action_facts/mod.rs`
- Modify: `src/ai/combat_search_v2/action_facts/tests.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`
- Modify: `src/ai/combat_search_v2/types/report/frontier.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`

**Interfaces:**
- Consumes: `timed_enemy_threat_for_target` and `timed_enemy_threats` from Task 1.
- Produces: public serializable `CombatSearchV2TimedEnemyThreatTargetFacts` and three aggregate enemy-mechanics report fields.

- [ ] **Step 1: Add failing action-fact and profile tests**

In `action_facts/tests.rs`, add:

```rust
#[test]
fn facts_report_timed_enemy_threat_on_target() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let mut exploder = test_monster(EnemyId::Exploder);
    exploder.id = 1;
    exploder.current_hp = 30;
    exploder.max_hp = 30;
    combat.entities.monsters = vec![exploder];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Explosive,
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

let threat = facts
    .target
    .and_then(|target| target.timed_enemy_threat)
    .expect("Explosive target should expose a timed threat");
assert_eq!(threat.kind, "forced_player_damage");
assert_eq!(threat.owner_turns_until_trigger, 3);
assert_eq!(threat.raw_player_damage, 30);
assert!(threat.canceled_by_owner_death);
}
```

In `enemy_mechanics_profile/tests.rs`, add:

```rust
#[test]
fn profile_reports_timed_enemy_threat_aggregates() {
    let mut combat = blank_test_combat();
    let mut exploder = test_monster(EnemyId::Exploder);
    exploder.id = 7;
    combat.entities.monsters = vec![exploder];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Explosive,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);
    let report = enemy_mechanics_profile_report(profile);

    assert_eq!(profile.timed_threat_count, 1);
    assert_eq!(profile.timed_threat_min_owner_turns, Some(3));
    assert_eq!(profile.timed_threat_total_raw_damage, 30);
    assert_eq!(report.profiling_policy, "typed_enemy_mechanics_fact_profile_no_direct_score");
}
```

- [ ] **Step 2: Run diagnostic tests and verify the red state**

Run:

```powershell
cargo test --lib facts_report_timed_enemy_threat -- --nocapture
cargo test --lib profile_reports_timed_enemy_threat -- --nocapture
```

Expected: compilation fails because the report types and aggregate fields are absent.

- [ ] **Step 3: Add public action-target projection**

Add this public report type in `action_facts/types.rs` and re-export it from `action_facts/mod.rs` and the existing `pub use action_facts` block in `combat_search_v2/mod.rs`:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TimedEnemyThreatTargetFacts {
    pub kind: &'static str,
    pub owner_turns_until_trigger: u32,
    pub raw_player_damage: i32,
    pub canceled_by_owner_death: bool,
}
```

Add to `CombatSearchV2ActionTargetFacts`:

```rust
pub timed_enemy_threat: Option<CombatSearchV2TimedEnemyThreatTargetFacts>,
```

In `action_facts/target.rs`, import `timed_enemy_threat_for_target` and `CombatSearchV2TimedEnemyThreatTargetFacts`, then populate:

```rust
timed_enemy_threat: timed_enemy_threat_for_target(combat, monster.id).map(|threat| {
    CombatSearchV2TimedEnemyThreatTargetFacts {
        kind: threat.kind.label(),
        owner_turns_until_trigger: threat.owner_turns_until_trigger,
        raw_player_damage: threat.raw_player_damage,
        canceled_by_owner_death: threat.canceled_by_owner_death,
    }
}),
```

Do not inspect `EnemyId::Exploder` in this module.

- [ ] **Step 4: Add aggregate profile fields without scoring them**

Add these fields to both `EnemyMechanicsProfileV1` and `CombatSearchV2EnemyMechanicsReport`:

```rust
timed_threat_count: usize,
timed_threat_min_owner_turns: Option<u32>,
timed_threat_total_raw_damage: i32,
```

At the start of `enemy_mechanics_profile`, call `timed_enemy_threats(combat)` once and initialize:

```rust
let timed_threats = timed_enemy_threats(combat);
let mut profile = EnemyMechanicsProfileV1 {
    timed_threat_count: timed_threats.len(),
    timed_threat_min_owner_turns: timed_threats
        .iter()
        .map(|threat| threat.owner_turns_until_trigger)
        .min(),
    timed_threat_total_raw_damage: timed_threats
        .iter()
        .map(|threat| threat.raw_player_damage)
        .sum(),
    ..EnemyMechanicsProfileV1::default()
};
```

Keep `tracked_monsters` semantics unchanged. In `enemy_mechanics_profile_report`, project the three fields and change the policy label to:

```rust
"typed_enemy_mechanics_fact_profile_no_direct_score"
```

- [ ] **Step 5: Run diagnostic tests and inspect serialized facts**

Run:

```powershell
cargo test --lib facts_report_timed_enemy_threat -- --nocapture
cargo test --lib profile_reports_timed_enemy_threat -- --nocapture
cargo test --lib action_facts -- --nocapture
cargo test --lib enemy_mechanics_profile -- --nocapture
```

Expected: all commands pass. Existing reports compile with the additional optional/aggregate fields and no value or policy consumer has changed.

---

### Task 3: Order comparable damage toward timed threats

**Files:**
- Modify: `src/ai/combat_search_v2/action_priority/priority.rs`
- Modify: `src/ai/combat_search_v2/action_priority/play_card/mod.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/tests/core.rs`
- Modify: `src/ai/combat_search_v2/rollout_action_selector/tests.rs`

**Interfaces:**
- Consumes: `timed_enemy_threat_for_target` from Task 1.
- Produces: `targets_timed_threat`, `timed_threat_urgency`, and `timed_threat_raw_damage` lexicographic ordering fields.
- Preserves: all existing role ranks and immediate survival ordering.

- [ ] **Step 1: Add failing priority and ordering tests**

In `action_priority/tests.rs`, extend the power imports and add this helper:

```rust
fn add_explosive(
    combat: &mut crate::runtime::combat::CombatState,
    entity_id: usize,
    amount: i32,
) {
    combat
        .entities
        .power_db
        .entry(entity_id)
        .or_default()
        .push(crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Explosive,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        });
}
```

Add the equivalent-target test:

```rust
#[test]
fn timed_threat_damage_progress_orders_before_neutral_target() {
    let mut combat = blank_test_combat();
    let mut neutral = test_monster(EnemyId::Repulsor);
    neutral.id = 1;
    neutral.current_hp = 40;
    neutral.max_hp = 40;
    let mut timed = test_monster(EnemyId::Exploder);
    timed.id = 2;
    timed.current_hp = 40;
    timed.max_hp = 40;
    combat.entities.monsters = vec![neutral, timed];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    add_explosive(&mut combat, 2, 3);

    let neutral = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let timed = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

assert_eq!(neutral.targets_timed_threat, 0);
assert_eq!(timed.targets_timed_threat, 1);
assert_eq!(timed.timed_threat_urgency, -3);
assert_eq!(timed.timed_threat_raw_damage, 30);
assert!(timed > neutral);
}
```

Add the urgency and no-power tests:

```rust
#[test]
fn timed_threat_damage_progress_prefers_shorter_countdown() {
    let mut combat = blank_test_combat();
    let mut slower = test_monster(EnemyId::Exploder);
    slower.id = 1;
    slower.current_hp = 40;
    slower.max_hp = 40;
    let mut sooner = test_monster(EnemyId::Exploder);
    sooner.id = 2;
    sooner.current_hp = 40;
    sooner.max_hp = 40;
    combat.entities.monsters = vec![slower, sooner];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    add_explosive(&mut combat, 1, 3);
    add_explosive(&mut combat, 2, 1);

    let slower = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let sooner = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(slower.timed_threat_urgency, -3);
    assert_eq!(sooner.timed_threat_urgency, -1);
    assert!(sooner > slower);
}

#[test]
fn timed_threat_damage_progress_is_neutral_without_power() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::Repulsor);
    first.id = 1;
    first.current_hp = 40;
    first.max_hp = 40;
    let mut second = test_monster(EnemyId::Exploder);
    second.id = 2;
    second.current_hp = 40;
    second.max_hp = 40;
    combat.entities.monsters = vec![first, second];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let first = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let second = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(first.targets_timed_threat, 0);
    assert_eq!(second.targets_timed_threat, 0);
    assert_eq!(first.timed_threat_urgency, 0);
    assert_eq!(second.timed_threat_urgency, 0);
    assert_eq!(first.timed_threat_raw_damage, 0);
    assert_eq!(second.timed_threat_raw_damage, 0);
    assert_eq!(first, second);
}
```

In `action_ordering/tests/core.rs`, add:

```rust
#[test]
fn visible_lethal_still_precedes_timed_threat_damage_progress() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 5;
    let mut exploder = test_monster(EnemyId::Exploder);
    exploder.id = 1;
    exploder.current_hp = 30;
    exploder.max_hp = 30;
    exploder.set_planned_move_id(1);
    combat.entities.monsters = vec![exploder];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Explosive,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
    ];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
        ),
    ];

    let ordered = order_action_choices(&EngineState::CombatPlayerTurn, &combat, choices);

    assert_eq!(ordered.choices[0].original_action_id, 1);
}
```

This proves the timed key does not become a new role rank.

- [ ] **Step 2: Run priority tests and verify the red state**

Run:

```powershell
cargo test --lib timed_threat_damage_progress -- --nocapture
cargo test --lib visible_lethal_still_precedes_timed_threat -- --nocapture
```

Expected: compilation fails because the three priority fields do not exist.

- [ ] **Step 3: Implement lexicographic ordering fields**

Add the three `i32` fields to `ActionOrderingPriority`, initialize them to zero in `neutral`, and compare them after `reactive_risk` and `collector_tactic` but before `phase_setup`:

```rust
.then_with(|| self.targets_timed_threat.cmp(&other.targets_timed_threat))
.then_with(|| self.timed_threat_urgency.cmp(&other.timed_threat_urgency))
.then_with(|| self.timed_threat_raw_damage.cmp(&other.timed_threat_raw_damage))
```

In `priority_for_play_card`, query the target fact only when `target_progress > 0` and `target` contains one specific entity:

```rust
let timed_threat = (target_progress > 0)
    .then(|| target.and_then(|entity_id| timed_enemy_threat_for_target(combat, entity_id)))
    .flatten();
```

Populate:

```rust
targets_timed_threat: i32::from(timed_threat.is_some_and(|fact| fact.canceled_by_owner_death)),
timed_threat_urgency: timed_threat
    .map(|fact| -(fact.owner_turns_until_trigger.min(i32::MAX as u32) as i32))
    .unwrap_or_default(),
timed_threat_raw_damage: timed_threat
    .map(|fact| fact.raw_player_damage)
    .unwrap_or_default(),
```

Do not change `ActionOrderingRole`, any role-rank constant, or target-progress calculation.

- [ ] **Step 4: Verify action ordering is green**

Run:

```powershell
cargo test --lib timed_threat_damage_progress -- --nocapture
cargo test --lib visible_lethal_still_precedes_timed_threat -- --nocapture
cargo test --lib action_priority -- --nocapture
cargo test --lib action_ordering -- --nocapture
```

Expected: timed-target and urgency tests pass; all existing ordering tests remain green.

- [ ] **Step 5: Add and run the rollout reuse test**

In `rollout_action_selector/tests.rs`, import `CardId`, `PowerId`, `CombatCard`, `Power`, and `PowerPayload`, then add:

```rust
#[test]
fn conservative_rollout_reuses_timed_threat_ordering() {
    let mut combat = blank_test_combat();
    let mut neutral = test_monster(EnemyId::Repulsor);
    neutral.id = 1;
    neutral.current_hp = 40;
    neutral.max_hp = 40;
    let mut timed = test_monster(EnemyId::Exploder);
    timed.id = 2;
    timed.current_hp = 40;
    timed.max_hp = 40;
    combat.entities.monsters = vec![neutral, timed];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    combat.entities.power_db.insert(
        2,
        vec![Power {
            power_type: PowerId::Explosive,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    let legal = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
        ),
    ];

    let selection = choose_rollout_action(
        CombatSearchRolloutPluginId::ConservativeNoPotion,
        &test_node(combat.clone()),
        &ProbeWinStepper,
        &test_config(),
        None,
        &EngineState::CombatPlayerTurn,
        &combat,
        legal,
        &mut RolloutPerformanceCounters::default(),
    )
    .expect("rollout should select an action");

    assert!(matches!(
        selection.choice.choice.input,
        ClientInput::PlayCard {
            target: Some(2),
            ..
        }
    ));
}
```

Add no rollout-specific threat score.

Run:

```powershell
cargo test --lib conservative_rollout_reuses_timed_threat_ordering -- --nocapture
```

Expected: the rollout selects the Exploder-target action through shared ordering.

- [ ] **Step 6: Commit the diagnostic and ordering consumer**

Run:

```powershell
git add -- src/ai/combat_search_v2
git commit -m "feat: prioritize timed enemy threat targets"
```

---

### Task 4: Verify the frozen case and repository

**Files:**
- Inspect: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json`
- Create ignored diagnostic: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_immediate_800k_8s_timed_enemy_threat.json`
- Create ignored diagnostic: `artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_root_guidance_maxloss40_timed_enemy_threat.json`

**Interfaces:**
- Consumes: the completed first-stage fact and ordering implementation.
- Produces: unforced evidence deciding whether a separate frontier-stage design is necessary.

- [ ] **Step 1: Run complete repository verification**

Run:

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both commands exit successfully with zero failed tests.

- [ ] **Step 2: Run an unforced eight-second A3F35 search**

Run:

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json --max-nodes 800000 --wall-ms 8000 --child-rollout-policy immediate --output artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_immediate_800k_8s_timed_enemy_threat.json
```

Record complete-win status, final HP, HP loss, nodes-to-first-win, and the first two turns of target actions. Success requires an unforced kill-before-trigger line; it does not require a fixed final-HP threshold.

- [ ] **Step 3: Rerun the bounded root guidance lab**

Run:

```powershell
cargo run --profile fast-run --quiet --bin combat_search_v2_driver -- --combat-snapshot artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/accepted_high_loss_combat/seed20260712001_g28_b0028_a3f35t0_spiker_spiker_repulsor_exploder.capture.json --max-nodes 50000 --wall-ms 1000 --max-hp-loss 40 --guidance-lab --probe-wall-ms 2000 --output artifacts/runs/bounded-mainline-seed-20260712001-boss-potion-rescue/diagnostics/a3f35_root_guidance_maxloss40_timed_enemy_threat.json
```

Verify Writhe still reports zero energy delta and inspect whether child searches maintain Exploder target progress. Do not compare against the pre-fix Writhe result.

- [ ] **Step 4: Decide the frontier-stage boundary from evidence**

If the unforced trajectory kills Exploder before its trigger, stop: the first-stage design achieved its goal and frontier remains unchanged. If action ordering starts on Exploder but the frontier later abandons it, report that exact divergence and write a separate frontier-stage design before changing value. If ordering never starts on Exploder, return to the focused ordering tests rather than adding frontier weights.

- [ ] **Step 5: Confirm a clean committed workspace**

Run:

```powershell
git status --short --branch
git log -4 --oneline
```

Expected: only ignored diagnostic artifacts are outside Git; the working tree is clean and the two code commits follow the design and plan commits.
