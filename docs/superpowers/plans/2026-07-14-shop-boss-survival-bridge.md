# Shop Boss-Survival Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Act 3 shop recognize Dark Shackles as a typed Awakened One timed bridge, without allowing Weak, Disarm, Maw Bank, or a survival emergency to erase the real policy boundaries.

**Architecture:** Split factual deck mitigation coverage in card semantics and `DeckRoleInventory`, then let `boss_survival_evidence` own the boss-specific policy. Carry the resulting `PlanRepair`/`TimedBridge` fact through the early shop bundle filter so the later acquisition policy is reachable; do not put combat-turn prediction in shop code.

**Tech Stack:** Rust 2021, existing strategy/card-semantics modules, Cargo library tests, `architecture_runtime_boundaries`.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator` on the existing local feature branch; do not create a worktree.
- Never run `cargo clean`.
- Use focused tests for every red/green cycle; run the full library and `architecture_runtime_boundaries` suites only at the completion checkpoint.
- Preserve the existing aggregate `mitigation_units` behavior for generic consumers while adding Weak, persistent Strength-down, and temporary Strength-down coverage.
- `Dark Shackles` is always a `TimedBridge` when the Awakened One pressure and temporary-coverage slot are open; upgrade and Runic Pyramid change score, not admission kind.
- A boss-survival repair may cross future-shop liquidity only when `boss_answer_needed` is true and `survival_purchase_needed` is false.
- Maw Bank rejection remains earlier than strategic repair; no repair kind may override it.
- Do not add card-name logic to combat search in this plan; combat ordering belongs to the separate Awakened One transition-window plan.

---

## File Map

- Modify `src/ai/analysis/card_semantics.rs`: add the factual temporary-enemy-Strength-down mechanic and assign it to Dark Shackles.
- Modify `src/ai/strategy/deck_role_inventory.rs`: retain aggregate mitigation and add three distinct coverage counters.
- Modify `src/ai/strategy/run_strategic_facts.rs`: expose factual Runic Pyramid ownership.
- Modify `src/ai/strategy/reward_admission.rs`, `src/ai/strategy/acquisition.rs`, `src/ai/strategy/boss_scaling_evidence.rs`, `src/ai/strategy/candidate_pressure_response.rs`, `src/ai/strategy/decision_pipeline.rs`, `src/ai/strategy/deck_admission.rs`, `src/ai/strategy/deck_construction_pressure.rs`, `src/ai/strategy/exhaust_corruption_assessment.rs`, and `src/ai/strategy/role_saturation.rs`: keep generic Strength-down consumers inclusive of both persistent and temporary variants.
- Modify `src/ai/strategy/boss_survival_evidence.rs`: own `BossSurvivalRepairKind` and Awakened One candidate-slot policy.
- Modify `src/ai/strategy/shop_purchase_bundle.rs`: carry the repair kind across the early liquidity filter.
- Modify `src/ai/strategy/acquisition.rs` and `src/ai/strategy/decision_pipeline.rs`: consume typed evidence and expose stable score/reason labels.
- Modify RunStrategicFacts test literals in `src/ai/strategy/acquisition.rs`, `src/ai/strategy/boss_scaling_evidence.rs`, `src/ai/strategy/boss_survival_evidence.rs`, `src/ai/strategy/decision_pipeline.rs`, `src/ai/strategy/deck_strategic_deficit.rs`, and `src/bin/combat_case_review/case_payload/derived.rs`.

### Task 1: Split Factual Mitigation Coverage

**Files:**
- Modify: `src/ai/analysis/card_semantics.rs`
- Modify: `src/ai/strategy/deck_role_inventory.rs`
- Modify: `src/ai/strategy/run_strategic_facts.rs`
- Modify: the generic consumers and RunStrategicFacts literal sites listed in the file map

**Interfaces:**
- Produces: `Mechanic::TemporaryEnemyStrengthDown`.
- Produces: `DeckRoleInventory::{weak_units, persistent_enemy_strength_down_units, temporary_enemy_strength_down_units}` while retaining `mitigation_units`.
- Produces: `RunStrategicFacts::has_runic_pyramid: bool`.

- [ ] **Step 1: Write failing semantic and inventory tests**

Replace the Dark Shackles semantic assertion and add inventory coverage tests:

```rust
#[test]
fn dark_shackles_semantics_distinguish_temporary_enemy_strength_down() {
    let definition = card_definition(CardId::DarkShackles);

    assert!(definition.play_effects.contains(&PlayEffect::Provide(
        Mechanic::TemporaryEnemyStrengthDown
    )));
    assert!(!definition
        .play_effects
        .contains(&PlayEffect::Provide(Mechanic::EnemyStrengthDown)));
    assert!(definition.play_effects.contains(&PlayEffect::ExhaustsSelf));
}
```

```rust
#[test]
fn role_inventory_separates_weak_persistent_and_temporary_mitigation() {
    let inventory = DeckRoleInventory::from_deck(&[
        card(CardId::Clothesline, 1),
        card(CardId::Disarm, 2),
        card(CardId::DarkShackles, 3),
    ]);

    assert_eq!(inventory.weak_units, 1);
    assert_eq!(inventory.persistent_enemy_strength_down_units, 1);
    assert_eq!(inventory.temporary_enemy_strength_down_units, 1);
    assert_eq!(inventory.mitigation_units, 3);
    assert_eq!(inventory.debuff_units, 3);
}
```

- [ ] **Step 2: Run the tests and verify the red state**

Run:

```powershell
cargo test --lib dark_shackles_semantics_distinguish_temporary_enemy_strength_down -- --nocapture
cargo test --lib role_inventory_separates_weak_persistent_and_temporary_mitigation -- --nocapture
```

Expected: compilation fails because the mechanic and coverage fields do not exist.

- [ ] **Step 3: Implement the semantic and inventory split**

Add the mechanic and classify the two cards explicitly:

```rust
pub enum Mechanic {
    Strength,
    TemporaryStrength,
    StrengthMultiplier,
    CardDraw,
    Energy,
    Block,
    Weak,
    Vulnerable,
    EnemyStrengthDown,
    TemporaryEnemyStrengthDown,
    TopdeckControl,
}

