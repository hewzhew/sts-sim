# Card Command API Reference

> ⚠️ **Partially stale** (last accurate: Jan 2026). File paths now use `engine/commands.rs` (split from monolithic `engine.rs`).
> The JSON format specs and command definitions remain accurate.

> 本文档定义了 `cards.json` 中 `logic.commands` 数组里每个命令对象的**规范格式**。  
> 所有卡牌数据必须遵守此接口定义。引擎 (`engine/commands.rs`) 根据此文档解析与执行。

## 目录

- [通用规范](#通用规范)
- [伤害类](#伤害类)
- [防御类](#防御类)
- [状态效果类](#状态效果类)
- [卡牌操作类](#卡牌操作类)
- [增益/减益类](#增益减益类)
- [生命值类](#生命值类)
- [能量类](#能量类)
- [标记类](#标记类)
- [特殊类](#特殊类)
- [实现状态总览](#实现状态总览)

---

## 通用规范

### 升级值规则
大多数命令使用 `base` / `upgrade` 模式：
```json
{ "type": "DealDamage", "params": { "base": 6, "upgrade": 9 } }
```
- `base`: 未升级时的值
- `upgrade`: 升级后的值
- 引擎根据 `CardInstance.upgraded` 选择使用哪个值

### 命名约定
| 术语 | 含义 | Rust 类型 |
|------|------|-----------|
| `base` / `upgrade` | 未升级值 / 升级值 | `i32` |
| `times` | 多段攻击次数 | `Option<i32>` (默认 1) |
| `status` | 状态效果名 (如 "Vulnerable") | `String` |
| `buff` | 增益效果名 (如 "Strength") | `String` |
| `scaling` | 特殊数值来源 (如 "Block") | `Option<String>` |

---

## 伤害类

### DealDamage
对单个目标造成伤害。

```json
{ "type": "DealDamage", "params": { "base": 6, "upgrade": 9 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `base` | i32 | ✅ | 基础伤害 |
| `upgrade` | i32 | ✅ | 升级伤害 |
| `times` | i32? | ❌ | 攻击次数 (默认 1) |
| `scaling` | string? | ❌ | 特殊缩放: `"Block"` = 伤害等于当前格挡 (如 Body Slam) |

**伤害计算**: `(base + strength × str_mult + vigor) × weak_mult`

**引擎状态**: ✅ 完全实现

**示例卡牌**: Strike (6/9), Bash (8/10), Body Slam (scaling=Block)

---

### DealDamageAll
对所有敌人造成伤害。

```json
{ "type": "DealDamageAll", "params": { "base": 8, "upgrade": 11 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `base` | i32 | ✅ | 基础伤害 |
| `upgrade` | i32 | ✅ | 升级伤害 |
| `times` | i32? | ❌ | 攻击次数 (默认 1) |

**引擎状态**: ✅ 完全实现

**示例卡牌**: Cleave (8/11), Whirlwind

---

### DealDamageRandom
对随机敌人造成多段伤害。

```json
{ "type": "DealDamageRandom", "params": { "base": 3, "upgrade": 3, "times": 3, "times_upgrade": 4 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `base` | i32 | ✅ | 每次伤害 |
| `upgrade` | i32 | ✅ | 升级每次伤害 |
| `times` | i32? | ❌ | 攻击次数 (base) |
| `times_upgrade` | i32? | ❌ | 升级攻击次数 |

**引擎状态**: ⚠️ 需实现

**示例卡牌**: Sword Boomerang (3dmg × 3/4次)

> **注意**: JSON 中不得使用 `damage_base`/`hits_base` 等非标准字段。一律用 `base`/`upgrade`/`times`/`times_upgrade`。

---

### StrengthMultiplier
修改力量在伤害计算中的倍率。必须与 `DealDamage` 搭配使用。

```json
{ "type": "StrengthMultiplier", "params": { "base": 3, "upgrade": 5 } }
```

**引擎状态**: ✅ 完全实现 (通过预处理设置 `card_modifiers.strength_multiplier`)

**示例卡牌**: Heavy Blade (3× / 5×)

---

## 防御类

### GainBlock
为玩家获得格挡。

```json
{ "type": "GainBlock", "params": { "base": 5, "upgrade": 8 } }
```

**引擎状态**: ✅ 完全实现

---

### DoubleBlock
将当前格挡翻倍。

```json
{ "type": "DoubleBlock", "params": {} }
```

**引擎状态**: ✅ 完全实现

---

## 状态效果类

### ApplyStatus
对目标敌人施加状态效果。

```json
{ "type": "ApplyStatus", "params": { "status": "Vulnerable", "base": 2, "upgrade": 3 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `status` | string | ✅ | 状态名: Vulnerable, Weak, Poison 等 |
| `base` | i32 | ✅ | 基础层数 |
| `upgrade` | i32 | ✅ | 升级层数 |

**引擎状态**: ✅ 完全实现

---

### ApplyStatusAll
对所有敌人施加状态效果。

```json
{ "type": "ApplyStatusAll", "params": { "status": "Vulnerable", "base": 1, "upgrade": 1 } }
```

**引擎状态**: ✅ 完全实现

**示例卡牌**: Thunderclap (Vulnerable ×1)

---

## 卡牌操作类

### DrawCards
从抽牌堆抽卡。

```json
{ "type": "DrawCards", "params": { "base": 1, "upgrade": 2 } }
```

**引擎状态**: ✅ 完全实现 (使用 `state.draw_cards()`)

---

### ExhaustSelf
打出后耗尽此卡。

```json
{ "type": "ExhaustSelf", "params": {} }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `base_only` | bool | ❌ | 仅未升级时耗尽 |
| `upgrade_only` | bool | ❌ | 仅升级后耗尽 |

**引擎状态**: ✅ 完全实现

---

### ExhaustCards
耗尽指定数量的卡牌。

```json
{ "type": "ExhaustCards", "params": { "base": 1, "upgrade": 1, "random": true } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `base` | i32 | ✅ | 耗尽数量 |
| `upgrade` | i32 | ✅ | 升级后耗尽数量 |
| `pile` | string? | ❌ | 来源牌堆 (默认 "hand") |
| `select_mode` | string? | ❌ | 选择模式: "random", "choose" |

> **注意**: JSON 中的 `"random": true` 字段会被忽略。请使用 `"select_mode": "random"` 指定随机选择。

**引擎状态**: ✅ 完全实现

**示例卡牌**: True Grit (block + exhaust random)

---

### AddCard
向指定牌堆添加一张卡。

```json
{ "type": "AddCard", "params": { "card": "this card", "destination": "discard pile" } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `card` | string | ✅ | 卡牌标识: `"this card"`, 或卡牌 ID (如 `"Wound"`) |
| `destination` | string | ✅ | 目的地: `"discard pile"`, `"draw pile"`, `"hand"` |
| `count` | i32 | ❌ | 数量 (默认 1) |

**引擎状态**: ⚠️ 需实现 (当前仅打印日志)

**示例卡牌**: Anger (添加自身副本到弃牌堆)

---

### ShuffleInto
将卡牌洗入牌堆。

```json
{ "type": "ShuffleInto", "params": { "card": "Wound" } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `card` | string? | ❌ | 卡牌 ID (null = 将弃牌堆洗入抽牌堆) |
| `destination` | string? | ❌ | 目的地 (默认 "draw pile") |
| `count` | i32? | ❌ | 数量 (默认 1) |

**引擎状态**: ✅ 完全实现

**示例卡牌**: Wild Strike (洗入 Wound)

---

### PutOnTop
将卡牌放到抽牌堆顶部。

```json
{ "type": "PutOnTop", "params": { "source": "discard pile" } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `source` | string? | ❌ | 来源: "discard pile", "hand" (默认 "discard pile") |
| `select_mode` | string? | ❌ | 选择模式 (默认 "choose") |
| `count` | i32? | ❌ | 数量 (默认 1) |

**引擎状态**: ✅ 完全实现

**示例卡牌**: Headbutt (弃牌堆 → 抽牌堆顶)

---

### MoveCard
在牌堆之间移动卡牌。

```json
{ "type": "MoveCard", "params": { "from": "hand", "to": "draw_pile", "insert_at": "top", "select": { "mode": "Choose", "count": 1 } } }
```

**引擎状态**: ✅ 完全实现

**示例卡牌**: Warcry (手牌 → 抽牌堆顶)

---

### UpgradeCards
升级指定位置的卡牌。

```json
{ "type": "UpgradeCards", "params": { "amount_base": 1, "amount_upgrade": "ALL", "target": "Hand" } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `amount_base` | int \| "ALL" | ✅ | 基础升级数量 |
| `amount_upgrade` | int \| "ALL" | ✅ | 升级后升级数量 |
| `target` | string | ✅ | 位置: "Hand", "DrawPile", "DiscardPile" |

**引擎状态**: ✅ 完全实现

**示例卡牌**: Armaments (升1张 / 升全部)

---

### PlayTopCard
打出抽牌堆顶部的卡牌。

```json
{ "type": "PlayTopCard", "params": {} }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `count` | i32? | ❌ | 打出数量 (默认 1) |
| `exhaust` | bool | ❌ | 打出后是否耗尽 (默认 false → 但 Havoc 文本暗示应耗尽) |

**引擎状态**: ⚠️ 需实现

**示例卡牌**: Havoc

---

## 增益/减益类

### GainBuff
获得增益效果。**立即生效，永久存在直到被移除。**

```json
{ "type": "GainBuff", "params": { "buff": "Strength", "base": 2, "upgrade": 4 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `buff` | string | ✅ | 增益名: Strength, Dexterity, Metallicize 等 |
| `base` | i32 | ✅ | 基础层数 |
| `upgrade` | i32 | ✅ | 升级层数 |

**引擎状态**: ✅ 完全实现 (使用 `player.apply_status()`)

---

### LoseBuff
移除增益层数。

```json
{ "type": "LoseBuff", "params": { "buff": "Strength", "amount": 2 } }
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `buff` | string | ✅ | 要移除的增益名 |
| `amount` | i32 | ✅ | 移除层数 |
| `all` | bool | ❌ | 移除全部 (默认 false) |

**引擎状态**: ✅ 完全实现

**示例卡牌**: Flex (搭配回合结束触发)

> **⚠️ 重要**: `LoseBuff` 的 JSON 参数使用 `amount`（非 `base`/`upgrade`）。若需回合结束触发，必须使用 `end_of_turn` 字段而非与 `GainBuff` 顺序执行。

---

### ApplyPower
施加能力（类似 GainBuff 但参数格式不同）。

```json
{ "type": "ApplyPower", "params": { "power": "Metallicize", "base": 3, "upgrade": 4 } }
```

**引擎状态**: ✅ 完全实现

---

### DoubleBuff
将某个增益翻倍。

```json
{ "type": "DoubleBuff", "params": { "buff": "Strength" } }
```

**引擎状态**: ✅ 完全实现

**示例卡牌**: Limit Break

---

## 生命值类

### LoseHp / GainHp
损失或恢复 HP。

```json
{ "type": "LoseHp", "params": { "base": 3, "upgrade": 2 } }
{ "type": "GainHp", "params": { "base": 6, "upgrade": 8 } }
```

**引擎状态**: ✅ 完全实现

---

## 能量类

### GainEnergy
获得能量。

```json
{ "type": "GainEnergy", "params": { "base": 1, "upgrade": 1 } }
```

**引擎状态**: ✅ 完全实现

---

### DoubleEnergy
将当前能量翻倍。

```json
{ "type": "DoubleEnergy", "params": {} }
```

**引擎状态**: ✅ 完全实现

---

## 标记类

### ExhaustSelf / RetainSelf / InnateSelf / Ethereal / Innate / Retain

这些是卡牌属性标记命令，不改变游戏状态，仅标记卡牌行为。

**引擎状态**: ✅ 已识别处理

---

## 特殊类

### Conditional
条件分支执行。

```json
{
  "type": "Conditional",
  "params": {
    "condition": { "type": "Fatal" },
    "then_do": [{ "type": "GainEnergy", "params": { "base": 3 } }]
  }
}
```

**引擎状态**: ✅ 完全实现

---

### Unplayable
标记卡牌不可打出。

**引擎状态**: ✅ 完全实现

---

## 实现状态总览

| 状态 | 命令 |
|------|------|
| ✅ 完全实现 | DealDamage, DealDamageAll, GainBlock, ApplyStatus, ApplyStatusAll, DrawCards, GainEnergy, UpgradeCards, ExhaustSelf, ExhaustCards, ExhaustCard, GainBuff, LoseBuff, DoubleBuff, LoseHp, GainHp, StrengthMultiplier, DoubleBlock, DoubleEnergy, ApplyPower, ApplyBuff, ApplyDebuff, MoveCard, ShuffleInto, PutOnTop, Conditional, Discard, DiscardCards, GainMaxHP, GainFocus |
| ⚠️ 需实现 | AddCard (stub), PlayTopCard (stub), DealDamageRandom (stub) |
| ❌ 未实现 | ChannelOrb, EvokeOrb, EnterStance, ExitStance, Scry, GainMantra, Discover, DrawUntil, DrawUntilFull, Execute, GainGold, ObtainPotion, RemoveBlock, SetCostAll, SetCostRandom, UpgradeCard, ExtraTurn, MultiplyStatus, DoubleStatus, IncreaseDamage |

---

## JSON 数据规范

### 禁止的非标准字段名

以下字段名**不允许**出现在 `cards.json` 中：

| ❌ 错误字段 | ✅ 正确字段 | 相关命令 |
|------------|-----------|---------|
| `damage_base` | `base` | DealDamage, DealDamageRandom |
| `damage_upgrade` | `upgrade` | DealDamage, DealDamageRandom |
| `hits_base` | `times` | DealDamage (`times` 字段) |
| `hits_upgrade` | `times_upgrade` | DealDamageRandom |

### 回合结束触发 (End-of-Turn Triggers)

某些卡牌的命令需要在回合结束时执行（如 Flex 的 LoseBuff）。  
使用 `end_of_turn: true` 字段标记：

```json
{ "type": "LoseBuff", "params": { "buff": "Strength", "amount": 2, "end_of_turn": true } }
```

引擎会将带 `end_of_turn: true` 的命令注册到回合结束队列，而非立即执行。
