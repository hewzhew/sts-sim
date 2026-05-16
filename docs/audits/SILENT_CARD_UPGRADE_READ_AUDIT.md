# Silent Card Upgrade Read Audit

Generated: `2026-05-16 10:38:13`

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

- Files with direct reads: `6`
- Direct read occurrences: `6`
- Unclassified files: `0`

## Category Definitions

- **应改为 evaluate_card_for_play**: ordinary `use` logic reads mutable rendered fields directly. The card should evaluate itself at play time or use a shared definition/upgrades helper.
- **合理特殊升级**: direct `card.upgrades` is part of Java action semantics or happens outside normal card play, so `evaluate_card_for_play` is not the right abstraction.
- **疑似测试掩盖风险**: current tests appear to prefill mutable render fields, which can hide the fact that normal play does not evaluate the card.

## 应改为 evaluate_card_for_play

No files.

## 合理特殊升级

| File | Fields | Lines | Reason | Recommendation |
| --- | --- | --- | --- | --- |
| `src/content/cards/silent/adrenaline.rs` | `upgrades` | `src/content/cards/silent/adrenaline.rs:33` | Java Adrenaline.use 根据 this.upgraded 决定 GainEnergyAction(2/1)；抽牌数已由 play 路径本地 evaluate，不再依赖 mutable magic 预填。 | 保留 direct upgraded 分支，并用测试覆盖 Adrenaline+ 在未预填 base_magic_num_mut 时仍获得 2 能量、抽 2 张。 |
| `src/content/cards/silent/doppelganger.rs` | `upgrades` | `src/content/cards/silent/doppelganger.rs:31` | Java DoppelgangerAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是动作语义参数，不是数值评价。 | 保留直接传 upgraded，但建议测试覆盖 X 费、free_to_play_once、energy_on_use 与升级组合。 |
| `src/content/cards/silent/malaise.rs` | `upgrades` | `src/content/cards/silent/malaise.rs:38` | Java MalaiseAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是 X 费动作语义参数。 | 保留直接传 upgraded，但建议测试覆盖升级和 energy_on_use 的组合。 |
| `src/content/cards/silent/reflex.rs` | `upgrades` | `src/content/cards/silent/reflex.rs:34` | triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 magic，避免依赖未评价的 mutable magic。 | 保留这种模式，后续可抽成 manual-discard magic helper，并继续保留与弃牌顺序相关测试。 |
| `src/content/cards/silent/storm_of_steel.rs` | `upgrades` | `src/content/cards/silent/storm_of_steel.rs:31` | Java BladeFuryAction 直接接收 this.upgraded；读取 card.upgrades 是生成 Shiv 是否升级的语义参数。 | 保留直接传 upgraded，确保 BladeFuryAction/生成 Shiv 的升级行为有测试覆盖。 |
| `src/content/cards/silent/tactician.rs` | `upgrades` | `src/content/cards/silent/tactician.rs:34` | triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 energy amount，匹配 Java addToTop(new GainEnergyAction(this.magicNumber))。 | 保留这种模式，后续可抽成 manual-discard magic helper，并继续保留 addToTop 顺序测试。 |

## 疑似测试掩盖风险

No files.

## Raw Matches

| File | Line | Field | Code |
| --- | --- | --- | --- |
| `src/content/cards/silent/adrenaline.rs` | 33 | `upgrades` | `amount: if card.upgrades > 0 { 2 } else { 1 },` |
| `src/content/cards/silent/doppelganger.rs` | 31 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/malaise.rs` | 38 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/reflex.rs` | 34 | `upgrades` | `let upgraded = if card.upgrades > 0 { 1 } else { 0 };` |
| `src/content/cards/silent/storm_of_steel.rs` | 31 | `upgrades` | `upgraded: card.upgrades > 0,` |
| `src/content/cards/silent/tactician.rs` | 34 | `upgrades` | `let upgraded = if card.upgrades > 0 { 1 } else { 0 };` |

## Immediate Recommendations

1. The `疑似测试掩盖风险` and `应改为 evaluate_card_for_play` groups are empty; ordinary Silent play paths no longer read transient mutable card fields directly.
2. Keep remaining direct `card.upgrades` reads limited to the `合理特殊升级` group.
3. Keep the `合理特殊升级` group as direct upgrade reads, but add comments/tests where the Java source passes `this.upgraded` directly to an action.
4. Keep this script failing on unclassified files so future direct reads cannot silently enter Silent card code.
