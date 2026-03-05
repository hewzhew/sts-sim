# 杀戮尖塔 RL 模拟器 — 项目状态

> **最后更新**: 2026-02-21 | **Rust LoC**: ~26K | **Tests**: 337 pass | **Data files**: 79

## 📊 模块覆盖

| 模块 | 状态 | 文件位置 |
|------|------|---------|
| 核心数据层 | ✅ 100% | `core/schema.rs`, `core/state.rs`, `core/loader.rs` |
| 引擎 (拆分为子模块) | ✅ | `engine/` (commands, combat, navigation, events, potions_use) |
| Power Hook 系统 | ✅ ~75/123 powers | `powers_mod/hooks.rs`, `powers_mod/power_set.rs` |
| 遗物系统 | ✅ | `items/relics.rs` — trigger system + 常用遗物 |
| 药水系统 | ✅ | `items/potions.rs` — PotionLibrary + 效果 |
| 敌人 AI | ✅ | `monsters/enemy.rs` — 行为模式 + 意图系统 |
| 遭遇池 | ✅ | `monsters/encounters.rs`, `monsters/act_config.rs` |
| 地图生成 | ✅ | `dungeon_mod/map.rs` — 程序化生成 + SimpleMap |
| 奖励系统 | ✅ | `dungeon_mod/rewards.rs` |
| 事件系统 | ✅ | `rooms/events.rs` — JSON驱动 |
| 商店 / 营火 | ✅ | `rooms/shop.rs`, `rooms/campfire.rs` |
| 卡牌数据 | ✅ 370张 | `data/cards/` (JSON) |
| Python 绑定 | ✅ | `interop.rs` — PyO3 Gym环境 + BatchSimulator |
| RL (编码/MCTS) | ✅ 骨架 | `ai/encoding.rs`, `ai/features.rs`, `ai/mcts.rs` |

## 🏗️ 代码架构

```
src/ (45 files, ~26K LoC)
├── core/           — schema, state, loader
├── engine/         — commands, combat, navigation, events, potions_use
├── powers_mod/     — PowerId enum dispatch + PowerSet
├── monsters/       — enemy AI, encounter pools, act config
├── rooms/          — shop, campfire, events
├── dungeon_mod/    — map generation, rewards
├── items/          — relics, potions
├── ai/             — encoding, features, card_features, mcts
├── interop.rs      — PyO3 bindings
├── lib.rs          — module declarations + pub use aliases
└── main.rs         — demo binary
```

详细结构参见 `.agent/workflows/project-structure.md`

## ❌ 尚未实现

- **Orb 系统** (Defect 角色) — ChannelOrb/EvokeOrb 命令有 stub
- **Stance 系统** (Watcher 角色) — EnterStance/ExitStance 命令有 stub
- **X 费用卡** — 动态费用系统
- **部分卡牌效果** — ~20个 CardCommand 仍为 TODO/stub

## 🚀 运行

```powershell
cargo test --lib          # 337 tests
cargo build --release     # lib + bins
maturin develop --release # Python module
```