// In card_definition_with_upgrades:
Disarm => CardDefinition::new(card).provides(EnemyStrengthDown),
DarkShackles => CardDefinition::new(card)
    .provides(TemporaryEnemyStrengthDown)
    .effect(ExhaustsSelf),
```

Add these factual counters beside `mitigation_units` and `debuff_units`, then aggregate them in one place:

```rust
pub mitigation_units: u8,
pub weak_units: u8,
pub persistent_enemy_strength_down_units: u8,
pub temporary_enemy_strength_down_units: u8,
pub debuff_units: u8,

fn add_mechanic(&mut self, mechanic: Mechanic) {
    match mechanic {
        Mechanic::Block => self.block_units += 1,
        Mechanic::Weak => {
            self.weak_units += 1;
            self.mitigation_units += 1;
            self.debuff_units += 1;
        }
        Mechanic::EnemyStrengthDown => {
            self.persistent_enemy_strength_down_units += 1;
            self.mitigation_units += 1;
            self.debuff_units += 1;
        }
        Mechanic::TemporaryEnemyStrengthDown => {
            self.temporary_enemy_strength_down_units += 1;
            self.mitigation_units += 1;
            self.debuff_units += 1;
        }
        Mechanic::Vulnerable => {
            self.debuff_units += 1;
            self.vulnerable_units += 1;
        }
        Mechanic::CardDraw => self.draw_units += 1,
        Mechanic::Energy => self.energy_units += 1,
        Mechanic::Strength => self.strength_source_units += 1,
        Mechanic::StrengthMultiplier => self.strength_multiplier_units += 1,
        Mechanic::TemporaryStrength | Mechanic::TopdeckControl => {}
    }
}
```

In every generic consumer that currently accepts `Mechanic::EnemyStrengthDown`, include `Mechanic::TemporaryEnemyStrengthDown` in the same match or predicate. Use these exact replacements:

```rust
// Mechanic matches
Mechanic::Weak | Mechanic::EnemyStrengthDown | Mechanic::TemporaryEnemyStrengthDown

// PlayEffect matches
PlayEffect::Provide(
    Mechanic::Weak
        | Mechanic::EnemyStrengthDown
        | Mechanic::TemporaryEnemyStrengthDown,
)

// admission predicates
admission_provides(admission, Mechanic::EnemyStrengthDown)
    || admission_provides(admission, Mechanic::TemporaryEnemyStrengthDown)

// reward_admission.rs label arm
Mechanic::TemporaryEnemyStrengthDown => "temp-str-down",
```

Do not change boss-specific policy in these generic files.

- [ ] **Step 4: Add and populate the Runic Pyramid fact**

Add the field and derive it from relic ownership:

```rust
use crate::content::relics::{energy_master_delta, RelicId};

