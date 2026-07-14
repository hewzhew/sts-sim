# Awakened One Strength-Transition Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teach combat-search ordering to notice the bounded opportunity “temporary Strength loss, then first-phase lethal” while leaving every legal action and all exact combat outcomes to the existing engine.

**Architecture:** Extend the read-only enemy profile with minimal Awakened One form-one facts. A focused `enemy_phase_transition` helper combines those facts with existing temporary-Strength-down action effects and an energy-bounded remaining-hand damage capacity; `phase_action_ordering` consumes the typed opportunity only as an ordering bonus and exposes it in diagnostics.

**Tech Stack:** Rust 2021, existing combat-search V2 typed facts, serde diagnostics, Cargo library tests, `architecture_runtime_boundaries`.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator` on the existing local feature branch; do not create a worktree.
- Never run `cargo clean`.
- Use focused red/green tests and run the full library plus `architecture_runtime_boundaries` only at completion.
- Do not special-case `CardId::DarkShackles` in ordering. Use the existing action-effect fact `temporary_enemy_strength_down > 0` as a generic gate, then recover the raw paired Strength/Shackled amount from the resolved actions.
- The opportunity is ordering/diagnostic evidence only: no pruning, no merge, no terminal claim, no combat-state mutation, and no nested search.
- The opportunity exists only for a targetable form-one Awakened One with positive Strength and enough representable remaining-hand damage capacity after paying/removing the setup card.
- Existing persistent Strength loss reduces the current positive Strength and therefore the convertible amount; it never acts as a binary veto.
- No bonus in form two, at zero positive Strength, for persistent-only Strength loss, or when remaining damage capacity is below HP plus block.
- Increment the combat-search diagnostics schema when adding serialized transition-window fields.

---

## File Map

- Modify `src/ai/combat_search_v2/enemy_mechanics_profile.rs`: expose targetable form-one entity, HP plus block, and positive Strength.
- Modify `src/ai/combat_search_v2/types/report/frontier.rs`: serialize those enemy profile facts.
- Create `src/ai/combat_search_v2/enemy_phase_transition/awakened_one.rs`: own the typed opportunity and bounded remaining-hand damage calculation.
- Modify `src/ai/combat_search_v2/enemy_phase_transition.rs`: export the opportunity and attach it to `EnemyPhaseTransitionHint`.
- Modify `src/ai/combat_search_v2/action_priority/play_card/mod.rs`: pass existing action-effect facts into the transition detector.
- Modify `src/ai/combat_search_v2/phase_action_ordering/constants.rs`, `rules.rs`, and `types.rs`: apply and retain a bounded setup bonus.
- Modify `src/ai/combat_search_v2/action_ordering/types.rs`, `summary.rs`, `diagnostics/samples.rs`, `diagnostics/collector.rs`, and `diagnostics/report.rs`: retain phase hints in action samples.
- Modify `src/ai/combat_search_v2/types/diagnostics/action.rs` and `src/ai/combat_search_v2/diagnostics/finish.rs`: expose the opportunity and bump schema 13 to 14.
- Modify `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`, `enemy_phase_transition/tests.rs`, `action_priority/tests.rs`, and `action_ordering/tests/diagnostics.rs`: focused regressions.
- Modify `src/content/monsters/beyond/awakened_one.rs`: strengthen the exact engine phase-transition regression.

### Task 1: Expose Minimal Form-One Runtime Facts

**Files:**
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile.rs`
- Modify: `src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs`
- Modify: `src/ai/combat_search_v2/types/report/frontier.rs`

**Interfaces:**
- Produces: `EnemyMechanicsProfileV1::{awakened_one_form_one_target, awakened_one_form_one_hp_with_block, awakened_one_positive_strength}`.
- Preserves: `awakened_one_curiosity_count` for the existing power-play penalty.

- [ ] **Step 1: Write the failing profile test**

```rust
#[test]
fn awakened_one_profile_reports_targetable_form_one_transition_facts() {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = 23;
    awakened.block = 4;
    awakened.awakened_one.form1 = true;
    combat.entities.monsters = vec![awakened];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 6,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let profile = enemy_mechanics_profile(&combat);
    let report = enemy_mechanics_profile_report(profile);

    assert_eq!(profile.awakened_one_form_one_target, Some(7));
    assert_eq!(profile.awakened_one_form_one_hp_with_block, Some(27));
    assert_eq!(profile.awakened_one_positive_strength, Some(6));
    assert_eq!(report.awakened_one_form_one_target, Some(7));
}
```

Add the form-two negative beside it:

```rust
#[test]
fn awakened_one_profile_hides_transition_facts_in_form_two() {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = 23;
    awakened.block = 4;
    awakened.awakened_one.form1 = false;
    combat.entities.monsters = vec![awakened];

    let profile = enemy_mechanics_profile(&combat);

    assert_eq!(profile.awakened_one_form_one_target, None);
    assert_eq!(profile.awakened_one_form_one_hp_with_block, None);
    assert_eq!(profile.awakened_one_positive_strength, None);
}
```

