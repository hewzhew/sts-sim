---
name: operational-lessons
description: Critical lessons learned, blind spots, and development patterns for the sts_sim project. Consult when debugging unexpected behavior, making architecture decisions, or starting new work sessions.
---

# Operational Lessons & Blind Spots

本文档记录开发过程中踩过的坑、关键的操作流程、以及容易遗忘的细节。
每次开始新的工作会话时应该快速浏览，避免重复犯同样的错误。

---

## 🔴 构建与部署

### Lesson #1: `cargo build` ≠ 安装到 Python 环境
- `cargo build --release` 只编译 Rust 库到 `target/release/`
- **不会** 更新 `.venv/Lib/site-packages/sts_sim/` 里的 `.pyd` 文件
- `play_ui.py`、`train_ppo.py` 等 Python 脚本加载的是虚拟环境里的旧 `.pyd`
- **必须运行** `maturin develop --release` 才能让 Python 看到新的 Rust 代码
- 症状：改了 Rust 代码、编译通过了，但 play_ui 行为没变化

### Lesson #2: 修改后的完整更新流程
```bash
# 1. 编译并安装到虚拟环境（一步到位）
maturin develop --release

# 2. 重启 Python 进程（Python 不会热加载 .pyd）
Stop-Process -Name python -Force
python scripts/play_ui.py
```

---

## 🟡 游戏机制 — 已发现的盲点

### Lesson #3: 战斗初始化必须调用 `initialize_combat_deck()`
- `master_deck` 是永久卡组，跨战斗存在
- `draw_pile` 是战斗内的抽牌堆，每场战斗开始时必须从 `master_deck` 复制过来
- 之前的 bug：`spawn_combat_enemies()` 只在 `draw_pile.is_empty() && hand.is_empty()` 时调用 `setup_starter_deck_self()`，但在第一场战斗后 master_deck 已有卡牌，所以后续战斗 draw_pile 始终为空
- **所有进入战斗的入口都必须调用 `initialize_combat_deck()`**

### Lesson #4: 回合生命周期钩子必须手动接入
- Java 的 `on_turn_end()` / `on_turn_start()` / `on_battle_start()` 是通过 Action Queue 自动触发的
- Rust 的同步架构需要**手动**在 `interop.rs` 的正确位置调用这些函数
- 已知需要手动调用的位置：
  - `execute_end_turn()` → `engine::on_turn_end()` (回合结束能力效果)
  - `execute_end_turn()` → `engine::on_turn_start()` (新回合开始能力效果)
  - `spawn_combat_enemies()` → `engine::on_battle_start()` + `on_turn_start()` (战斗开始)

### Lesson #5: Reward 系统观察
- 当前 Skip Rewards 按钮在拿完所有金币/卡牌/遗物后可能多余（待验证）
- **重要**：奖励获取**顺序**在特定场景下很关键：
  - 先拿卡牌，再拿瓶装遗物（Bottled Flame/Lightning/Tornado）→ 可以将新卡标记为 Innate
  - 先拿遗物再拿卡牌 → 瓶装效果只能选已有卡牌
  - 这种"顺序敏感"的奖励逻辑是 AI 训练需要学会的
- 当前实现可能没有考虑奖励获取顺序的差异

---

## 🟢 开发模式 — 来自 Vibe Coding 文章的启发

### Lesson #6: Agent 写代码时你该做什么
不是盯着代码看，而是"向上跳一层"思考：
1. **画数据流图** — 数据从入口到出口的路径，每步的失败模式
2. **想系统交互** — 新功能与现有功能的竞态条件、一致性问题
3. **预判架构瓶颈** — 推理延迟、缓存策略、预加载队列

### Lesson #7: 五层 Review 框架
从高到低，集中精力在 Agent 最弱的层次：

| 层次 | 内容 | Agent 能力 | 人的价值 |
|------|------|-----------|---------|
| L1 问题定义 | "该不该做这件事？" | ❌ 极弱 | 🔴 必须人来 |
| L2 架构设计 | 模块间关系、数据流、错误传播 | ⚠️ 局部优化 | 🔴 全局视角 |
| L3 关键决策把关 | 数据模型、安全、并发、依赖选择 | ⚠️ 常遗漏 | 🟡 重点审查 |
| L4 质量防线 | 边界条件、性能、代码味道 | ✅ 不错 | 🟢 抽查即可 |
| L5 知识萃取 | 从 Agent 代码中学习新模式 | N/A | 🟢 持续投资 |

