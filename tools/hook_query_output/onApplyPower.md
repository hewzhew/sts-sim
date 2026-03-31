# Hook Query: `onApplyPower`

## 1. Base Class Definition (1 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L371

```java
public void onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source) {
    }
```

## 2. Engine Call Sites (2)

### `ApplyPoisonOnRandomMonsterAction.update()`

File: `actions\common\ApplyPoisonOnRandomMonsterAction.java`

```java
// --- Line 61 ---
                this.powerToApply = new PoisonPower(this.target, this.source, this.amount);
                if (this.source != null) {
                    for (AbstractPower abstractPower : this.source.powers) {
>>>                     abstractPower.onApplyPower(this.powerToApply, this.target, this.source);
                    }
                }
                if (this.target.hasPower("Artifact")) {
```

**Iterates**: `this.source.powers` (ordered — sensitive to iteration order)

**Hardcoded checks in this method:**

- L64: `hasPower("Artifact")`

### `ApplyPowerAction.update()`

File: `actions\common\ApplyPowerAction.java`

```java
// --- Line 102 ---
                }
                if (this.source != null) {
                    for (AbstractPower abstractPower : this.source.powers) {
>>>                     abstractPower.onApplyPower(this.powerToApply, this.target, this.source);
                    }
                }
                if (AbstractDungeon.player.hasRelic("Champion Belt") && this.source != null && this.source.isPlayer && this.target != this.source && this.powerToApply.ID.equals("Vulnerable") && !this.target.hasPower("Artifact")) {
```

**Iterates**: `this.source.powers` (ordered — sensitive to iteration order)

**Hardcoded checks in this method:**

- L105: `hasRelic("Champion Belt")`
- L105: `hasPower("Artifact")`
- L113: `hasRelic("Ginger")`
- L119: `hasRelic("Turnip")`
- L125: `hasPower("Artifact")`

## 3. Subclass Overrides (1)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| SadisticPower | AbstractPower | `powers\SadisticPower.java` | 35-41 | ✅ | QUEUES_ACTIONS(addToBot) |

### SadisticPower `((AbstractPower power, AbstractCreature target, AbstractCreature source))` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\SadisticPower.java` L35-41

```java
@Override
    public void onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source) {
        if (power.type == AbstractPower.PowerType.DEBUFF && !power.ID.equals("Shackled") && source == this.owner && target != this.owner && !target.hasPower("Artifact")) {
            this.flash();
            this.addToBot(new DamageAction(target, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

## 4. Rust Current Status (18 refs)

- `D:\rust\sts_simulator\src\combat.rs:66:    pub on_apply_power: smallvec::SmallVec<[usize; 4]>,`
- `D:\rust\sts_simulator\src\combat.rs:153:        if sub.on_apply_power { self.relic_buses.on_apply_power.push(index); }`
- `D:\rust\sts_simulator\src\engine\action_handlers.rs:245:                    let hook_actions = crate::content::powers::resolve_power_on_apply_power(`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:567:pub fn resolve_power_on_apply_power(`
- `D:\rust\sts_simulator\src\content\relics\champion_belt.rs:9:    pub fn on_apply_power(power_id: PowerId, target: EntityId) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\ginger.rs:7:/// The game checks this at power application time. We hook into `on_apply_power` (to be created/used depending on Power architecture)`
- `D:\rust\sts_simulator\src\content\relics\ginger.rs:15:pub fn on_apply_power(_state: &CombatState, _power_id: PowerId) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:346:pub fn on_apply_power(state: &CombatState, power_id: crate::content::powers::PowerId, target: crate::core::EntityId) -> smallvec::SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:349:    for &relic_index in &buses.on_apply_power {`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:352:            RelicId::ChampionBelt => actions.extend(crate::content::relics::champion_belt::ChampionBelt::on_apply_power(power_id, target)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:353:            RelicId::SneckoSkull => actions.extend(crate::content::relics::snecko_skull::on_apply_power(power_id)),`
- `D:\rust\sts_simulator\src\content\relics\hooks.rs:354:            _ => unreachable!("Relic present in on_apply_power bus but unhandled in hooks.rs match arm"),`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:539:    pub on_apply_power: bool,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:606:        RelicId::ChampionBelt => sub.on_apply_power = true,`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:731:        RelicId::SneckoSkull => sub.on_apply_power = true,`
- `D:\rust\sts_simulator\src\content\relics\snecko_skull.rs:6:pub fn on_apply_power(power_id: crate::content::powers::PowerId) -> SmallVec<[ActionInfo; 4]> {`
- `D:\rust\sts_simulator\src\content\relics\snecko_skull.rs:10:        // Technically, this shouldn't be an Action dispatch inside on_apply_power to "Apply 1 more Poison",`
- `D:\rust\sts_simulator\src\content\relics\snecko_skull.rs:11:        // because that might cause an infinite loop of on_apply_power. Wait!`

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| SadisticPower | SadisticPower | ✅ IMPLEMENTED |

