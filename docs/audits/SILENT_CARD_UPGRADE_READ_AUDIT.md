# Silent Card Upgrade Read Audit

Generated: `2026-05-16 10:07:54`

Command:

```powershell
python tools/audit_silent_card_upgrade_reads.py
```

Scope: `src/content/cards/silent/*.rs` excluding `mod.rs`.

Scanned direct reads:

- `card.base_magic_num_mut`
- `card.base_damage_mut`
- `card.base_block_mut`
- `card.upgrades`

This report is an audit artifact only. It does not modify gameplay code.

## Summary

- Files with direct reads: `18`
- Direct read occurrences: `24`
- Unclassified files: `0`

## Category Definitions

- **应改为 evaluate_card_for_play**: ordinary `use` logic reads mutable rendered fields directly. The card should evaluate itself at play time or use a shared definition/upgrades helper.
- **合理特殊升级**: direct `card.upgrades` is part of Java action semantics or happens outside normal card play, so `evaluate_card_for_play` is not the right abstraction.
- **疑似测试掩盖风险**: current tests appear to prefill mutable render fields, which can hide the fact that normal play does not evaluate the card.

## 应改为 evaluate_card_for_play

| File | Fields | Lines | Reason | Recommendation |
| --- | --- | --- | --- | --- |
| `src/content/cards/silent/adrenaline.rs` | `base_magic_num_mut, upgrades` | `src/content/cards/silent/adrenaline.rs:9`, `src/content/cards/silent/adrenaline.rs:14` | GainEnergy 的 card.upgrades 分支符合 Java this.upgraded，但 DrawCards 读取 card.base_magic_num_mut；Java 固定 DrawCardAction(p, 2)，不依赖 mutable 渲染字段。 | 保留 energy 的 upgraded 分支；DrawCards 改为 Java 字面量 2 或由统一评价/定义 helper 取得，避免默认 0。 |
| `src/content/cards/silent/backflip.rs` | `base_block_mut, base_magic_num_mut` | `src/content/cards/silent/backflip.rs:10`, `src/content/cards/silent/backflip.rs:15` | 读取 card.base_block_mut 和 card.base_magic_num_mut；这是普通 use 路径，升级后的 block/draw 应在本函数入口通过 card evaluation 得到。 | 改为调用 evaluate_card_for_play(card, state, None)，再读取 evaluated.base_block_mut/base_magic_num_mut。 |
| `src/content/cards/silent/blade_dance.rs` | `base_magic_num_mut` | `src/content/cards/silent/blade_dance.rs:10` | 读取 card.base_magic_num_mut 决定生成 Shiv 数；普通 use 路径不应依赖调用者预先刷新 mutable 字段。 | 改为调用 evaluate_card_for_play(card, state, None) 或统一 magic helper。 |
| `src/content/cards/silent/bouncing_flask.rs` | `base_magic_num_mut` | `src/content/cards/silent/bouncing_flask.rs:10` | 读取 card.base_magic_num_mut 决定随机弹跳次数；普通 use 路径不应依赖外部预评价。 | 改为调用 evaluate_card_for_play(card, state, None) 后使用 evaluated.base_magic_num_mut。 |
| `src/content/cards/silent/burst.rs` | `base_magic_num_mut` | `src/content/cards/silent/burst.rs:12` | 读取 card.base_magic_num_mut 作为 BurstPower amount；升级影响 amount，普通 use 路径应自行评价。 | 改为调用 evaluate_card_for_play(card, state, None)。 |
| `src/content/cards/silent/catalyst.rs` | `base_magic_num_mut` | `src/content/cards/silent/catalyst.rs:18` | 读取 card.base_magic_num_mut 计算毒倍数；升级影响 multiplier，普通 use 路径应自行评价。 | 改为调用 evaluate_card_for_play(card, state, Some(target)) 或定义专用 magic helper 后再计算 extra poison。 |
| `src/content/cards/silent/cloak_and_dagger.rs` | `base_block_mut, base_magic_num_mut` | `src/content/cards/silent/cloak_and_dagger.rs:11`, `src/content/cards/silent/cloak_and_dagger.rs:18` | 读取 card.base_block_mut 和 card.base_magic_num_mut；升级影响格挡和 Shiv 数，普通 use 路径应自行评价。 | 改为调用 evaluate_card_for_play(card, state, None)，并用 evaluated block/magic。 |
| `src/content/cards/silent/dagger_throw.rs` | `base_damage_mut` | `src/content/cards/silent/dagger_throw.rs:20`, `src/content/cards/silent/dagger_throw.rs:22` | 读取 card.base_damage_mut 作为输出伤害，并用 def.base_damage 判断 modified；普通 attack use 应在本函数内 calculate damage。 | 改为调用 evaluate_card_for_play(card, state, Some(target))，并用 evaluated damage。 |
| `src/content/cards/silent/deadly_poison.rs` | `base_magic_num_mut` | `src/content/cards/silent/deadly_poison.rs:17` | 读取 card.base_magic_num_mut 作为 Poison amount；升级影响 poison 数值，普通 use 路径应自行评价。 | 改为调用 evaluate_card_for_play(card, state, Some(target)) 或统一 magic helper。 |
| `src/content/cards/silent/footwork.rs` | `base_magic_num_mut` | `src/content/cards/silent/footwork.rs:12` | 读取 card.base_magic_num_mut 作为 Dexterity amount；普通 power use 应自行评价升级后的 magic。 | 改为调用 evaluate_card_for_play(card, state, None)。 |
| `src/content/cards/silent/noxious_fumes.rs` | `base_magic_num_mut` | `src/content/cards/silent/noxious_fumes.rs:12` | 读取 card.base_magic_num_mut 作为 power amount；升级影响 amount，普通 power use 应自行评价。 | 改为调用 evaluate_card_for_play(card, state, None)。 |
| `src/content/cards/silent/poisoned_stab.rs` | `base_damage_mut, base_magic_num_mut` | `src/content/cards/silent/poisoned_stab.rs:23`, `src/content/cards/silent/poisoned_stab.rs:25`, `src/content/cards/silent/poisoned_stab.rs:34` | 同时读取 card.base_damage_mut 和 card.base_magic_num_mut；伤害和 poison 都受升级/力量等评价影响，普通 attack use 应自行评价。 | 改为调用 evaluate_card_for_play(card, state, Some(target))，统一使用 evaluated damage/magic。 |
| `src/content/cards/silent/survivor.rs` | `base_block_mut` | `src/content/cards/silent/survivor.rs:10` | 读取 card.base_block_mut；Survivor+ 升级格挡，普通 use 路径应自行评价。 | 改为调用 evaluate_card_for_play(card, state, None)。 |

