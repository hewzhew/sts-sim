# Hook Query: `onAttack`

## 1. Base Class Definition (2 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L262

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

**Class**: `AbstractRelic` — `relics\AbstractRelic.java` L565

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

## 2. Engine Call Sites (0)

*No call sites found outside base classes.*

## 3. Subclass Overrides (1)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| EnvenomPower | AbstractPower | `powers\EnvenomPower.java` | 35-41 | ✅ | QUEUES_ACTIONS(addToTop) |

### EnvenomPower `((DamageInfo info, int damageAmount, AbstractCreature target))` ⚠️ QUEUES_ACTIONS(addToTop)

File: `powers\EnvenomPower.java` L35-41

```java
@Override
    public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
        if (damageAmount > 0 && target != this.owner && info.type == DamageInfo.DamageType.NORMAL) {
            this.flash();
            this.addToTop(new ApplyPowerAction(target, this.owner, (AbstractPower)new PoisonPower(target, this.owner, this.amount), this.amount, true));
        }
    }
```

## 4. Rust Current Status (70 refs)

- `D:\rust\sts_simulator\src\action.rs:190:    pub on_attack_to_change_damage: Vec<DamageModifierId>, // Attacker changes damage`
- `D:\rust\sts_simulator\src\action.rs:191:    pub on_attacked_to_change_damage: Vec<DamageModifierId>, // Defender changes damage`
- `D:\rust\sts_simulator\src\action.rs:192:    pub on_attack_hooks: Vec<HookId>,                      // Execute after damage is calculated`
- `D:\rust\sts_simulator\src\action.rs:193:    pub on_attacked_hooks: Vec<HookId>,                    // Execute after being attacked`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:92:                // on_attacked power hooks for player — fires regardless of block (matches Java)`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:94:                // Thorns/HpLoss damage does NOT trigger on_attacked hooks (prevents cascading)`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:95:                let should_fire_on_attacked = info.damage_type != crate::action::DamageType::Thorns `
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:97:                if should_fire_on_attacked {`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:100:                        let hook_actions = crate::content::powers::resolve_power_on_attacked(`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:109:                } // end should_fire_on_attacked`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:141:                // on_attacked power hooks for monster (CurlUp, Thorns, Angry, etc.)`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:142:                // Java: AbstractCreature.damage() skips on_attacked for THORNS and HP_LOSS`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:144:                let should_fire_monster_on_attacked = info.damage_type != crate::action::DamageType::Thorns`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:146:                if should_fire_monster_on_attacked {`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:150:                        let hook_actions = crate::content::powers::resolve_power_on_attacked(`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:174:                } // end should_fire_monster_on_attacked`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:436:            // This goes through the full damage pipeline including on_attacked hooks (Malleable, etc.)`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:553:            let non_attacks: Vec<u32> = state.hand.iter()`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:560:            let count = non_attacks.len() as i32;`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:564:            for uuid in non_attacks {`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:575:            let non_attacks: Vec<u32> = state.hand.iter()`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:582:            for uuid in non_attacks {`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:425:pub fn resolve_power_on_attack(id: PowerId, _state: &CombatState, _owner: crate::core::EntityId, damage: i32, _target: crate::core::EntityId, _power_amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:427:        PowerId::PainfulStabs => core::painful_stabs::on_attack(damage),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:439:pub fn resolve_power_on_attacked(id: PowerId, state: &CombatState, owner: crate::core::EntityId, damage: i32, source: crate::core::EntityId, power_amount: i32) -> smallvec::SmallVec<[crate::action::Action; 2]> {`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:441:        PowerId::FlameBarrier => ironclad::flame_barrier::on_attacked(source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:442:        PowerId::CurlUp => core::curl_up::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:443:        PowerId::Angry => core::angry::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:444:        // SharpHide: moved from on_attacked to on_card_played (Java uses onUseCard, not onAttacked)`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:445:        PowerId::Flight => core::flight::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:446:        PowerId::Malleable => core::malleable::on_attacked(state, owner, damage, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:447:        PowerId::PlatedArmor => core::plated_armor::on_attacked(state, owner, damage, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:448:        PowerId::Thorns => core::thorns::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:449:        PowerId::Shifting => core::shifting::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:450:        PowerId::Reactive => core::reactive::on_attacked(state, owner, damage, source, power_amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:471:pub fn resolve_power_on_attack_to_change_damage(`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:475:        PowerId::Strength => core::strength::on_attack_to_change_damage(current_damage, amount),`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:480:pub fn resolve_power_on_attacked_to_change_damage(`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:486:            core::vulnerable::on_attacked_to_change_damage(current_damage, amount, has_odd_mushroom)`
- `D:\rust\sts_simulator\src\content\relics\boot.rs:4:    pub fn on_attack_to_change_damage(damage: i32) -> i32 {`
- `D:\rust\sts_simulator\src\content\powers\ironclad\flame_barrier.rs:4:pub fn on_attacked(source: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {`
- `D:\rust\sts_simulator\src\content\cards\ironclad\clash.rs:9:    let has_non_attacks = state.hand.iter().any(|c| {`
- `D:\rust\sts_simulator\src\content\cards\ironclad\clash.rs:12:    if has_non_attacks {`
- `D:\rust\sts_simulator\src\content\powers\core\curl_up.rs:5:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\curl_up.rs:19:    // zeroes the CurlUp amount before dispatching on_attacked hooks, so`
- `D:\rust\sts_simulator\src\content\powers\core\angry.rs:5:pub fn on_attacked(_state: &CombatState, owner: crate::core::EntityId, _damage: i32, _source: crate::core::EntityId, power_amount: i32) -> SmallVec<[Action; 2]> {`
- `D:\rust\sts_simulator\src\content\powers\core\flight.rs:13:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\malleable.rs:14:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\painful_stabs.rs:4:pub fn on_attack(damage: i32) -> smallvec::SmallVec<[Action; 2]> {`
- `D:\rust\sts_simulator\src\content\powers\core\plated_armor.rs:17:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\reactive.rs:5:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\shifting.rs:6:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\content\powers\core\strength.rs:9:pub fn on_attack_to_change_damage(current_damage: i32, amount: i32) -> i32 {`
- `D:\rust\sts_simulator\src\content\powers\core\vulnerable.rs:13:pub fn on_attacked_to_change_damage(current_damage: i32, amount: i32, has_odd_mushroom: bool) -> i32 {`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:589:        RelicId::Boot => {}, // Engine native query hook for on_attack_to_change_damage`
- `D:\rust\sts_simulator\src\content\powers\core\thorns.rs:5:pub fn on_attacked(`
- `D:\rust\sts_simulator\src\action.rs:29:    ExhaustAllNonAttack,`
- `D:\rust\sts_simulator\src\action.rs:30:    BlockPerNonAttack { block_per_card: i32 },`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:47:                // trigger onAttacked hooks (Thorns etc.).`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:59:                // Java: Torii.onAttacked() checks: owner != null, type != HP_LOSS,`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:93:                // Java: ThornsPower.onAttacked checks info.type != THORNS && != HP_LOSS`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:104:                            // Java: onAttacked uses addToTop — reactive damage fires before next hit`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:122:                // Boot relic: Java calls onAttackToChangeDamage AFTER decrementBlock()`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:551:        Action::BlockPerNonAttack { block_per_card } => {`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:573:        Action::ExhaustAllNonAttack => {`
- `D:\rust\sts_simulator\src\content\powers\core\beat_of_death.rs:19:        // Wait, Thorns damage type in STS bypasses unblockable? No, Thorns damage type does NOT trigger 'onAttacked' hooks (like Torii) and just hits block, then HP.`
- `D:\rust\sts_simulator\src\content\powers\core\curl_up.rs:14:    // Java CurlUp.onAttacked conditions:`
- `D:\rust\sts_simulator\src\content\powers\core\malleable.rs:6:/// Java MalleablePower.onAttacked():`
- `D:\rust\sts_simulator\src\content\cards\ironclad\second_wind.rs:8:            action: Action::BlockPerNonAttack {`
- `D:\rust\sts_simulator\src\content\cards\ironclad\sever_soul.rs:9:            action: Action::ExhaustAllNonAttack,`

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| EnvenomPower | Envenom | ❌ MISSING (no PowerId) |

