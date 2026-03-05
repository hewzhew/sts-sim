# API Implementation Tracker

> ⚠️ **Partially stale** (last accurate: Jan 2026). Many items marked TODO are now implemented.
> For current coverage, see `ENGINE_STATUS.md` or run `scripts/validate_cards.py`.

此文档追踪从 `cards.json` 补丁生成的所有API类型及其在Rust中的实现状态。

**生成日期**: 2026-01-21
**最后更新**: 2026-01-21 (Phase 2.5 完成)
**补丁覆盖率**: 98/109 (89.9%)
**剩余手动处理**: 11张卡牌

---

## 📋 命令类型 (CardCommand)

### ✅ 已完整实现 (有完整执行逻辑)

| Command | 使用次数 | 实现状态 | 备注 |
|---------|---------|----------|------|
| DealDamage | 111x | ✅ 完整 | 支持Vigor、Fatal追踪、多段攻击 |
| DealDamageAll | 20x | ✅ 完整 | AOE攻击，支持Vigor |
| GainBlock | 62x | ✅ 完整 | 获得格挡 |
| ApplyStatus | 23x | ✅ 完整 | 施加状态 |
| ApplyStatusAll | 4x | ✅ 完整 | AOE状态 |
| DrawCards | 43x | ✅ 完整 | 抽牌 |
| GainEnergy | 25x | ✅ 完整 | 获得能量 |
| ExhaustSelf | 81x | ✅ 完整 | 消耗自身 |
| AddCard | 24x | ✅ 完整 | 添加卡牌 |
| GainBuff | 29x | ✅ 完整 | 获得buff |
| DoubleBuff | 1x | ✅ 完整 | 翻倍buff |
| ChannelOrb | 17x | ✅ stub | 充能球体 (需Orb系统) |
| EvokeOrb | 3x | ✅ stub | 激发球体 (需Orb系统) |
| EnterStance | 8x | ✅ stub | 进入姿态 (需Stance系统) |
| ExitStance | 3x | ✅ stub | 退出姿态 (需Stance系统) |
| Scry | 4x | ✅ stub | 预言 (需UI交互) |
| ApplyPower | 11x | ✅ 完整 | 应用能力 |
| UpgradeCards | 2x | ✅ 完整 | 升级卡牌 |
| LoseHP | 5x | ✅ 完整 | 失去生命 |
| Unplayable | 20x | ✅ 完整 | 不可打出 |
| Heal | 4x | ✅ 完整 | 治疗 |
| DoubleBlock | 1x | ✅ 完整 | 翻倍格挡 |
| DoubleEnergy | 1x | ✅ 完整 | 翻倍能量 |
| GainMaxHP | 1x | ✅ 完整 | 获得最大生命 |
| Draw | 1x | ✅ 完整 | 抽牌(另一格式) |
| MultiHit | - | ✅ 完整 | 多段攻击，支持Vigor |
| **ApplyBuff** | 9x | ✅ **完整** | **Vigor, DoubleTap, Burst, Blur, Intangible** |
| **ApplyDebuff** | 1x | ✅ **完整** | **应用debuff到目标** |
| **Conditional** | 4x | ✅ **完整** | **Fatal, EnemyHpBelow, HandFull等条件** |

### 🟡 已定义但执行逻辑为TODO (打印日志)

| Command | 使用次数 | 优先级 | 描述 |
|---------|---------|--------|------|
| Discard | 10x | 🔴 高 | 弃牌 |
| MoveCard | 10x | 🔴 高 | 移动卡牌 (手牌/牌堆) |
| ShuffleInto | 14x | 🔴 高 | 洗入牌堆 |
| EndTurn | 3x | 🟡 中 | 结束回合 |
| PutOnTop | 3x | 🟡 中 | 放置牌堆顶 |
| ExhaustCard | 2x | 🟡 中 | 消耗其他卡牌 |
| ExhaustCards | 5x | 🟡 中 | 消耗多张卡牌 |
| IncreaseDamage | 2x | 🟡 中 | 增加伤害 (Rampage) |
| LoseBuff | 9x | 🟡 中 | 失去buff |
| RemoveEnemyBuff | 3x | 🟡 中 | 移除敌人buff (Artifact) |
| DealDamageRandom | 2x | 🟡 中 | 随机目标伤害 |
| PlayTopCard | 2x | 🟢 低 | 打出顶牌 |
| Discover | 1x | 🟢 低 | 发现卡牌 |
| DrawUntil | 1x | 🟢 低 | 抽牌直到 |
| DrawUntilFull | 1x | 🟢 低 | 抽满手牌 |
| Execute | 1x | 🟢 低 | 处决 (Judgment) |
| GainGold | 2x | 🟢 低 | 获得金币 |
| ObtainPotion | 1x | 🟢 低 | 获得药水 |
| RemoveBlock | 1x | 🟢 低 | 移除格挡 |
| SetCostAll | 1x | 🟢 低 | 设置所有费用 |
| SetCostRandom | 1x | 🟢 低 | 随机设置费用 |
| UpgradeCard | 1x | 🟢 低 | 升级单卡 |
| ExtraTurn | 1x | 🟢 低 | 额外回合 |
| MultiplyStatus | 1x | 🟢 低 | 翻倍状态 |
| DoubleStatus | 0x | 🟢 低 | 翻倍状态(简化版) |
| Ethereal | 9x | 🟢 低 | 虚无标记 |
| Innate | 16x | 🟢 低 | 固有标记 |
| Retain | 13x | 🟢 低 | 保留标记 |