### Lesson #8: 实际检查清单（Agent 生成代码后）
1. **先跑一遍** — 能跑不代表没问题，跑不通一定有问题
2. **看边界** — 空输入？超大输入？并发？网络断？数据库挂？
3. **看性能** — 循环里做 DB 查询？无分页全量加载？热路径序列化？
4. **凌晨三点测试** — "如果这段代码半夜出问题，日志能定位吗？"

### Lesson #9: 与本项目的具体对应
- **L1（问题定义）**= "AI 训练需要哪些机制准确？哪些可以简化？"
- **L2（架构设计）**= Java Action Queue vs Rust 同步执行的取舍
- **L3（关键决策）**= 哪些 power hook 必须实现 vs 可以跳过
- **L4（边界条件）**= 第二场战斗、空 draw pile、手牌上限 10 张
- **L5（知识萃取）**= 从 Java 源码中学习的 STS 设计模式

### Lesson #10: Windows 临时文件路径
- `/tmp/` 在 Windows 上不存在（映射到 `C:\tmp`）
- **临时脚本应写到 `C:\tmp\`**，不是 `/tmp/`
- `os.path.join` 和 `Path` 对象在 Windows 上自动处理反斜杠
- PowerShell 的 `python /tmp/script.py` → `C:\tmp\script.py`

### Lesson #11: 遗物计数器在 step verifier 中是系统性丢失
- Step verifier 每步重建状态（hydrate），遗物的计数器（counter）不在 CommunicationMod JSON 中
- **受影响遗物**（有内部 counter 的）:
  - **Shuriken**: 每 3 张攻击卡 +1 Strength → 导致 Δ=-1 enemy_hp
  - **Kunai**: 每 3 张攻击卡 +1 Dexterity
  - **Fan**: 每 3 张攻击卡 +4 Block
  - **Pen Nib**: 每 10 张攻击卡 ×2 damage
  - **Nunchaku**: 每 10 张攻击卡 +1 Energy
  - **Sundial**: 每 3 次 shuffle +2 金币
  - **Incense Burner**: 每 6 回合 +1 Intangible
- 这不是 bug —— 这是 step verifier 架构的固有限制。每步独立验证无法跟踪跨步计数器
- **影响**：有这些遗物时，Δ=-1/-2 偏差应视为"预期内的验证噪音"，不是引擎 bug

---

## ⚠️ 预测的未来盲点

1. **多场战斗的状态残留** — 上一场战斗的 powers/statuses 是否正确清理？
2. **遗物计数器重置** — Necronomicon、Sundial 等遗物的 counter 是否在每场战斗 / 每回合正确重置？
3. **RNG 流分离** — Java 用独立 RNG 流（cardRng, shuffleRng, potionRng），Rust 用单一 rng
4. **手牌上限** — Java 限制 10 张，Rust 是否在所有路径都检查？
5. **Reward 屏幕的状态机** -- 选完卡牌后是否正确移除已选选项？
6. **怪物亡语 (On-Death Triggers)** — 多个怪物有死亡时触发效果:
   - Spore Cloud (Fungi Beast): 释放孢子 → 玩家 +2 Vulnerable
   - Exploder: 死亡时爆炸 → AoE 伤害?
   - Darkling: 非全部死亡时复活
   - Awakened One: Phase 1 死亡 → Phase 2 复活 (HP重置=300, 获得新能力)
   - Transient: 并非传统亡语，但回合计时器到时消失
   - Writhing Mass: 随攻击类型改变意图（非亡语但需特殊处理）
7. **跨回合 Power 衰减** — 某些 Power 回合结束时 -1 或移除:
   - DuplicationPower: 回合结束-1（如果为0则移除）
   - Flex 系: Strength 回合结束扣除
   - Draw Reduction / No Draw: 回合结束移除

### Lesson #12: 参考源可信度层次
- **Java 反编译源 > slay-the-model > 独立推测** -- 但三者都不完美
- Java：完整但设计差（动画逻辑混杂），slay-the-model：也有 bug（README 列出）
- slay-the-model 参考价值在于**架构设计思路**，NOT 正确性
- Java 参考价值在于**机制完整性**（hook 触发顺序、数值公式）
- 最终裁判：三角验证 + 独立推理

### Lesson #13: Java 动画 Bug 策略
- Java 原版有 "逃课" 机制（动画结算中的优先级冲突）
- Rust 中 **不保留**这些 bug -- 保持纯逻辑确定性
- 发现时记录到 `known_animation_bugs.md`
- 未来可作为 AI "dirty tools"（`env.enable_exploits(True)`）

### Lesson #12: 保持编译输出干净
- 91 个 `gen_range` deprecated warning 淹没了有用的编译错误信息
- 及时处理 deprecation warning（如 `gen_range` → `random_range`），即使功能不受影响
- **干净的编译输出 = 更快发现新问题**
- 用 `cargo check 2>&1 | Select-String "error"` 过滤噪音

---

## 🔵 怪物数据架构 — 核心决策记录

### Lesson #13: 怪物 ID 命名规则
- **Rust canonical ID = 游戏显示名的 PascalCase**，不用 Java 内部 ID
- Java ID 命名极不规范：`Serpent`=Spire Growth, `Healer`=Mystic, `GremlinTsundere`=Shield Gremlin
- 19/68 个 Java ID 与显示名完全不同，不适合直接采用
- 规则：`"Jaw Worm"` → `JawWorm`，`"The Champ"` → `Champ`（去冠词），`"Acid Slime (L)"` → `AcidSlime_L`
- **完整映射表**在 `implementation_plan.md` 和 `tests/id_comparison.txt`
- 同时保留 `java_id` 字段方便搜索 Java 源码

### Lesson #14: JSON 数据可信度 — 81% 规则
- 自动审计结果：63 个 Java 怪物中 **51 个 (81%)** 的 JSON effects 与 Java 一致
- **9 个 (14%) 有实质性缺漏**：Chosen 缺 Strength、Deca 缺 PlatedArmor 等
- Rust `MoveEffect` 只反序列化 6/14 个 JSON 字段，其余被 serde **静默丢弃**
- JSON 有 22 种 effect type，其中 15 种 Rust 完全不识别
- `ascension_scaling` 完全没接入 Rust 反序列化
- **审计工具**：`tests/audit_json_vs_java.py`，随时可重跑

### Lesson #15: Intent 中间层是一个反模式
- `resolve_intent()` 把多效果的 `effects[]` 压缩成单一 `Intent` 枚举变体（如 `Buff` 或 `Defend`）
- Java `takeTurn()` 通常执行多个 Action（GainBlock + ApplyPower + DamageAction）
- Intent 枚举无法表达这种组合 → 丢失 block/buff/debuff 分量
- **正确做法**：执行层直接翻译 Java `takeTurn()`，不经过 Intent 中转
- Intent 降级为 `DisplayIntent`，仅供 AI 编码和 UI 显示

### Lesson #16: Hydrate 状态缺失 — `just_applied` 问题
- 从 JSON 快照 hydrate GameState 时，`just_applied` 标志集合为空
- Java 的 `justApplied` 标志控制 debuff 首轮不衰减
- 如果 before-state 中的 Vulnerable 是本轮刚施加的，hydrate 后 `on_round_end()` 会错误地衰减它
- 表现：`player_powers.Weakened expected 1 got 0`（衰减太快一轮）
- 修复方向：从 JSONL 命令序列推断 `just_applied`，或全路径回放

### Lesson #17: 怪物数据架构最终方案
- **JSON 只存静态数值**（HP/名称/招式数值/ascension 数值），不存行为逻辑
- **行为选择**（getMove）：`hardcoded_ai.rs`（已覆盖 57/66 个怪物）
- **效果执行**（takeTurn）：新增 `hardcoded_take_turn()` 对照 Java
- **最终目标**：全 Rust。JSON 逐步降级为归档参考文件
- 卡牌/药水/遗物 JSON 同理：名字不对应是老问题，需要统一 ID 体系

### Lesson #18: bottled_ai_fresh 有自己的 .pyd 副本
- `bottled_ai_fresh/sts_sim.pyd` 是 Python 实际加载的模块
- `cargo build --release` 和 `maturin develop` 都不会更新这个文件
- **必须手动复制**: `Copy-Item target\release\sts_sim.dll bottled_ai_fresh\sts_sim.pyd`
- 这个问题浪费了 30+ 分钟调试时间（2026-03-05）

### Lesson #19: 不要写终端 Python one-liner
- PowerShell 引号转义与 Python f-string 冲突，语法错误率 > 50%
- 用 `sim_diag.py` 子命令代替: `summary`, `report`, `inspect`, `context`
- 详见 `/diagnose-divergence` 工作流和 `divergence-diagnosis` skill