pub struct RunStrategicFacts {
    pub entering_act: u8,
    pub starter_basic_count: usize,
    pub curse_count: usize,
    pub has_energy_relic: bool,
    pub has_runic_pyramid: bool,
}

// inside from_run_state
has_runic_pyramid: run_state
    .relics
    .iter()
    .any(|relic| relic.id == RelicId::RunicPyramid),
```

Add `has_runic_pyramid: false` to every existing test/adapter literal. Add this focused test in `run_strategic_facts.rs`:

```rust
#[test]
fn strategic_facts_report_runic_pyramid_without_policy_judgment() {
    let mut run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run.relics.push(crate::content::relics::RelicState::new(
        RelicId::RunicPyramid,
    ));

    assert!(RunStrategicFacts::from_run_state(&run).has_runic_pyramid);
}
```

- [ ] **Step 5: Run focused and compile-surface tests**

Run:

```powershell
cargo test --lib dark_shackles_semantics_distinguish_temporary_enemy_strength_down -- --nocapture
cargo test --lib role_inventory_separates_weak_persistent_and_temporary_mitigation -- --nocapture
cargo test --lib strategic_facts_report_runic_pyramid_without_policy_judgment -- --nocapture
cargo test --lib reward_admission -- --nocapture
```

Expected: all selected tests pass and no exhaustive `Mechanic` match remains broken.

- [ ] **Step 6: Commit the factual layer**

```powershell
git add src/ai/analysis/card_semantics.rs src/ai/strategy src/bin/combat_case_review/case_payload/derived.rs
git commit -m "refactor: split enemy mitigation coverage"
```

### Task 2: Add Typed Awakened One Survival Evidence

**Files:**
- Modify: `src/ai/strategy/boss_survival_evidence.rs`
- Modify: `src/ai/strategy/acquisition.rs`
- Modify: `src/ai/strategy/decision_pipeline.rs`

**Interfaces:**
- Produces: `BossSurvivalRepairKind::{PlanRepair, TimedBridge}`.
- Produces: `BossSurvivalEvidence::repair_kind: Option<BossSurvivalRepairKind>` and `repairs_plan()`.
- Consumes: the three new inventory coverage fields and `RunStrategicFacts::has_runic_pyramid`.

- [ ] **Step 1: Write failing evidence tests**

Add tests that exercise complementarity and duplicate saturation:

```rust
#[test]
fn dark_shackles_is_timed_bridge_alongside_existing_weak_and_disarm() {
    let plan = deck_plan(
        &[
            CardId::DemonForm,
            CardId::Whirlwind,
            CardId::Clothesline,
            CardId::Disarm,
        ],
        Some(EncounterId::AwakenedOne),
    );
    let admission = RewardAdmission {
        card: Some(CardId::DarkShackles),
        class: RewardAdmissionClass::ImmediateWork,
        reasons: vec![
            RewardAdmissionReason::Provides(Mechanic::TemporaryEnemyStrengthDown),
            RewardAdmissionReason::ExhaustsSelf,
        ],
    };

    let evidence = assess_boss_survival_evidence(
        plan,
        Some((CardId::DarkShackles, 0)),
        &admission,
    );

    assert_eq!(evidence.repair_kind, Some(BossSurvivalRepairKind::TimedBridge));
    assert_eq!(evidence.label, "awakened-one-temporary-strength-timed-bridge");
}

#[test]
fn duplicate_dark_shackles_is_score_only_not_second_timed_bridge() {
    let plan = deck_plan(
        &[CardId::DemonForm, CardId::Whirlwind, CardId::DarkShackles],
        Some(EncounterId::AwakenedOne),
    );
    let admission = RewardAdmission {
        card: Some(CardId::DarkShackles),
        class: RewardAdmissionClass::ImmediateWork,
        reasons: vec![RewardAdmissionReason::Provides(
            Mechanic::TemporaryEnemyStrengthDown,
        )],
    };

    let evidence = assess_boss_survival_evidence(
        plan,
        Some((CardId::DarkShackles, 1)),
        &admission,
    );

    assert_eq!(evidence.repair_kind, None);
    assert!(evidence.score_delta > 0);
}