- [ ] **Step 2: Run the tests and verify the red state**

```powershell
cargo test --lib awakened_one_profile_reports_targetable_form_one_transition_facts -- --nocapture
```

Expected: compilation fails because the profile/report fields do not exist.

- [ ] **Step 3: Implement the factual profile fields**

Add these optional fields to the internal profile:

```rust
pub(super) awakened_one_form_one_target: Option<usize>,
pub(super) awakened_one_form_one_hp_with_block: Option<i32>,
pub(super) awakened_one_positive_strength: Option<i32>,
```

Add the public serialized counterparts to `CombatSearchV2EnemyMechanicsReport`:

```rust
pub awakened_one_form_one_target: Option<usize>,
pub awakened_one_form_one_hp_with_block: Option<i32>,
pub awakened_one_positive_strength: Option<i32>,
```

Populate them only inside the already-targetable monster loop:

```rust
EnemyId::AwakenedOne => {
    profile.tracked_monsters += 1;
    if store::has_power(combat, monster.id, PowerId::Curiosity) {
        profile.awakened_one_curiosity_count += 1;
    }
    if monster.awakened_one.form1 {
        profile.awakened_one_form_one_target = Some(monster.id);
        profile.awakened_one_form_one_hp_with_block =
            Some(monster.current_hp.saturating_add(monster.block));
        profile.awakened_one_positive_strength = Some(
            store::power_amount(combat, monster.id, PowerId::Strength).max(0),
        );
    }
}
```

Copy the three fields in `enemy_mechanics_profile_report`:

```rust
awakened_one_form_one_target: profile.awakened_one_form_one_target,
awakened_one_form_one_hp_with_block: profile.awakened_one_form_one_hp_with_block,
awakened_one_positive_strength: profile.awakened_one_positive_strength,
```

Do not infer policy or a score here.

- [ ] **Step 4: Run focused profile tests**

```powershell
cargo test --lib enemy_mechanics_profile -- --nocapture
```

Expected: all enemy-profile tests pass, including the form-two negative case.

- [ ] **Step 5: Commit the profile boundary**

```powershell
git add src/ai/combat_search_v2/enemy_mechanics_profile.rs src/ai/combat_search_v2/enemy_mechanics_profile/tests.rs src/ai/combat_search_v2/types/report/frontier.rs
git commit -m "feat: expose awakened one form facts"
```

### Task 2: Compute a Typed, Energy-Bounded Transition Opportunity

**Files:**
- Create: `src/ai/combat_search_v2/enemy_phase_transition/awakened_one.rs`
- Modify: `src/ai/combat_search_v2/enemy_phase_transition.rs`
- Modify: `src/ai/combat_search_v2/enemy_phase_transition/tests.rs`

**Interfaces:**
- Consumes: `EnemyMechanicsProfileV1` and `CardPlayEffectFacts`.
- Produces: `AwakenedOneStrengthTransitionOpportunity`.
- Produces: `awakened_one_strength_transition_opportunity(combat, card_index, target, profile, effects) -> Option<AwakenedOneStrengthTransitionOpportunity>`.

- [ ] **Step 1: Write failing positive, Disarm-interaction, and negative tests**

Add this local fixture before the tests:

```rust
fn awakened_transition_test_combat(
    hp: i32,
    strength: i32,
) -> crate::runtime::combat::CombatState {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = hp;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = true;
    combat.entities.monsters = vec![awakened];
    combat
        .entities
        .power_db
        .insert(7, vec![test_power(PowerId::Strength, strength)]);
    combat
}
```

Then add:

```rust
#[test]
fn temporary_strength_down_reports_reachable_form_one_lethal_window() {
    let mut combat = awakened_transition_test_combat(12, 4);
    combat.turn.energy = 2;
    combat.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Carnage, 11),
    ];
    let effects = super::super::action_effects::card_play_effect_facts(
        &combat,
        &combat.zones.hand[0],
        Some(7),
    );

    let opportunity = awakened_one_strength_transition_opportunity(
        &combat,
        0,
        Some(7),
        enemy_mechanics_profile(&combat),
        effects,
    )
    .expect("reachable transition window");

    assert_eq!(opportunity.temporary_strength_down, 9);
    assert_eq!(opportunity.convertible_positive_strength, 4);
    assert!(opportunity.remaining_damage_upper_bound >= 12);
    assert_eq!(opportunity.phase_one_hp_with_block, 12);
}

#[test]
fn persistent_strength_loss_reduces_conversion_without_closing_window() {
    let mut combat = awakened_transition_test_combat(12, 2);
    combat.turn.energy = 2;
    combat.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Carnage, 11),
    ];
    let effects = super::super::action_effects::card_play_effect_facts(
        &combat,
        &combat.zones.hand[0],
        Some(7),
    );

    let opportunity = awakened_one_strength_transition_opportunity(
        &combat,
        0,
        Some(7),
        enemy_mechanics_profile(&combat),
        effects,
    )
    .expect("Disarm-reduced positive Strength still leaves a transition window");

    assert_eq!(opportunity.convertible_positive_strength, 2);
}
```