---

## 🔋 Power类型

Powers 是通过 ApplyPower 命令应用的持久效果（整场战斗）。

| Power | 状态 | 描述 |
|-------|------|------|
| Accuracy | 🟡 TODO | 匕首伤害+4 |
| Barricade | 🟡 TODO | 格挡不消失 |
| Buffer | 🟡 TODO | 防止下次伤害 |
| Corruption | 🟡 TODO | 技能费用0消耗 |
| Draw | 🟡 TODO | 每回合抽牌+1 |
| Echo Form | 🟡 TODO | 第一张牌打两次 |
| Electrodynamics | 🟡 TODO | 闪电击中全体 |
| Establishment | 🟡 TODO | 保留时降费 |
| Master Reality | 🟡 TODO | 创建卡时升级 |
| Sadistic Nature | 🟡 TODO | 施加debuff时伤害 |
| Well-Laid Plans | 🟡 TODO | 回合结束保留 |

**实现位置**: `player.powers: HashMap<String, PowerInstance>`

---

## ✨ Buff类型

Buffs 是通过 ApplyBuff/GainBuff 应用的临时/可叠加效果。

| Buff | 状态 | 描述 |
|------|------|------|
| Strength | ✅ 有实现 | 力量 |
| Dexterity | ✅ 有实现 | 敏捷 |
| Vulnerable | ✅ 有实现 | 易伤 |
| Weak | ✅ 有实现 | 虚弱 |
| Artifact | 🟡 TODO | 抵消debuff |
| Blur | 🟡 TODO | 保留格挡1回合 |
| Double Tap | 🟡 TODO | 下次攻击打两次 |
| Focus | 🟡 TODO | 球体专注 |
| Frail | 🟡 TODO | 脆弱(格挡减少) |
| Free Attack | 🟡 TODO | 下次攻击免费 |
| Intangible | 🟡 TODO | 无实体(伤害=1) |
| Mantra | 🟡 TODO | 真言(10层入神) |
| No Draw | 🟡 TODO | 无法抽牌 |
| Plated Armor | 🟡 TODO | 甲(回合获得格挡) |
| Rebound | 🟡 TODO | 打出的牌返回顶 |
| Vigor | 🟡 TODO | 活力(下次攻击+伤害) |

**实现位置**: `player.buffs: HashMap<String, i32>`

---

## 📊 ValueSource类型 (来自 schema_ext.rs)

| Type | 状态 | 描述 |
|------|------|------|
| Fixed | ✅ 在ext | 固定值 |
| CardCount | ✅ 在ext | 按卡牌数量 |
| LastUnblockedDamage | ✅ 在ext | 最后未格挡伤害 |
| CurrentBlock | ✅ 在ext | 当前格挡值 |
| StrengthScaled | ✅ 在ext | 力量缩放 |
| Accumulating | ✅ 在ext | 累积值(Rampage) |

---

## 🚩 Flag类型 (特殊属性)

这些flag表示需要特殊处理的卡牌属性:

### 触发器类型
- `trigger_on_any_exhaust` - 任何卡消耗时触发
- `trigger_on_self_damage` - 从卡牌失去HP时触发
- `trigger_on_stance_change` - 切换姿态时触发
- `trigger_on_scry_return` - 预言时返回
- `on_exhaust_trigger` - 被消耗时触发
- `skill_exhaust_trigger` - 打出技能时消耗

### 条件类型
- `play_condition` - 打出条件
- `condition_enemy_weak` - 敌人虚弱条件
- `condition_discarded_this_turn` - 本回合弃过牌

### 费用/数值修改
- `cost_reduction_per_hp_loss` - 每失去HP降费
- `cost_zero_until_played` - 直到打出前费用0
- `generated_cost` - 生成的卡牌费用
- `self_cost_reduction` - 自身降费
- `diminishing_damage` - 每次使用减少伤害
- `diminishing_block` - 每次使用减少格挡