#[test]
fn upgrade_and_pyramid_raise_timed_bridge_score_without_changing_kind() {
    let base = deck_plan(
        &[CardId::DemonForm, CardId::Whirlwind, CardId::Clothesline],
        Some(EncounterId::AwakenedOne),
    );
    let mut retained = base;
    retained.run_facts.has_runic_pyramid = true;
    let admission = RewardAdmission {
        card: Some(CardId::DarkShackles),
        class: RewardAdmissionClass::ImmediateWork,
        reasons: vec![RewardAdmissionReason::Provides(
            Mechanic::TemporaryEnemyStrengthDown,
        )],
    };

    let plain = assess_boss_survival_evidence(
        base,
        Some((CardId::DarkShackles, 0)),
        &admission,
    );
    let upgraded_retained = assess_boss_survival_evidence(
        retained,
        Some((CardId::DarkShackles, 1)),
        &admission,
    );

    assert_eq!(plain.repair_kind, upgraded_retained.repair_kind);
    assert!(upgraded_retained.score_delta > plain.score_delta);
}
```

- [ ] **Step 2: Run the tests and verify the red state**

```powershell
cargo test --lib dark_shackles_is_timed_bridge_alongside_existing_weak_and_disarm -- --nocapture
cargo test --lib duplicate_dark_shackles_is_score_only_not_second_timed_bridge -- --nocapture
cargo test --lib upgrade_and_pyramid_raise_timed_bridge_score_without_changing_kind -- --nocapture
```

Expected: compilation fails because typed repair evidence does not exist.

- [ ] **Step 3: Implement the typed evidence contract**

Use one source of truth rather than retaining a parallel boolean:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossSurvivalRepairKind {
    PlanRepair,
    TimedBridge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BossSurvivalEvidence {
    pub label: &'static str,
    pub score_delta: i32,
    pub repair_kind: Option<BossSurvivalRepairKind>,
}

impl BossSurvivalEvidence {
    const fn plan_repair(label: &'static str, score_delta: i32) -> Self {
        Self { label, score_delta, repair_kind: Some(BossSurvivalRepairKind::PlanRepair) }
    }

    const fn timed_bridge(label: &'static str, score_delta: i32) -> Self {
        Self { label, score_delta, repair_kind: Some(BossSurvivalRepairKind::TimedBridge) }
    }

    const fn score_only(label: &'static str, score_delta: i32) -> Self {
        Self { label, score_delta, repair_kind: None }
    }

    const fn none() -> Self {
        Self { label: "", score_delta: 0, repair_kind: None }
    }

    pub const fn repairs_plan(self) -> bool {
        self.repair_kind.is_some()
    }
}
```

Apply these exact mechanical replacements to the existing callers:

```rust
BossSurvivalEvidence::relevant(label, score_delta)
// becomes
BossSurvivalEvidence::plan_repair(label, score_delta)

evidence.relevant_to_boss_survival_plan
// becomes
evidence.repairs_plan()
```

- [ ] **Step 4: Implement candidate-slot Awakened One policy**

Remove `deck.roles.mitigation_units == 0` from the pressure gate. Keep Act 3 plus the existing scaling/multi-hit context, then gate each candidate by its own slot:

```rust
fn awakened_one_survival_pressure_open(deck: DeckPlanSnapshot) -> bool {
    deck.context.act >= 3
        && (deck.roles.strength_source_units > 0 || deck.roles.aoe_units > 0)
}

fn dark_shackles_bridge_score(deck: DeckPlanSnapshot, upgrades: u8) -> i32 {
    80 + if upgrades > 0 { 20 } else { 0 }
        + if deck.run_facts.has_runic_pyramid { 15 } else { 0 }
}
```

In `awakened_one_survival_evidence` use this complete match so the existing block/exhaust behavior stays explicit:

```rust
match card {
    CardId::Disarm if deck.roles.persistent_enemy_strength_down_units == 0 => {
        BossSurvivalEvidence::plan_repair("awakened-one-strength-down-survival", 100)
    }
    CardId::Disarm => {
        BossSurvivalEvidence::score_only("awakened-one-duplicate-strength-down", 20)
    }
    CardId::Shockwave if deck.roles.weak_units == 0 => {
        BossSurvivalEvidence::plan_repair("awakened-one-weak-strength-down-survival", 95)
    }
    CardId::Shockwave => {
        BossSurvivalEvidence::score_only("awakened-one-duplicate-weak", 15)
    }
    CardId::DarkShackles if deck.roles.temporary_enemy_strength_down_units == 0 => {
        BossSurvivalEvidence::timed_bridge(
            "awakened-one-temporary-strength-timed-bridge",
            dark_shackles_bridge_score(deck, upgrades),
        )
    }
    CardId::DarkShackles => {
        BossSurvivalEvidence::score_only("awakened-one-duplicate-timed-bridge", 15)
    }
    CardId::Impervious | CardId::PowerThrough => {
        BossSurvivalEvidence::plan_repair("awakened-one-dark-echo-block-plan", 85)
    }
    CardId::FlameBarrier => {
        BossSurvivalEvidence::plan_repair("awakened-one-repeatable-block-plan", 70)
    }
    CardId::SecondWind
        if deck.roles.exhaust_stream_units > 0 || deck.roles.corruption_units > 0 =>
    {
        BossSurvivalEvidence::plan_repair("awakened-one-exhaust-block-plan", 65)
    }
    CardId::FeelNoPain
        if deck.roles.exhaust_stream_units > 0 || deck.roles.corruption_units > 0 =>
    {
        BossSurvivalEvidence::plan_repair("awakened-one-exhaust-block-engine", 60)
    }
    CardId::ShrugItOff if upgrades > 0 => {
        BossSurvivalEvidence::score_only("awakened-one-generic-block-access", 20)
    }
    _ => BossSurvivalEvidence::none(),
}
```

Do not let this function inspect combat state.

- [ ] **Step 5: Run the focused evidence and existing boss tests**

```powershell
cargo test --lib boss_survival_evidence -- --nocapture
cargo test --lib reward_awakened_one_context -- --nocapture
```

Expected: all boss-survival and existing reward tests pass.

- [ ] **Step 6: Commit typed boss evidence**

```powershell
git add src/ai/strategy/boss_survival_evidence.rs src/ai/strategy/acquisition.rs src/ai/strategy/decision_pipeline.rs
git commit -m "feat: type boss survival repairs"
```

### Task 3: Carry Survival Evidence Across the Early Shop Filter

**Files:**
- Modify: `src/ai/strategy/shop_purchase_bundle.rs`
- Modify: `src/ai/strategy/decision_pipeline.rs`

**Interfaces:**
- Consumes: `BossSurvivalRepairKind` from Task 2.
- Produces: `ShopPurchaseCandidateEvidence::boss_survival_repair` and `ShopPurchaseBundleFacts::boss_survival_repair`.
- Preserves: `ShopPurchaseBundleVerdict::StrategicBossRepairBuy`; stable reason/score labels distinguish scaling from survival.

- [ ] **Step 1: Write failing bundle boundary tests**

Add these tests beside the existing semantic boss-scaling repair tests:

```rust
#[test]
fn timed_boss_survival_bridge_can_spend_future_shop_liquidity() {
    let shackles = evaluation(
        DecisionCandidateKind::ShopBuyCard {
            card: CardId::DarkShackles,
            upgrades: 1,
            price: 78,
        },
        120,
    );
    let opportunity = ShopGoldOpportunity {
        boss_answer_needed: true,
        ..visible_future_shop_opportunity(180)
    };

    let decision = evaluate_shop_purchase_bundle_with_evidence(
        opportunity,
        &shackles,
        ShopPurchaseCandidateEvidence {
            repairs_boss_scaling_plan: false,
            boss_survival_repair: Some(BossSurvivalRepairKind::TimedBridge),
        },
    );

    assert_eq!(decision.verdict, ShopPurchaseBundleVerdict::StrategicBossRepairBuy);
    assert_eq!(decision.reason, "StrategicBossSurvivalTimedBridge");
    assert_eq!(decision.facts.boss_survival_repair, Some(BossSurvivalRepairKind::TimedBridge));
}

#[test]
fn boss_survival_plan_repair_uses_distinct_bundle_reason() {
    let disarm = evaluation(
        DecisionCandidateKind::ShopBuyCard {
            card: CardId::Disarm,
            upgrades: 0,
            price: 75,
        },
        120,
    );
    let decision = evaluate_shop_purchase_bundle_with_evidence(
        ShopGoldOpportunity {
            boss_answer_needed: true,
            ..visible_future_shop_opportunity(180)
        },
        &disarm,
        ShopPurchaseCandidateEvidence {
            repairs_boss_scaling_plan: false,
            boss_survival_repair: Some(BossSurvivalRepairKind::PlanRepair),
        },
    );

    assert_eq!(
        decision.verdict,
        ShopPurchaseBundleVerdict::StrategicBossRepairBuy
    );
    assert_eq!(decision.reason, "StrategicBossSurvivalPlanRepair");
}

#[test]
fn timed_bridge_does_not_override_maw_bank_or_survival_emergency() {
    let shackles = evaluation(
        DecisionCandidateKind::ShopBuyCard {
            card: CardId::DarkShackles,
            upgrades: 1,
            price: 78,
        },
        120,
    );
    let evidence = ShopPurchaseCandidateEvidence {
        repairs_boss_scaling_plan: false,
        boss_survival_repair: Some(BossSurvivalRepairKind::TimedBridge),
    };

    let maw = evaluate_shop_purchase_bundle_with_evidence(
        ShopGoldOpportunity { boss_answer_needed: true, ..maw_bank_opportunity(180) },
        &shackles,
        evidence,
    );
    let emergency = evaluate_shop_purchase_bundle_with_evidence(
        ShopGoldOpportunity {
            boss_answer_needed: true,
            survival_purchase_needed: true,
            ..visible_future_shop_opportunity(180)
        },
        &shackles,
        evidence,
    );

    assert_eq!(maw.reason, "BreaksMawBankWithoutHardNeed");
    assert_eq!(emergency.reason, "SpendsFutureShopLiquidityWithoutHardNeed");
}
```