Add the four hard-gate negatives without card-name policy:

```rust
#[test]
fn transition_opportunity_requires_form_one_strength_temporary_loss_and_damage() {
    let mut form_two = awakened_transition_test_combat(6, 4);
    form_two.entities.monsters[0].awakened_one.form1 = false;
    form_two.turn.energy = 2;
    form_two.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Carnage, 11),
    ];

    let mut zero_strength = awakened_transition_test_combat(6, 0);
    zero_strength.turn.energy = 2;
    zero_strength.zones.hand = form_two.zones.hand.clone();

    let mut insufficient = awakened_transition_test_combat(40, 4);
    insufficient.turn.energy = 2;
    insufficient.zones.hand = form_two.zones.hand.clone();

    let mut persistent_only = awakened_transition_test_combat(6, 4);
    persistent_only.turn.energy = 2;
    persistent_only.zones.hand = vec![
        CombatCard::new(CardId::Disarm, 10),
        CombatCard::new(CardId::Carnage, 11),
    ];

    for (label, combat) in [
        ("form-two", form_two),
        ("zero-strength", zero_strength),
        ("insufficient-damage", insufficient),
        ("persistent-only", persistent_only),
    ] {
        let effects = super::super::action_effects::card_play_effect_facts(
            &combat,
            &combat.zones.hand[0],
            Some(7),
        );
        assert!(
            awakened_one_strength_transition_opportunity(
                &combat,
                0,
                Some(7),
                enemy_mechanics_profile(&combat),
                effects,
            )
            .is_none(),
            "unexpected opportunity for {label}"
        );
    }
}
```

- [ ] **Step 2: Run the tests and verify the red state**

```powershell
cargo test --lib temporary_strength_down_reports_reachable_form_one_lethal_window -- --nocapture
cargo test --lib persistent_strength_loss_reduces_conversion_without_closing_window -- --nocapture
```

Expected: compilation fails because the opportunity interface does not exist.

- [ ] **Step 3: Define the opportunity and hard gates**

Create `enemy_phase_transition/awakened_one.rs` with this public-to-module contract:

```rust
use super::{damage_projection, PhaseProjection};
use super::super::action_effects::CardPlayEffectFacts;
use super::super::enemy_mechanics_profile::EnemyMechanicsProfileV1;
use crate::content::cards::{self, CardTarget, CardType};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct AwakenedOneStrengthTransitionOpportunity {
    pub(in crate::ai::combat_search_v2) temporary_strength_down: i32,
    pub(in crate::ai::combat_search_v2) convertible_positive_strength: i32,
    pub(in crate::ai::combat_search_v2) remaining_damage_upper_bound: i32,
    pub(in crate::ai::combat_search_v2) phase_one_hp_with_block: i32,
}

pub(in crate::ai::combat_search_v2) fn awakened_one_strength_transition_opportunity(
    combat: &CombatState,
    setup_card_index: usize,
    target: Option<usize>,
    profile: EnemyMechanicsProfileV1,
    effects: CardPlayEffectFacts,
) -> Option<AwakenedOneStrengthTransitionOpportunity> {
    let target_id = profile.awakened_one_form_one_target?;
    let hp_with_block = profile.awakened_one_form_one_hp_with_block?;
    let positive_strength = profile.awakened_one_positive_strength?.max(0);
    let weighted_temporary_strength_down =
        effects.direct.temporary_enemy_strength_down.max(0);
    if target != Some(target_id)
        || positive_strength == 0
        || weighted_temporary_strength_down == 0
    {
        return None;
    }
    let temporary_strength_down =
        raw_temporary_strength_down_for_target(combat, setup_card_index, target_id);
    if temporary_strength_down == 0 {
        return None;
    }

    let remaining_damage_upper_bound = remaining_hand_damage_upper_bound(
        combat,
        setup_card_index,
        target_id,
    );
    if remaining_damage_upper_bound < hp_with_block {
        return None;
    }

    Some(AwakenedOneStrengthTransitionOpportunity {
        temporary_strength_down,
        convertible_positive_strength: temporary_strength_down.min(positive_strength),
        remaining_damage_upper_bound,
        phase_one_hp_with_block: hp_with_block,
    })
}
```

The existing action-effect fact is weighted by attack relevance, so it is only the cheap generic gate above. Add this helper in the same file to recover the raw applied amount without checking a card ID:

```rust
fn raw_temporary_strength_down_for_target(
    combat: &CombatState,
    card_index: usize,
    target_id: usize,
) -> i32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0;
    };
    let mut candidate = card.clone();
    if candidate.cost_for_turn_java() < 0 {
        candidate.energy_on_use = i32::from(combat.turn.energy);
    }
    let evaluated = cards::evaluate_card_for_play(&candidate, combat, Some(target_id));
    let actions = cards::resolve_card_play_with_context(
        evaluated.id,
        combat,
        &evaluated,
        Some(target_id),
        cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut strength_loss = 0i32;
    let mut shackled = 0i32;
    for info in actions {
        match info.action {
            Action::ApplyPower {
                target,
                power_id,
                amount,
                ..
            }
            | Action::ApplyPowerDetailed {
                target,
                power_id,
                amount,
                ..
            }
            | Action::ApplyPowerWithPayload {
                target,
                power_id,
                amount,
                ..
            } if target == target_id => match power_id {
                PowerId::Strength if amount < 0 => {
                    strength_loss = strength_loss.saturating_add(amount.saturating_neg());
                }
                PowerId::Shackled if amount > 0 => {
                    shackled = shackled.saturating_add(amount);
                }
                _ => {}
            },
            _ => {}
        }
    }
    strength_loss.min(shackled)
}
```

Resolving the candidate once here is permitted because it is bounded to candidates that already passed the temporary-Strength action-effect gate; it does not recurse or mutate combat state.

Export the module contract and add the optional field to `EnemyPhaseTransitionHint`:

```rust
mod awakened_one;
pub(super) use awakened_one::{
    awakened_one_strength_transition_opportunity,
    AwakenedOneStrengthTransitionOpportunity,
};

pub(super) struct EnemyPhaseTransitionHint {
    pub(super) split_trigger_count: usize,
    pub(super) split_debt_hp: i32,
    pub(super) guardian_mode_shift_trigger_count: usize,
    pub(super) guardian_min_threshold_remaining_before_hit: Option<i32>,
    pub(super) lagavulin_wake_risk_count: usize,
    pub(super) champ_threshold_trigger_count: usize,
    pub(super) champ_threshold_debt_hp: i32,
    pub(super) awakened_one_strength_transition:
        Option<AwakenedOneStrengthTransitionOpportunity>,
}
```

- [ ] **Step 4: Implement the bounded remaining-hand capacity**

The helper must clone a post-setup view, remove the setup card, subtract its non-negative cost, and increment `cards_played_this_turn`. For every remaining legal attack, freshly evaluate damage against the Awakened One and feed `(cost, damage)` into a 0/1 energy knapsack. For X-cost attacks, evaluate one mutually exclusive option for each spend `0..=remaining_energy` from the same pre-item DP row; spend zero preserves effects such as Chemical X while still consuming zero energy.

Use this shape:

```rust
fn remaining_hand_damage_upper_bound(
    combat: &CombatState,
    setup_card_index: usize,
    target_id: usize,
) -> i32 {
    let Some(setup) = combat.zones.hand.get(setup_card_index) else {
        return 0;
    };
    let available = i32::from(combat.turn.energy);
    if cards::can_play_card(setup, combat).is_err() {
        return 0;
    }
    let setup_cost = if setup.cost_for_turn_java() < 0 {
        available
    } else {
        setup.cost_for_turn_java().max(0)
    };
    if setup_cost > available {
        return 0;
    }

    let mut post_setup = combat.clone();
    post_setup.zones.hand.remove(setup_card_index);
    post_setup.turn.energy = (available - setup_cost) as u8;
    post_setup.turn.counters.cards_played_this_turn = post_setup
        .turn
        .counters
        .cards_played_this_turn
        .saturating_add(1);

    let energy = usize::from(post_setup.turn.energy);
    let mut best = vec![0i32; energy + 1];
    for card in &post_setup.zones.hand {
        if cards::get_card_definition(card.id).card_type != CardType::Attack
            || cards::can_play_card_ignoring_energy(card, &post_setup).is_err()
        {
            continue;
        }
        let before = best.clone();
        if card.cost_for_turn_java() < 0 {
            for spend in 0..=energy {
                let damage = projected_attack_damage(
                    &post_setup,
                    card,
                    target_id,
                    spend as i32,
                );
                for budget in spend..=energy {
                    best[budget] = best[budget]
                        .max(before[budget - spend].saturating_add(damage));
                }
            }
        } else {
            let cost = card.cost_for_turn_java().max(0) as usize;
            if cost > energy {
                continue;
            }
            let damage = projected_attack_damage(&post_setup, card, target_id, 0);
            for budget in cost..=energy {
                best[budget] = best[budget]
                    .max(before[budget - cost].saturating_add(damage));
            }
        }
    }
    best[energy]
}
```

