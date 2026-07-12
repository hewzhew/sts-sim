# Self-Damage Supply Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Distinguish limited from repeatable card self-damage so Offering plus Rupture remains a plausible engine seed without being treated as stable boss scaling.

**Architecture:** `DeckMechanicContext` remains the semantic source of event availability and gains a parallel collection for repeatable event supply. Self-damage package maturity, deck-role inventory, boss-scaling evidence, and the legacy Strength profile consume that shared fact; they do not classify individual card IDs or introduce score thresholds.

**Tech Stack:** Rust, Cargo unit/integration tests, existing card-semantics and non-combat strategy pipeline.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Never run `cargo clean`.
- Follow red-green TDD with focused tests and frequent local commits.
- Do not introduce a fixed self-damage source-count threshold or card-ID allowlist.
- Do not change Exhaust package maturity, score constants, Collector policy, route policy, or combat-search behavior.
- Preserve Offering's independent draw and energy semantics.
- Run the full library and `architecture_runtime_boundaries` suites only at the completion checkpoint.

---

### Task 1: Derive Repeatable Event Supply From Card Semantics

**Files:**
- Modify: `src/ai/analysis/card_semantics.rs:171-266`
- Test: `src/ai/analysis/card_semantics.rs:562`

**Interfaces:**
- Produces: `DeckMechanicContext::repeatable_event_streams: Vec<CombatEvent>`.
- Semantics: a direct emitted event is repeatable when its card does not have `PlayEffect::ExhaustsSelf`; `TriggeredEffect::LoseHpFromCard` on an available recurring handler produces repeatable `CardSelfDamage`.
- Consumers: Tasks 2, 3, and 4 read `repeatable_event_streams`.

- [ ] **Step 1: Write failing semantic lifetime tests**

```rust
#[test]
fn offering_self_damage_is_available_but_not_repeatable() {
    let context = DeckMechanicContext::from_definitions(&[card_definition(CardId::Offering)]);
    assert!(context.event_streams.contains(&CombatEvent::CardSelfDamage));
    assert!(!context.repeatable_event_streams.contains(&CombatEvent::CardSelfDamage));
}

#[test]
fn non_exhausting_direct_self_damage_is_repeatable() {
    for card in [CardId::Bloodletting, CardId::Hemokinesis] {
        let context = DeckMechanicContext::from_definitions(&[card_definition(card)]);
        assert!(context.repeatable_event_streams.contains(&CombatEvent::CardSelfDamage));
    }
}

#[test]
fn recurring_power_self_damage_is_repeatable() {
    for card in [CardId::Combust, CardId::Brutality] {
        let context = DeckMechanicContext::from_definitions(&[card_definition(card)]);
        assert!(context.repeatable_event_streams.contains(&CombatEvent::CardSelfDamage));
    }
}
```

- [ ] **Step 2: Run the first test and verify the red state**

Run: `cargo test --lib offering_self_damage_is_available_but_not_repeatable`

Expected: compilation fails because `repeatable_event_streams` does not exist.

- [ ] **Step 3: Implement the semantic lifetime fact**

Add this field to `DeckMechanicContext`:

```rust
pub repeatable_event_streams: Vec<CombatEvent>,
```

In `add_direct_definition_facts`, derive direct repeatability from the complete definition:

```rust
let exhausts_self = definition.play_effects.contains(&PlayEffect::ExhaustsSelf);
PlayEffect::EmitEvent(event) => {
    push_unique(&mut self.event_streams, *event);
    if !exhausts_self {
        push_unique(&mut self.repeatable_event_streams, *event);
    }
}
```

Place `exhausts_self` immediately before the existing effect loop, then replace its existing `PlayEffect::EmitEvent(event)` arm with the arm shown above. No other match arm changes.

When `derive_triggered_facts` handles `TriggeredEffect::LoseHpFromCard`, add both facts:

```rust
TriggeredEffect::LoseHpFromCard => {
    push_unique(&mut self.event_streams, CombatEvent::CardSelfDamage);
    push_unique(&mut self.repeatable_event_streams, CombatEvent::CardSelfDamage);
}
```

Correct Offering's semantic lifetime:

```rust
Offering => CardDefinition::new(card)
    .provides(CardDraw)
    .provides(Energy)
    .effect(EmitEvent(CardSelfDamage))
    .effect(ExhaustsSelf)
    .burden(HpCost),
```

- [ ] **Step 4: Run the three tests and verify green**

Run:

```powershell
cargo test --lib offering_self_damage_is_available_but_not_repeatable
cargo test --lib non_exhausting_direct_self_damage_is_repeatable
cargo test --lib recurring_power_self_damage_is_repeatable
```

Expected: all three tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/analysis/card_semantics.rs
git commit -m "feat: model repeatable self-damage supply"
```

---

### Task 2: Refine Self-Damage Package Maturity

**Files:**
- Modify: `src/ai/strategy/package_state.rs:54-61`
- Test: `src/ai/strategy/package_state.rs`
- Test: `src/ai/strategy/reward_admission.rs:617`

**Interfaces:**
- Consumes: `event_streams` and `repeatable_event_streams` from Task 1.
- Produces: limited supply plus Rupture is `Seeded`; repeatable supply plus Rupture is `Supported`.
- Downstream effect: reward admission classifies Offering-backed Rupture as `EngineSeed` without score changes.

- [ ] **Step 1: Write failing package tests**

Append this test module to `package_state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::analysis::card_semantics::card_definition;
    use crate::content::cards::CardId;

    fn state(cards: &[CardId]) -> PackageStateReport {
        let definitions = cards.iter().copied().map(card_definition).collect::<Vec<_>>();
        assess_package_state(&DeckMechanicContext::from_definitions(&definitions))
    }

    #[test]
    fn limited_self_damage_with_rupture_is_seeded() {
        assert_eq!(state(&[CardId::Offering, CardId::Rupture]).self_damage, PackageMaturity::Seeded);
    }

    #[test]
    fn repeatable_self_damage_with_rupture_is_supported() {
        assert_eq!(state(&[CardId::Bloodletting, CardId::Rupture]).self_damage, PackageMaturity::Supported);
    }
}
```

Add this admission contract to `reward_admission.rs`:

```rust
#[test]
fn offering_backed_rupture_is_an_engine_seed_not_a_supported_package() {
    let admission = assess_reward_admission(&[CardId::Offering], CardId::Rupture);
    assert_eq!(admission.class, RewardAdmissionClass::EngineSeed);
    assert!(!admission.reasons.contains(&RewardAdmissionReason::Supports(PackageKind::SelfDamage)));
}
```

- [ ] **Step 2: Run the limited-supply test and verify it fails as `Supported`**

Run: `cargo test --lib limited_self_damage_with_rupture_is_seeded`

Expected: FAIL because the current maturity is `Supported`.

- [ ] **Step 3: Implement the targeted maturity table**

```rust
fn assess_self_damage_package(ctx: &DeckMechanicContext) -> PackageMaturity {
    let has_source = ctx.event_streams.contains(&CombatEvent::CardSelfDamage);
    let has_repeatable_source = ctx.repeatable_event_streams.contains(&CombatEvent::CardSelfDamage);
    let has_payoff = ctx.payoff_requirements.contains(&PayoffRequirement::WantsEventStream(
        CombatEvent::CardSelfDamage,
    ));

    match (has_source, has_repeatable_source, has_payoff) {
        (_, true, true) => PackageMaturity::Supported,
        (true, false, true) => PackageMaturity::Seeded,
        (true, _, false) => PackageMaturity::SourceOnly,
        (false, _, true) => PackageMaturity::PayoffOnly,
        (false, _, false) => PackageMaturity::None,
    }
}
```

- [ ] **Step 4: Run all three focused tests and verify green**

```powershell
cargo test --lib limited_self_damage_with_rupture_is_seeded
cargo test --lib repeatable_self_damage_with_rupture_is_supported
cargo test --lib offering_backed_rupture_is_an_engine_seed_not_a_supported_package
```

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strategy/package_state.rs src/ai/strategy/reward_admission.rs
git commit -m "fix: keep limited self-damage packages seeded"
```

---

### Task 3: Align Deck Roles and Boss-Scaling Authority

**Files:**
- Modify: `src/ai/strategy/deck_role_inventory.rs:1-152`
- Modify: `src/ai/strategy/deck_plan.rs:80-89`
- Modify: `src/ai/strategy/boss_scaling_evidence.rs:46-73,209-217`
- Test: `src/ai/strategy/deck_role_inventory.rs:163`
- Test: `src/ai/strategy/boss_scaling_evidence.rs:234`

**Interfaces:**
- Consumes: repeatable supply from `DeckMechanicContext`.
- Produces: `DeckRoleInventory::repeatable_self_damage_supply: bool` and a stable-source helper that accepts that fact.
- Boss evidence grants Rupture stable-source authority only with repeatable supply.