- [ ] **Step 2: Run the tests and verify the red state**

```powershell
cargo test --lib timed_boss_survival_bridge_can_spend_future_shop_liquidity -- --nocapture
cargo test --lib boss_survival_plan_repair_uses_distinct_bundle_reason -- --nocapture
cargo test --lib timed_bridge_does_not_override_maw_bank_or_survival_emergency -- --nocapture
```

Expected: compilation fails because shop evidence has no survival repair field.

- [ ] **Step 3: Add the typed bundle fields and verdict ordering**

```rust
use crate::ai::strategy::boss_survival_evidence::BossSurvivalRepairKind;

pub struct ShopPurchaseBundleFacts {
    pub kind: ShopPurchaseBundleKind,
    pub total_cost: i32,
    pub gold_after: i32,
    pub breaks_maw_bank: bool,
    pub future_gold_lost_if_breaks_maw_bank: i32,
    pub preserves_remove_option: bool,
    pub preserves_next_shop_option: bool,
    pub solves_next_fight: bool,
    pub solves_boss_gap: bool,
    pub repairs_boss_scaling_plan: bool,
    pub boss_survival_repair: Option<BossSurvivalRepairKind>,
    pub adds_deck_burden: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShopPurchaseCandidateEvidence {
    pub repairs_boss_scaling_plan: bool,
    pub boss_survival_repair: Option<BossSurvivalRepairKind>,
}
```

In `bundle_facts`, admit the field only under the existing boss-need and no-emergency gate:

```rust
boss_survival_repair: if opportunity.boss_answer_needed
    && !opportunity.survival_purchase_needed
{
    evidence.boss_survival_repair
} else {
    None
},
```

Keep `facts.breaks_maw_bank` before strategic repairs. Immediately after the existing scaling-repair branch, add:

```rust
if let Some(repair) = facts.boss_survival_repair {
    return (
        ShopPurchaseBundleVerdict::StrategicBossRepairBuy,
        match repair {
            BossSurvivalRepairKind::PlanRepair => "StrategicBossSurvivalPlanRepair",
            BossSurvivalRepairKind::TimedBridge => "StrategicBossSurvivalTimedBridge",
        },
    );
}
```

Update existing `ShopPurchaseCandidateEvidence` literals with `..Default::default()`.

- [ ] **Step 4: Populate early evidence from the candidate**

In `shop_purchase_candidate_evidence`, assess survival evidence before the bundle filter can reject the card:

```rust
let boss_survival_repair = admission.and_then(|admission| {
    assess_boss_survival_evidence(
        context.deck_plan,
        candidate_card(candidate.kind),
        admission,
    )
    .repair_kind
});

ShopPurchaseCandidateEvidence {
    repairs_boss_scaling_plan,
    boss_survival_repair,
}
```

In `shop_purchase_bundle_score`, select stable labels by facts:

```rust
ShopPurchaseBundleVerdict::StrategicBossRepairBuy => {
    match bundle.facts.boss_survival_repair {
        Some(BossSurvivalRepairKind::PlanRepair) =>
            "shop-bundle-boss-survival-plan-repair",
        Some(BossSurvivalRepairKind::TimedBridge) =>
            "shop-bundle-boss-survival-timed-bridge",
        None => "shop-bundle-boss-scaling-repair",
    }
}
```