Add the target-only projection helper; it sets `energy_on_use` before fresh evaluation, uses the real card actions, and delegates damage interpretation to the existing projection:

```rust
fn projected_attack_damage(
    combat: &CombatState,
    card: &CombatCard,
    target_id: usize,
    energy_on_use: i32,
) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target_id && monster.is_alive_for_action())
    else {
        return 0;
    };
    let before_effort = monster.current_hp.saturating_add(monster.block);
    let mut candidate = card.clone();
    if candidate.cost_for_turn_java() < 0 {
        candidate.energy_on_use = energy_on_use;
    }
    let target = match cards::effective_target(&candidate) {
        CardTarget::Enemy | CardTarget::SelfAndEnemy => Some(target_id),
        CardTarget::AllEnemy | CardTarget::All => None,
        CardTarget::SelfTarget | CardTarget::None => return 0,
    };
    let evaluated = cards::evaluate_card_for_play(&candidate, combat, target);
    let actions = cards::resolve_card_play_with_context(
        evaluated.id,
        combat,
        &evaluated,
        target,
        cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut projection = PhaseProjection::from_combat(combat);
    damage_projection::observe_actions_damage(
        &mut projection,
        actions.into_iter().map(|info| info.action),
    );
    let Some(projected) = projection.monsters.get(&target_id) else {
        return 0;
    };
    before_effort.saturating_sub(
        projected
            .projected_hp
            .saturating_add(projected.projected_block),
    )
}
```

Import `CardTarget` with `CardType`. This counts projected HP loss plus absorbed block. It is a bounded representable capacity, not a recursive search or a terminal proof.

- [ ] **Step 5: Run all transition detector tests**

```powershell
cargo test --lib enemy_phase_transition -- --nocapture
```

Expected: positive, Disarm-complementarity, and all negative cases pass; existing split/Guardian/Lagavulin/Champ tests remain green.

- [ ] **Step 6: Commit the opportunity detector**

```powershell
git add src/ai/combat_search_v2/enemy_phase_transition.rs src/ai/combat_search_v2/enemy_phase_transition
git commit -m "feat: detect awakened strength transition windows"
```

### Task 3: Apply the Ordering Hint and Expose Diagnostics

**Files:**
- Modify: `src/ai/combat_search_v2/action_priority/play_card/mod.rs`
- Modify: `src/ai/combat_search_v2/phase_action_ordering/constants.rs`
- Modify: `src/ai/combat_search_v2/phase_action_ordering/rules.rs`
- Modify: `src/ai/combat_search_v2/phase_action_ordering/types.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/types.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/summary.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/samples.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/collector.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/diagnostics/report.rs`
- Modify: `src/ai/combat_search_v2/action_ordering/tests/diagnostics.rs`
- Modify: `src/ai/combat_search_v2/types/diagnostics/action.rs`
- Modify: `src/ai/combat_search_v2/diagnostics/finish.rs`

**Interfaces:**
- Consumes: `EnemyPhaseTransitionHint::awakened_one_strength_transition`.
- Produces: positive `PhaseActionOrderingHint::awakened_one_strength_transition_setup` diagnostics and a bounded role adjustment.
- Produces: serialized per-action transition-window diagnostic fields under schema version 14.

- [ ] **Step 1: Write the failing ordering test**

Add this fixture and priority wrapper in `action_priority/tests.rs`:

```rust
fn awakened_transition_ordering_combat(
    hp: i32,
    strength: i32,
    form_one: bool,
) -> crate::runtime::combat::CombatState {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = hp;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = form_one;
    combat.entities.monsters = vec![awakened];
    combat.entities.power_db.insert(
        7,
        vec![crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Strength,
            instance_id: None,
            amount: strength,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        }],
    );
    combat
}

fn transition_priority(
    combat: &crate::runtime::combat::CombatState,
    card_index: usize,
) -> ActionOrderingPriority {
    priority_for_input(
        &EngineState::CombatPlayerTurn,
        combat,
        &ClientInput::PlayCard {
            card_index,
            target: Some(7),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    )
}
```

Build form one at Strike-lethal HP with positive Strength and put Dark Shackles before Strike in the hand:

```rust
#[test]
fn awakened_transition_setup_orders_before_direct_form_one_lethal() {
    let mut combat = awakened_transition_ordering_combat(6, 4, true);
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Strike, 11),
    ];

    let dark = transition_priority(&combat, 0);
    let lethal = transition_priority(&combat, 1);

    assert!(dark.role_rank > lethal.role_rank, "dark={dark:?} lethal={lethal:?}");
    assert!(dark.phase_hint.awakened_one_strength_transition_setup.is_some());
}
```

Add explicit ordering negatives:

```rust
#[test]
fn awakened_transition_setup_bonus_requires_all_hard_gates() {
    for (label, mut combat) in [
        (
            "form-two",
            awakened_transition_ordering_combat(6, 4, false),
        ),
        (
            "zero-strength",
            awakened_transition_ordering_combat(6, 0, true),
        ),
        (
            "insufficient-damage",
            awakened_transition_ordering_combat(40, 4, true),
        ),
    ] {
        combat.turn.energy = 1;
        combat.zones.hand = vec![
            CombatCard::new(CardId::DarkShackles, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let priority = transition_priority(&combat, 0);
        assert!(
            priority
                .phase_hint
                .awakened_one_strength_transition_setup
                .is_none(),
            "unexpected setup hint for {label}"
        );
        assert_eq!(priority.phase_hint.role_rank_adjustment, 0, "{label}");
    }
}
```

- [ ] **Step 2: Run the tests and verify the red state**

```powershell
cargo test --lib awakened_transition_setup_orders_before_direct_form_one_lethal -- --nocapture
```

Expected: the test fails because no opportunity reaches action ordering and lethal remains first.

- [ ] **Step 3: Thread the opportunity into phase ordering**

In `enemy_phase_transition.rs`, keep the `ClientInput` wrapper for existing callers and tests, then add this explicit reusable boundary:

```rust
use super::action_effects::{card_play_effect_facts, CardPlayEffectFacts};
use super::enemy_mechanics_profile::{
    enemy_mechanics_profile,
    EnemyMechanicsProfileV1,
};

pub(super) fn enemy_phase_transition_hint_for_input_with_effects(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
    enemy_mechanics: EnemyMechanicsProfileV1,
    effects: CardPlayEffectFacts,
    phase_guard: CombatSearchPhaseGuardPluginId,
) -> EnemyPhaseTransitionHint {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return EnemyPhaseTransitionHint::default();
    };
    let mut hint = play_card_phase_transition_hint(combat, card, target, phase_guard);
    hint.awakened_one_strength_transition =
        awakened_one_strength_transition_opportunity(
            combat,
            card_index,
            target,
            enemy_mechanics,
            effects,
        );
    hint
}
```

Make the existing `ClientInput` wrapper delegate with freshly observed facts for its old callers and tests:

```rust
pub(super) fn enemy_phase_transition_hint_for_input(
    combat: &CombatState,
    input: &ClientInput,
    phase_guard: CombatSearchPhaseGuardPluginId,
) -> EnemyPhaseTransitionHint {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return EnemyPhaseTransitionHint::default();
            };
            enemy_phase_transition_hint_for_input_with_effects(
                combat,
                *card_index,
                *target,
                enemy_mechanics_profile(combat),
                card_play_effect_facts(combat, card, *target),
                phase_guard,
            )
        }
        _ => EnemyPhaseTransitionHint::default(),
    }
}
```

In `priority_for_play_card`, use the already-created `effects` and `phase_profile.enemy_mechanics` rather than re-observing the card:

```rust
use super::super::enemy_phase_transition::
    enemy_phase_transition_hint_for_input_with_effects;

let phase_transition = enemy_phase_transition_hint_for_input_with_effects(
    combat,
    card_index,
    target,
    phase_profile.enemy_mechanics,
    effects,
    plugins.phase_guard,
);
```

Keep the existing public wrapper for old callers/tests; it may compute action effects once and delegate to the new function.

Import the opportunity type in `phase_action_ordering/types.rs`:

```rust
use super::super::enemy_phase_transition::AwakenedOneStrengthTransitionOpportunity;
```

Add this field to `PhaseActionOrderingHint`:

```rust
pub(in crate::ai::combat_search_v2) awakened_one_strength_transition_setup:
    Option<AwakenedOneStrengthTransitionOpportunity>,
```

Define a narrowly scoped bonus large enough to put a valid temporary-mitigation setup immediately before a form-one lethal, without changing any action set:

```rust
pub(super) const AWAKENED_STRENGTH_TRANSITION_SETUP_BONUS: i32 = 36;

if let Some(opportunity) = facts.phase_transition.awakened_one_strength_transition {
    hint.role_rank_adjustment = hint
        .role_rank_adjustment
        .saturating_add(AWAKENED_STRENGTH_TRANSITION_SETUP_BONUS);
    hint.phase_setup = hint
        .phase_setup
        .saturating_add(opportunity.convertible_positive_strength.max(1));
    hint.awakened_one_strength_transition_setup = Some(opportunity);
}
```

Update `PhaseActionOrderingHint::has_signal` to include the optional opportunity:

```rust
pub(in crate::ai::combat_search_v2) fn has_signal(self) -> bool {
    self.role_rank_adjustment != 0
        || self.phase_setup != 0
        || self.phase_survival != 0
        || self.phase_transition_safety != 0
        || self.awakened_one_strength_transition_setup.is_some()
}
```

