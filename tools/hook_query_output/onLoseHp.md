# Hook Query: `onLoseHp`

## 1. Base Class Definition (2 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L381

```java
public int onLoseHp(int damageAmount) {
        return damageAmount;
    }
```

**Class**: `AbstractRelic` — `relics\AbstractRelic.java` L959

```java
public void onLoseHp(int damageAmount) {
    }
```

## 2. Engine Call Sites (0)

*No call sites found outside base classes.*

## 3. Subclass Overrides (1)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| DEPRECATEDCondensePower | AbstractPower | `powers\deprecated\DEPRECATEDCondensePower.java` | 27-34 | ✅ | pure |

### DEPRECATEDCondensePower `((int damageAmount))`

File: `powers\deprecated\DEPRECATEDCondensePower.java` L27-34

```java
@Override
    public int onLoseHp(int damageAmount) {
        if (damageAmount > this.amount) {
            this.flash();
            return this.amount;
        }
        return damageAmount;
    }
```

## 4. Rust Current Status (24 refs)

- `D:\rust\sts_simulator\src\combat.rs:64:    pub on_lose_hp: smallvec::SmallVec<[usize; 4]>,`
- `D:\rust\sts_simulator\src\combat.rs:151:        if sub.on_lose_hp { self.relic_buses.on_lose_hp.push(index); }`
- `D:\rust\sts_simulator\src\action.rs:194:    pub on_lose_hp_last: Vec<DamageModifierId>,            // Final modifier (Tungsten Rod)`
- `D:\rust\sts_simulator\src\action.rs:195:    pub on_lose_hp: Vec<HookId>,`
- `D:\rust\sts_simulator\src\content\relics\centennial_puzzle.rs:7:    pub fn on_lose_hp(used_up: bool) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\emotion_chip.rs:15:pub fn on_lose_hp(_state: &CombatState, relic: &mut RelicState, amount: i32) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:172:pub fn on_lose_hp(state: &CombatState, amount: i32) -> smallvec::SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:175:    for &relic_index in &buses.on_lose_hp {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:178:            RelicId::CentennialPuzzle => actions.extend(crate::content::relics::centennial_puzzle::CentennialPuzzle::on_lose_hp(relic_state.used_up)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:179:            RelicId::EmotionChip => actions.extend(crate::content::relics::emotion_chip::on_lose_hp(state, &mut state.player.relics.clone()[relic_index], amount)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:180:            RelicId::LizardTail => actions.extend(crate::content::relics::lizard_tail::on_lose_hp(state, relic_state.used_up)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:181:            RelicId::SelfFormingClay => actions.extend(crate::content::relics::self_forming_clay::on_lose_hp()),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:182:            RelicId::TungstenRod => actions.extend(crate::content::relics::tungsten_rod::on_lose_hp(amount)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:192:            _ => unreachable!("Relic present in on_lose_hp bus but unhandled in hooks.rs match arm"),`
- `D:\rust\sts_simulator\src\content\relics\lizard_tail.rs:9:pub fn on_lose_hp(state: &CombatState, used: bool) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:537:    pub on_lose_hp: bool,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:604:        RelicId::CentennialPuzzle => sub.on_lose_hp = true,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:630:            sub.on_lose_hp = true;`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:695:        RelicId::LizardTail => sub.on_lose_hp = true,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:725:        RelicId::SelfFormingClay => sub.on_lose_hp = true,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:737:        RelicId::RunicCube => sub.on_lose_hp = true,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:767:        RelicId::TungstenRod => sub.on_lose_hp = true,`
- `D:\rust\sts_simulator\src\content\relics\self_forming_clay.rs:5:pub fn on_lose_hp() -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\tungsten_rod.rs:5:pub fn on_lose_hp(_amount: i32) -> SmallVec<[ActionInfo; 4]> {`

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| DEPRECATEDCondensePower | DEPRECATEDCondense | ❌ MISSING (no PowerId) |