- [ ] **Step 1: Write failing role and boss tests**

Add to `deck_role_inventory.rs`:

```rust
#[test]
fn offering_backed_rupture_is_conditional_not_stable_strength() {
    let inventory = DeckRoleInventory::from_deck(&[
        card(CardId::Offering, 1), card(CardId::Rupture, 2),
    ]);
    assert!(!inventory.repeatable_self_damage_supply);
    assert_eq!(inventory.strength_source_units, 0);
    assert_eq!(inventory.conditional_strength_source_units, 1);
}

#[test]
fn repeatable_self_damage_makes_rupture_stable_strength() {
    let inventory = DeckRoleInventory::from_deck(&[
        card(CardId::Bloodletting, 1), card(CardId::Rupture, 2),
    ]);
    assert!(inventory.repeatable_self_damage_supply);
    assert_eq!(inventory.strength_source_units, 1);
}
```

Add to `boss_scaling_evidence.rs`:

```rust
#[test]
fn offering_backed_rupture_is_not_relevant_boss_scaling() {
    let (deck, plan) = deck_plan(&[CardId::Offering]);
    let admission = assess_reward_admission_from_master_deck(&deck, CardId::Rupture, 0);
    let evidence = assess_boss_scaling_evidence(plan, Some((CardId::Rupture, 0)), &admission);
    assert!(!evidence.relevant_to_boss_plan);
    assert_ne!(evidence.label, "boss-scaling-source");
}
```

- [ ] **Step 2: Run the role test and verify the missing-field compilation failure**

Run: `cargo test --lib offering_backed_rupture_is_conditional_not_stable_strength`

- [ ] **Step 3: Derive supply once and classify Strength handlers**

Add the inventory field:

```rust
pub repeatable_self_damage_supply: bool,
```

Build definitions and context once at the start of `from_deck`:

```rust
let definitions = deck
    .iter()
    .map(|card| card_definition_with_upgrades(card.id, card.upgrades))
    .collect::<Vec<_>>();
let context = DeckMechanicContext::from_definitions(&definitions);
inventory.repeatable_self_damage_supply = context
    .repeatable_event_streams
    .contains(&CombatEvent::CardSelfDamage);
```

Iterate `deck.iter().zip(&definitions)`. In `add_event_handler`, use:

```rust
if event == CombatEvent::CardSelfDamage && !self.repeatable_self_damage_supply {
    self.conditional_strength_source_units += 1;
    return;
}
```

Change the stable-source helper to:

```rust
pub(super) fn card_is_stable_strength_source(
    card: CardId,
    upgrades: u8,
    repeatable_self_damage_supply: bool,
) -> bool {
    let definition = card_definition_with_upgrades(card, upgrades);
    (definition.play_effects.contains(&PlayEffect::Provide(Mechanic::Strength))
        && !definition_is_conditional_strength_source(&definition))
        || definition.event_handlers.iter().any(|handler| {
            handler.effect == TriggeredEffect::Provide(Mechanic::Strength)
                && (handler.on != CombatEvent::CardSelfDamage || repeatable_self_damage_supply)
        })
}
```

- [ ] **Step 4: Pass the supply fact through deck plan and boss evidence**

In `DeckPlanSnapshot::repairs_strength_package_reliability`:

```rust
candidate.is_some_and(|(card, upgrades)| {
    card_is_stable_strength_source(
        card,
        upgrades,
        self.roles.repeatable_self_damage_supply,
    )
})
```

In `assess_boss_scaling_evidence`, replace the existing condition

```rust
if admission_provides(admission, Mechanic::Strength)
    || card_grants_strength(card_semantics.as_ref())
{
```

with:

```rust
if admission_provides(admission, Mechanic::Strength)
    || card.is_some_and(|(id, upgrades)| {
        card_is_stable_strength_source(id, upgrades, deck.roles.repeatable_self_damage_supply)
    })
{
```

Delete the local `card_grants_strength` function and its unused `TriggeredEffect` import.

- [ ] **Step 5: Run focused role and boss tests**

