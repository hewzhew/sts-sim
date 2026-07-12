# Card Fact Semantics Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct the non-combat strategy facts for Reaper, Dark Shackles, and Shockwave without changing scoring or seed-specific policy.

**Architecture:** Extend the existing declarative `CardDefinition` match in `src/ai/analysis/card_semantics.rs`, and remove the duplicate incorrect Shockwave fact in the legacy `card_facts` layer. Tests assert card mechanics directly so they remain valid independently of candidate scores, route choices, or the frozen Collector seed.

**Tech Stack:** Rust, built-in test harness, Cargo.

## Global Constraints

- Do not change score constants, candidate lanes, Collector-specific rules, shop bundle rules, combat search behavior, or runtime card implementations.
- Reuse the existing coarse `EnemyStrengthDown` mechanic for Dark Shackles; do not introduce a temporal-debuff type.
- Do not add a frozen seed outcome assertion.
- Follow red-green TDD for every production change.
- Work in the stable checkout and do not run `cargo clean`.

---

### Task 1: Model Reaper's direct AOE sustain

**Files:**
- Modify: `src/ai/analysis/card_semantics.rs:312`
- Test: `src/ai/analysis/card_semantics.rs`

**Interfaces:**
- Consumes: `card_definition(CardId) -> CardDefinition`, `PlayEffect`, `Mechanic`.
- Produces: Reaper facts containing Strength scaling, area damage, and current-HP recovery.

- [ ] **Step 1: Add the failing Reaper semantic contract**

Append this test module to `src/ai/analysis/card_semantics.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaper_semantics_include_aoe_sustain_and_strength_scaling() {
        let definition = card_definition(CardId::Reaper);

        assert!(definition
            .play_effects
            .contains(&PlayEffect::DamageUses(Mechanic::Strength)));
        assert!(definition.play_effects.contains(&PlayEffect::AreaDamage));
        assert!(definition
            .play_effects
            .contains(&PlayEffect::RecoverCurrentHp));
    }
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --lib reaper_semantics_include_aoe_sustain_and_strength_scaling
```

Expected: the test fails because Reaper lacks `AreaDamage` and `RecoverCurrentHp`; it must not fail from compilation or test discovery.

- [ ] **Step 3: Split Reaper from the generic Strength-payoff arm**

Replace:

```rust
        HeavyBlade | SwordBoomerang | Reaper => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .effect(DamageUses(Strength)),
```

with:

```rust
        HeavyBlade | SwordBoomerang => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .effect(DamageUses(Strength)),
        Reaper => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .effect(DamageUses(Strength))
            .effect(AreaDamage)
            .effect(RecoverCurrentHp),
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```powershell
cargo test --lib reaper_semantics_include_aoe_sustain_and_strength_scaling
```

Expected: one matching test passes with zero failures.

- [ ] **Step 5: Commit the Reaper fact repair**

```powershell
git add src/ai/analysis/card_semantics.rs
git commit -m "fix: model Reaper sustain semantics"
```

---

### Task 2: Model Dark Shackles' immediate mitigation

**Files:**
- Modify: `src/ai/analysis/card_semantics.rs:485`
- Test: `src/ai/analysis/card_semantics.rs`

**Interfaces:**
- Consumes: the test module introduced in Task 1.
- Produces: Dark Shackles facts containing enemy Strength reduction and self-exhaust.

- [ ] **Step 1: Add the failing Dark Shackles contract inside the existing test module**

Add after the Reaper test:

```rust
    #[test]
    fn dark_shackles_semantics_include_strength_down_and_self_exhaust() {
        let definition = card_definition(CardId::DarkShackles);

        assert!(definition
            .play_effects
            .contains(&PlayEffect::Provide(Mechanic::EnemyStrengthDown)));
        assert!(definition
            .play_effects
            .contains(&PlayEffect::ExhaustsSelf));
    }
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --lib dark_shackles_semantics_include_strength_down_and_self_exhaust
```

Expected: the test fails because the default empty `CardDefinition` has neither required effect.

- [ ] **Step 3: Add the minimal Dark Shackles definition**

Immediately after the existing `Intimidate` arm, add:

```rust
        DarkShackles => CardDefinition::new(card)
            .provides(EnemyStrengthDown)
            .effect(ExhaustsSelf),
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```powershell
cargo test --lib dark_shackles_semantics_include_strength_down_and_self_exhaust
```

Expected: one matching test passes with zero failures.

- [ ] **Step 5: Commit the Dark Shackles fact repair**

```powershell
git add src/ai/analysis/card_semantics.rs
git commit -m "fix: model Dark Shackles mitigation"
```

---

### Task 3: Remove false direct Strength reduction from Shockwave