Do not change `ActionOrderingRole`, pruning, or frontier value.

- [ ] **Step 4: Preserve the phase hint in action diagnostics**

Import `PhaseActionOrderingHint` in `action_ordering/types.rs` and `diagnostics/samples.rs`, then add this field to both `ActionOrderingActionEffectSummary` and `ActionOrderingActionEffectObservation`:

```rust
pub(in crate::ai::combat_search_v2::action_ordering) phase_hint: PhaseActionOrderingHint,
```

Change `summarize_ordering` so phase-only signals are sampled and copied:

```rust
if entry.priority.effects.has_reactive_signal() || entry.priority.phase_hint.has_signal() {
    action_effect_samples.push(ActionOrderingActionEffectSummary {
        original_action_id: entry.original_action_id,
        ordered_index,
        role: entry.priority.role,
        action_key: entry.choice.action_key.clone(),
        effects: entry.priority.effects,
        phase_hint: entry.priority.phase_hint,
    });
}
```

Copy the field in `ActionOrderingDiagnosticsCollector::observe`:

```rust
self.remember_action_effect(ActionOrderingActionEffectObservation {
    observed_at_state_query: self.states_observed,
    original_action_id: sample.original_action_id,
    ordered_index: sample.ordered_index,
    role: sample.role,
    action_key: sample.action_key.clone(),
    effects: sample.effects,
    phase_hint: sample.phase_hint,
});
```

Make the bounded sample reservoir retain transition windows ahead of ordinary reactive samples by adding this first comparison in `remember_action_effect`:

```rust
self.action_effect_samples.sort_by(|left, right| {
    right
        .phase_hint
        .awakened_one_strength_transition_setup
        .is_some()
        .cmp(
            &left
                .phase_hint
                .awakened_one_strength_transition_setup
                .is_some(),
        )
        .then_with(|| {
            right
                .effects
                .derived
                .reactive_risk_score
                .cmp(&left.effects.derived.reactive_risk_score)
        })
        .then_with(|| {
            right
                .effects
                .derived
                .mitigation_score
                .cmp(&left.effects.derived.mitigation_score)
        })
        .then_with(|| {
            right
                .effects
                .reactive
                .enemy_damage
                .cmp(&left.effects.reactive.enemy_damage)
        })
        .then_with(|| {
            left.observed_at_state_query
                .cmp(&right.observed_at_state_query)
        })
        .then_with(|| left.ordered_index.cmp(&right.ordered_index))
});
```

Expose this new serialized structure:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsActionPhase {
    pub awakened_one_strength_transition: bool,
    pub temporary_strength_down: i32,
    pub convertible_positive_strength: i32,
    pub remaining_damage_upper_bound: i32,
    pub phase_one_hp_with_block: i32,
}

pub struct CombatSearchV2DiagnosticsActionEffectSample {
    pub observed_at_state_query: u64,
    pub original_action_id: usize,
    pub ordered_index: usize,
    pub role: String,
    pub action_key: String,
    pub direct: CombatSearchV2DiagnosticsActionEffectDirect,
    pub reactive: CombatSearchV2DiagnosticsActionEffectReactive,
    pub access: CombatSearchV2DiagnosticsActionEffectAccess,
    pub derived: CombatSearchV2DiagnosticsActionEffectDerived,
    pub phase: CombatSearchV2DiagnosticsActionPhase,
}
```

Import `CombatSearchV2DiagnosticsActionPhase` in `diagnostics/report.rs` and populate zeros/false when no opportunity exists:

```rust
let opportunity = sample
    .phase_hint
    .awakened_one_strength_transition_setup;

phase: CombatSearchV2DiagnosticsActionPhase {
    awakened_one_strength_transition: opportunity.is_some(),
    temporary_strength_down: opportunity
        .map(|value| value.temporary_strength_down)
        .unwrap_or_default(),
    convertible_positive_strength: opportunity
        .map(|value| value.convertible_positive_strength)
        .unwrap_or_default(),
    remaining_damage_upper_bound: opportunity
        .map(|value| value.remaining_damage_upper_bound)
        .unwrap_or_default(),
    phase_one_hp_with_block: opportunity
        .map(|value| value.phase_one_hp_with_block)
        .unwrap_or_default(),
},
```

Bump `CombatSearchV2DiagnosticsReport.schema_version` from 13 to 14.

- [ ] **Step 5: Add and run the diagnostics regression**

Add this complete collector test in `action_ordering/tests/diagnostics.rs`:

```rust
#[test]
fn ordering_collector_reports_awakened_strength_transition_window() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = 6;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = true;
    combat.entities.monsters = vec![awakened];
    combat.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    combat.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    let ordered = order_action_choices(
        &EngineState::CombatPlayerTurn,
        &combat,
        vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(7),
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(7),
                },
            ),
        ],
    );
    let mut collector = ActionOrderingDiagnosticsCollector::default();
    collector.observe(&ordered.summary);
    let report = collector.finish();