## 合理特殊升级

| File | Fields | Lines | Reason | Recommendation |
| --- | --- | --- | --- | --- |
| `src/content/cards/silent/doppelganger.rs` | `upgrades` | `src/content/cards/silent/doppelganger.rs:8` | Java DoppelgangerAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是动作语义参数，不是数值评价。 | 保留直接传 upgraded，但建议测试覆盖 X 费、free_to_play_once、energy_on_use 与升级组合。 |
| `src/content/cards/silent/malaise.rs` | `upgrades` | `src/content/cards/silent/malaise.rs:15` | Java MalaiseAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是 X 费动作语义参数。 | 保留直接传 upgraded，但建议测试覆盖升级和 energy_on_use 的组合。 |
| `src/content/cards/silent/reflex.rs` | `upgrades` | `src/content/cards/silent/reflex.rs:11` | triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 magic，避免依赖未评价的 mutable magic。 | 保留这种模式，后续可抽成 manual-discard magic helper，并继续保留与弃牌顺序相关测试。 |
| `src/content/cards/silent/storm_of_steel.rs` | `upgrades` | `src/content/cards/silent/storm_of_steel.rs:8` | Java BladeFuryAction 直接接收 this.upgraded；读取 card.upgrades 是生成 Shiv 是否升级的语义参数。 | 保留直接传 upgraded，确保 BladeFuryAction/生成 Shiv 的升级行为有测试覆盖。 |
| `src/content/cards/silent/tactician.rs` | `upgrades` | `src/content/cards/silent/tactician.rs:11` | triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 energy amount，匹配 Java addToTop(new GainEnergyAction(this.magicNumber))。 | 保留这种模式，后续可抽成 manual-discard magic helper，并继续保留 addToTop 顺序测试。 |