**Files:**
- Modify: `src/ai/analysis/card_semantics.rs:405`
- Modify: `src/ai/card_reward_policy_v1/facts.rs:159`
- Test: `src/ai/analysis/card_semantics.rs`
- Test: `src/ai/card_reward_policy_v1/facts.rs`

**Interfaces:**
- Consumes: `card_definition(CardId)` and private legacy `card_facts(&RewardCard)`.
- Produces: both fact systems describe Shockwave as Weak + Vulnerable + self-exhaust, not direct Strength reduction.

- [ ] **Step 1: Add the failing new-semantics Shockwave contract**

Inside `src/ai/analysis/card_semantics.rs`'s existing test module, add:

```rust
    #[test]
    fn shockwave_semantics_do_not_claim_direct_strength_reduction() {
        let definition = card_definition(CardId::Shockwave);

        assert!(definition
            .play_effects
            .contains(&PlayEffect::Provide(Mechanic::Weak)));
        assert!(definition
            .play_effects
            .contains(&PlayEffect::Provide(Mechanic::Vulnerable)));
        assert!(definition
            .play_effects
            .contains(&PlayEffect::ExhaustsSelf));
        assert!(!definition
            .play_effects
            .contains(&PlayEffect::Provide(Mechanic::EnemyStrengthDown)));
    }
```

- [ ] **Step 2: Add the failing legacy-facts Shockwave contract**

Inside the existing `#[cfg(test)] mod tests` in `src/ai/card_reward_policy_v1/facts.rs`, add:

```rust
    #[test]
    fn shockwave_facts_do_not_claim_direct_strength_reduction() {
        let facts = card_facts(&RewardCard::new(CardId::Shockwave, 0));

        assert!(facts.weak > 0);
        assert!(facts.vulnerable > 0);
        assert_eq!(facts.enemy_strength_down, 0);
    }
```

- [ ] **Step 3: Run both focused tests and verify RED**

Run:

```powershell
cargo test --lib shockwave_semantics_do_not_claim_direct_strength_reduction
cargo test --lib shockwave_facts_do_not_claim_direct_strength_reduction
```

Expected: both tests fail only because Shockwave currently reports `EnemyStrengthDown`.

- [ ] **Step 4: Remove the incorrect new-semantics fact**

Change the Shockwave arm in `src/ai/analysis/card_semantics.rs` from:

```rust
        Shockwave => CardDefinition::new(card)
            .provides(Vulnerable)
            .provides(Weak)
            .provides(EnemyStrengthDown)
            .effect(ExhaustsSelf),
```

to:

```rust
        Shockwave => CardDefinition::new(card)
            .provides(Vulnerable)
            .provides(Weak)
            .effect(ExhaustsSelf),
```

- [ ] **Step 5: Remove the incorrect legacy fact**

Change `enemy_strength_down` in `src/ai/card_reward_policy_v1/facts.rs` from:

```rust
        CardId::Disarm | CardId::Shockwave | CardId::DarkShackles | CardId::PiercingWail => magic,
```

to:

```rust
        CardId::Disarm | CardId::DarkShackles | CardId::PiercingWail => magic,
```

- [ ] **Step 6: Run both focused tests and verify GREEN**

Run:

```powershell
cargo test --lib shockwave_semantics_do_not_claim_direct_strength_reduction
cargo test --lib shockwave_facts_do_not_claim_direct_strength_reduction
```

Expected: both matching tests pass with zero failures.

- [ ] **Step 7: Commit the Shockwave fact repair**

```powershell
git add src/ai/analysis/card_semantics.rs src/ai/card_reward_policy_v1/facts.rs
git commit -m "fix: correct Shockwave mitigation semantics"
```

---

### Task 4: Verify the completed semantic slice

**Files:**
- Verify: `src/ai/analysis/card_semantics.rs`
- Verify: `src/ai/card_reward_policy_v1/facts.rs`

**Interfaces:**
- Consumes: all semantic changes and contracts from Tasks 1-3.
- Produces: formatting, library, architecture, and repository-cleanliness evidence.

- [ ] **Step 1: Check formatting and textual diff integrity**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
```

Expected: both commands exit successfully with no output indicating an error.

- [ ] **Step 2: Run the full library suite**

Run:

```powershell
cargo test --lib
```

Expected: all library tests pass with zero failures.

- [ ] **Step 3: Run the architecture boundary suite**

Run:

```powershell
cargo test --test architecture_runtime_boundaries
```

Expected: all architecture boundary tests pass with zero failures.

- [ ] **Step 4: Verify scope and worktree state**

Run:

```powershell
git show --stat --oneline HEAD~3..HEAD
git status --short --branch
```

Expected: the implementation commits only touch the two semantic source files, the current plan/spec commits remain present, and the tracked worktree is clean.