### 保留相关
- `retain_damage_increase` - 保留时增加伤害
- `retain_block_increase` - 保留时增加格挡
- `retain_cost_reduction` - 保留时降低费用

### 特殊属性
- `infinite_upgrade` - 可无限升级
- `cannot_remove` - 无法移除
- `returns_when_removed` - 移除时返回
- `end_of_turn_damage` - 回合结束伤害
- `escalating_gain` - 每回合增加效果
- `claw_damage_buff` - 爪击伤害增益
- `card_play_limit` - 打牌限制
- `copy_to_draw_top_end_of_turn` - 回合结束复制到顶
- `lose_hp_per_card_in_hand` - 按手牌数失去HP

### 选择相关
- `requires_card_choice` - 需要选牌
- `choose_card_types` - 选择卡牌类型
- `play_chosen_twice_exhaust` - 打出选择的牌两次
- `draw_equal` - 抽等量的牌
- `has_next_turn_effect` - 有下回合效果

---

## 📝 待实现优先级

### Phase 1: 核心命令 (高优先级)
1. `ApplyBuff` - 很多卡牌使用
2. `Conditional` - 条件逻辑核心
### Phase 1.5: 高优先级命令 (✅ 完成!)
1. ✅ `ApplyBuff` - 临时buff系统 (Vigor, DoubleTap, Burst, Blur, Intangible)
2. ✅ `Conditional` - 条件逻辑 (Fatal, EnemyHpBelow, HandFull, PlayerHasStatus等)
3. ✅ `ApplyDebuff` - debuff应用

### Phase 2: 核心卡牌操作 (下一步)
1. `MoveCard` - 卡牌移动核心
2. `Discard` - 弃牌系统
3. `ShuffleInto` - 洗牌系统

### Phase 2.5: 扩展命令 (中优先级) - schema定义完成,执行TODO
4. `EndTurn` - 回合结束
5. `LoseBuff` - 失去buff
6. `ExhaustCard/ExhaustCards` - 消耗其他卡
7. `RemoveEnemyBuff` - 移除敌人buff
8. `PutOnTop` - 放置顶部
9. `IncreaseDamage` - 伤害增加

### Phase 3: 特殊效果 (低优先级) - schema定义完成,执行TODO
10. 金币/药水相关 (GainGold, ObtainPotion)
11. 执行/处决 (Execute)
12. 额外回合 (ExtraTurn)
13. 费用修改 (SetCostAll, SetCostRandom)

### Phase 4: Defect球体系统 - 需要单独实现
- Blizzard, Darkness, Loop, Fission, Recursion, Tempest
- 需要完整的Orb系统 (已有 ChannelOrb, EvokeOrb 定义)

### Phase 5: 复杂系统 - 需要特殊处理
- X费用卡 (Malaise, Tempest, Collect) - 需要动态费用系统
- Pressure Points (印记系统) - Mark buff特殊处理
- Brilliance (真言累积) - 需要Mantra累积追踪

---

## 📌 当前状态总结

### ✅ 已完成 (Phase 2.5)
- `schema.rs`: 所有命令类型定义完成 (35+ CardCommand variants)
- `engine.rs`: 基础命令执行逻辑 + stub实现
- `state.rs`: **新增 TemporaryBuffs 系统** (Vigor, DoubleTap, Burst, Blur, Intangible)
- `state.rs`: **新增 CardPlayModifiers** (支持卡牌复制效果)
- `state.rs`: **新增攻击结果追踪** (last_attack_killed, last_unblocked_damage)
- `engine.rs`: **完整实现 ApplyBuff** (支持ValueSource解析)
- `engine.rs`: **完整实现 Conditional** (支持多种条件类型)
- `engine.rs`: **DealDamage 支持 Vigor 和 Fatal 追踪**
- `engine.rs`: **play_card 支持 DoubleTap/Burst 卡牌复制**
- Release编译通过 + 全部测试通过

### 🟡 下一步
- 卡牌操作命令 (MoveCard, Discard, ShuffleInto)
- Power系统实现

### ❌ 待开始
- Orb系统实现
- Stance系统实现
- X费用卡系统

---

## 🔄 更新日志

- 2026-01-21: 初始创建，补丁覆盖率 89.9%
- 2026-01-21: schema.rs 扩展完成，engine.rs stub添加，编译通过
- 2026-01-21: **Phase 2.5 完成** - ApplyBuff, Conditional, ApplyDebuff 完整实现
  - 新增 TemporaryBuffs 结构 (Vigor, DoubleTap, Burst, Blur, Intangible)
  - 新增 Condition 枚举 (Fatal, EnemyHpBelow, HandFull, PlayerHasStatus等)
  - DealDamage 支持 Vigor 消耗和 Fatal 追踪
  - play_card 支持 DoubleTap/Burst 卡牌复制效果
  - 全部15个测试通过