```powershell
cargo test --lib offering_backed_rupture_is_conditional_not_stable_strength
cargo test --lib repeatable_self_damage_makes_rupture_stable_strength
cargo test --lib offering_backed_rupture_is_not_relevant_boss_scaling
cargo test --lib repeated_rupture_does_not_repeat_full_boss_scaling_credit
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```powershell
git add src/ai/strategy/deck_role_inventory.rs src/ai/strategy/deck_plan.rs src/ai/strategy/boss_scaling_evidence.rs
git commit -m "fix: gate Rupture scaling on repeatable supply"
```

---

### Task 4: Align Legacy Strength and Startup Profiles

**Files:**
- Modify: `src/ai/strength_profile_v1.rs:35-102`
- Test: `src/ai/strength_profile_v1.rs:179`
- Test: `src/ai/deck_startup_profile_v1.rs:570`

**Interfaces:**
- Consumes: repeatable supply from Task 1.
- Produces: stable and persistent Strength counts that do not promote Offering-backed Rupture.
- Preserves: `self_damage_source_count` remains a broad diagnostic count.

- [ ] **Step 1: Write failing profile tests**

Add to `strength_profile_v1.rs`:

```rust
#[test]
fn offering_does_not_make_rupture_a_stable_strength_source() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.add_card_to_deck(CardId::Offering);
    run.add_card_to_deck(CardId::Rupture);
    assert_eq!(strength_profile_v1(&run).stable_sources, 0);
}

#[test]
fn repeatable_self_damage_makes_rupture_a_stable_strength_source() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.add_card_to_deck(CardId::Bloodletting);
    run.add_card_to_deck(CardId::Rupture);
    assert_eq!(strength_profile_v1(&run).stable_sources, 1);
}
```

Add to `deck_startup_profile_v1.rs`:

```rust
#[test]
fn offering_backed_rupture_remains_non_persistent_at_startup() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.add_card_to_deck(CardId::Offering);
    run.add_card_to_deck(CardId::Rupture);
    let profile = deck_startup_profile_v1(&run);
    assert_eq!(profile.self_damage_source_count, 1);
    assert_eq!(profile.persistent_strength_source_count, 0);
}
```

- [ ] **Step 2: Run the Offering test and verify `stable_sources` is incorrectly 1**

Run: `cargo test --lib offering_does_not_make_rupture_a_stable_strength_source`

- [ ] **Step 3: Consume semantic repeatability in `strength_profile_v1`**

Add the semantic imports and derive the deck context:

```rust
let definitions = run_state
    .master_deck
    .iter()
    .map(|card| card_definition_with_upgrades(card.id, card.upgrades))
    .collect::<Vec<_>>();
let context = DeckMechanicContext::from_definitions(&definitions);
let has_repeatable_self_damage = context
    .repeatable_event_streams
    .contains(&CombatEvent::CardSelfDamage);
```

Remove `self_damage_sources` and its increment. Replace the promotion condition with:

```rust
if rupture_count > 0 && has_repeatable_self_damage {
    profile.stable_sources = profile.stable_sources.saturating_add(rupture_count);
}
```

- [ ] **Step 4: Run focused Strength and startup tests**

```powershell
cargo test --lib offering_does_not_make_rupture_a_stable_strength_source
cargo test --lib repeatable_self_damage_makes_rupture_a_stable_strength_source
cargo test --lib offering_backed_rupture_remains_non_persistent_at_startup
cargo test --lib rupture_requires_self_damage_before_counting_as_strength_source
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strength_profile_v1.rs src/ai/deck_startup_profile_v1.rs
git commit -m "fix: align Rupture strength reliability profiles"
```

---

### Task 5: Completion Verification

**Files:**
- Verify: all files changed in Tasks 1-4

**Interfaces:**
- Consumes: all completed implementation tasks.
- Produces: repository-level evidence for library behavior and architecture boundaries.

- [ ] **Step 1: Format and inspect**

```powershell
cargo fmt --all
cargo fmt --all -- --check
git diff --check
git status --short
```

- [ ] **Step 2: Run the full library suite once**

Run: `cargo test --lib`

Expected: all library tests pass.

- [ ] **Step 3: Run architecture boundaries once**

Run: `cargo test --test architecture_runtime_boundaries`

Expected: all seven tests pass.

- [ ] **Step 4: Commit formatter changes only when present**

```powershell
git add src/ai/analysis/card_semantics.rs src/ai/strategy/package_state.rs src/ai/strategy/reward_admission.rs src/ai/strategy/deck_role_inventory.rs src/ai/strategy/deck_plan.rs src/ai/strategy/boss_scaling_evidence.rs src/ai/strength_profile_v1.rs src/ai/deck_startup_profile_v1.rs
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m "style: format self-damage reliability repair" }
```

- [ ] **Step 5: Record final state**

```powershell
git status --short --branch
git log -5 --oneline
```

Expected: the worktree is clean and recent history contains the design, plan, and focused implementation commits.