- [ ] **Step 5: Run bundle and pipeline tests**

```powershell
cargo test --lib shop_purchase_bundle -- --nocapture
cargo test --lib shop_boss_scaling_repair_precedes_cleanup_and_survives_liquidity_guard -- --nocapture
```

Expected: all tests pass; the old scaling-repair label remains unchanged.

- [ ] **Step 6: Commit the early bridge**

```powershell
git add src/ai/strategy/shop_purchase_bundle.rs src/ai/strategy/decision_pipeline.rs
git commit -m "feat: bridge boss survival evidence into shops"
```

### Task 4: Lock the Exact Seed006 F35 Decision and Finish Verification

**Files:**
- Modify: `src/ai/strategy/decision_pipeline.rs`

**Interfaces:**
- Consumes: all Task 1-3 interfaces.
- Verifies: exact F35 facts from the durable seed006 artifact without replaying the entire seed.

- [ ] **Step 1: Write the exact F35 decision regression**

Use the artifact deck and resources, including Clothesline and Runic Pyramid:

```rust
#[test]
fn seed006_f35_dark_shackles_bridge_survives_early_liquidity_filter() {
    let cards = [
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
        CardId::Berserk,
        CardId::Clothesline,
        CardId::Feed,
        CardId::BattleTrance,
        CardId::Armaments,
        CardId::ShrugItOff,
        CardId::MasterOfStrategy,
        CardId::Inflame,
    ];
    let mut deck = test_deck(&cards);
    for card in &mut deck {
        if matches!(
            card.id,
            CardId::Bash
                | CardId::Clothesline
                | CardId::BattleTrance
                | CardId::Armaments
                | CardId::ShrugItOff
                | CardId::MasterOfStrategy
                | CardId::Inflame
        ) {
            card.upgrades = 1;
        }
    }
    let context = DecisionPipelineContext::shop(
        DeckPlanSnapshot::from_deck(
            &deck,
            DeckAdmissionContext { act: 3, current_hp: 110, max_hp: 110 },
            RunStrategicFacts {
                entering_act: 3,
                starter_basic_count: 6,
                curse_count: 0,
                has_energy_relic: true,
                has_runic_pyramid: true,
            },
        )
        .with_boss_key(Some(EncounterId::AwakenedOne)),
        180,
    )
    .with_shop_gold_opportunity(ShopGoldOpportunity {
        current_gold: 180,
        current_hp: 110,
        max_hp: 110,
        active_maw_bank: false,
        future_rooms_before_next_shop: 2,
        hard_checkpoint_imminent: false,
        survival_purchase_needed: false,
        boss_answer_needed: true,
    });

    let shackles = shop_card_in_context_with_price(
        context,
        &deck,
        CardId::DarkShackles,
        1,
        78,
    );

    assert_eq!(shackles.lane, CandidateLane::Mainline, "{shackles:#?}");
    assert_ne!(
        shackles.inspect_only_reason(),
        Some("SpendsFutureShopLiquidityWithoutHardNeed")
    );
    assert!(shackles.scores.iter().any(|score| {
        score.by == "awakened-one-temporary-strength-timed-bridge"
    }));
    assert!(shackles.scores.iter().any(|score| {
        score.by == "shop-bundle-boss-survival-timed-bridge"
    }));
}
```

- [ ] **Step 2: Run the exact regression and nearby safety tests**

```powershell
cargo test --lib seed006_f35_dark_shackles_bridge_survives_early_liquidity_filter -- --nocapture
cargo test --lib strategic_boss_scaling_repair_does_not_override_maw_bank_or_survival_emergency -- --nocapture
cargo test --lib reward_awakened_one_context -- --nocapture
```

Expected: all tests pass. The exact regression is Mainline and carries both evidence labels.

- [ ] **Step 3: Run formatting and the completion suites**

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected: formatting succeeds, the full library has zero failures, all architecture boundary tests pass, and `git diff --check` prints nothing.

- [ ] **Step 4: Commit the integration regression**

```powershell
git add src/ai/strategy/decision_pipeline.rs
git commit -m "test: cover seed006 timed shop bridge"
git status --short --branch
```

Expected: the final status is clean on `agent/reproducible-search-comparability`.