## 疑似测试掩盖风险

No files.

## Raw Matches

| File | Line | Field | Code |
| --- | --- | --- | --- |
| `src/content/cards/silent/adrenaline.rs` | 9 | `upgrades` | `amount: if card.upgrades > 0 { 2 } else { 1 },` |
| `src/content/cards/silent/adrenaline.rs` | 14 | `base_magic_num_mut` | `action: Action::DrawCards(card.base_magic_num_mut.max(0) as u32),` |
| `src/content/cards/silent/backflip.rs` | 10 | `base_block_mut` | `amount: card.base_block_mut,` |
| `src/content/cards/silent/backflip.rs` | 15 | `base_magic_num_mut` | `action: Action::DrawCards(card.base_magic_num_mut as u32),` |
| `src/content/cards/silent/blade_dance.rs` | 10 | `base_magic_num_mut` | `amount: card.base_magic_num_mut.max(0) as u8,` |
| `src/content/cards/silent/bouncing_flask.rs` | 10 | `base_magic_num_mut` | `num_times: card.base_magic_num_mut.max(0) as u8,` |
| `src/content/cards/silent/burst.rs` | 12 | `base_magic_num_mut` | `amount: card.base_magic_num_mut,` |
| `src/content/cards/silent/catalyst.rs` | 18 | `base_magic_num_mut` | `let extra = poison * (card.base_magic_num_mut - 1).max(1);` |
| `src/content/cards/silent/cloak_and_dagger.rs` | 11 | `base_block_mut` | `amount: card.base_block_mut,` |
| `src/content/cards/silent/cloak_and_dagger.rs` | 18 | `base_magic_num_mut` | `amount: card.base_magic_num_mut.max(0) as u8,` |
| `src/content/cards/silent/dagger_throw.rs` | 20 | `base_damage_mut` | `output: card.base_damage_mut,` |
| `src/content/cards/silent/dagger_throw.rs` | 22 | `base_damage_mut` | `is_modified: card.base_damage_mut != def.base_damage,` |
| `src/content/cards/silent/deadly_poison.rs` | 17 | `base_magic_num_mut` | `amount: card.base_magic_num_mut,` |
| `src/content/cards/silent/doppelganger.rs` | 8 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/footwork.rs` | 12 | `base_magic_num_mut` | `amount: card.base_magic_num_mut,` |
| `src/content/cards/silent/malaise.rs` | 15 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/noxious_fumes.rs` | 12 | `base_magic_num_mut` | `amount: card.base_magic_num_mut,` |
| `src/content/cards/silent/poisoned_stab.rs` | 23 | `base_damage_mut` | `output: card.base_damage_mut,` |
| `src/content/cards/silent/poisoned_stab.rs` | 25 | `base_damage_mut` | `is_modified: card.base_damage_mut != def.base_damage,` |
| `src/content/cards/silent/poisoned_stab.rs` | 34 | `base_magic_num_mut` | `amount: card.base_magic_num_mut,` |
| `src/content/cards/silent/reflex.rs` | 11 | `upgrades` | `let upgraded = if card.upgrades > 0 { 1 } else { 0 };` |
| `src/content/cards/silent/storm_of_steel.rs` | 8 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/survivor.rs` | 10 | `base_block_mut` | `amount: card.base_block_mut,` |
| `src/content/cards/silent/tactician.rs` | 11 | `upgrades` | `let upgraded = if card.upgrades > 0 { 1 } else { 0 };` |

## Immediate Recommendations

1. The `疑似测试掩盖风险` group is empty; continue with the `应改为 evaluate_card_for_play` group.
2. Make ordinary play functions evaluate locally before reading damage/block/magic.
3. Keep the `合理特殊升级` group as direct upgrade reads, but add comments/tests where the Java source passes `this.upgraded` directly to an action.
4. Keep this script failing on unclassified files so future direct reads cannot silently enter Silent card code.