let sample = report
    .action_effect_samples
    .iter()
    .find(|sample| sample.action_key.contains("Dark Shackles"))
    .expect("transition setup sample");
assert!(sample.phase.awakened_one_strength_transition);
assert_eq!(sample.phase.convertible_positive_strength, 4);
assert!(sample.phase.remaining_damage_upper_bound >= sample.phase.phase_one_hp_with_block);
}
```

Run:

```powershell
cargo test --lib awakened_transition_setup_orders_before_direct_form_one_lethal -- --nocapture
cargo test --lib ordering_collector_reports_awakened_strength_transition_window -- --nocapture
cargo test --lib phase_action_ordering -- --nocapture
```

Expected: all pass; existing Time Eater, Guardian, Lagavulin, and split ordering tests remain unchanged.

- [ ] **Step 6: Commit ordering and diagnostics**

```powershell
git add src/ai/combat_search_v2/action_priority src/ai/combat_search_v2/phase_action_ordering src/ai/combat_search_v2/action_ordering src/ai/combat_search_v2/types/diagnostics/action.rs src/ai/combat_search_v2/diagnostics/finish.rs
git commit -m "feat: order awakened strength transition setups"
```

### Task 4: Verify the Exact Engine Formula and Complete the Suite

**Files:**
- Modify: `src/content/monsters/beyond/awakened_one.rs`

**Interfaces:**
- Verifies: the actual engine, not the ordering projection, retains `max(0, S - D)` Strength after the immediate first-phase purge.

- [ ] **Step 1: Strengthen the existing engine regression**

In `first_phase_death_immediately_sets_rebirth_state_and_queues_java_set_move`, make the pre-transition state represent positive Strength after temporary loss and explicitly assert the retained amount:

```rust
// S=10 before Dark Shackles, D=8 => the state immediately before lethal
// contains Strength(2) plus Shackled(8). The purge removes Shackled and
// preserves the positive Strength(2).
store::set_powers_for(
    &mut state,
    7,
    vec![
        power(PowerId::Regen, 10),
        power(PowerId::Curiosity, 1),
        power(PowerId::Unawakened, -1),
        power(PowerId::Shackled, 8),
        power(PowerId::Weak, 2),
        power(PowerId::Strength, 2),
    ],
);

// after lethal and immediate purge
assert_eq!(store::power_amount(&state, 7, PowerId::Strength), 2);
assert!(!store::has_power(&state, 7, PowerId::Shackled));
```

Add a second compact case with `S < D` represented by negative Strength plus Shackled, using the same death handler:

```rust
#[test]
fn first_phase_purge_removes_negative_strength_when_temporary_loss_exceeds_strength() {
    let mut awakened = crate::test_support::test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = 1;
    awakened.max_hp = 300;
    awakened.set_planned_move_id(SLASH);
    let mut state = crate::test_support::combat_with_monsters(vec![awakened]);
    store::set_powers_for(
        &mut state,
        7,
        vec![
            power(PowerId::Regen, 10),
            power(PowerId::Unawakened, -1),
            power(PowerId::Strength, -6),
            power(PowerId::Shackled, 8),
        ],
    );

    crate::engine::action_handlers::damage::handle_damage(
        crate::runtime::action::DamageInfo {
            source: PLAYER,
            target: 7,
            base: 1,
            output: 1,
            damage_type: crate::runtime::action::DamageType::Normal,
            is_modified: true,
        },
        &mut state,
    );

    assert_eq!(store::power_amount(&state, 7, PowerId::Strength), 0);
    assert!(!store::has_power(&state, 7, PowerId::Strength));
    assert!(!store::has_power(&state, 7, PowerId::Shackled));
}
```

- [ ] **Step 2: Run the exact engine test and focused combat-search suites**

```powershell
cargo test --lib first_phase_death_immediately_sets_rebirth_state_and_queues_java_set_move -- --nocapture
cargo test --lib first_phase_purge_removes_negative_strength_when_temporary_loss_exceeds_strength -- --nocapture
cargo test --lib enemy_phase_transition -- --nocapture
cargo test --lib action_priority -- --nocapture
cargo test --lib action_ordering -- --nocapture
```

Expected: all pass, including both retained-Strength cases.

- [ ] **Step 3: Run formatting and completion verification**

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected: zero failures; architecture boundaries pass; `git diff --check` prints nothing.

- [ ] **Step 4: Commit the engine regression and report clean status**

```powershell
git add src/content/monsters/beyond/awakened_one.rs
git commit -m "test: lock awakened strength purge formula"
git status --short --branch
```

Expected: the final status is clean on `agent/reproducible-search-comparability`.
