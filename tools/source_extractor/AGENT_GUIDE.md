# Agent Guide: StS Rust 复刻参考手册

## 这些文件是什么

这些产物是从杀戮尖塔（Slay the Spire）反编译 Java 源码中**程序化提取**的结构化信息。
所有内容都由 AST 分析器直接从源码提取，不含任何人工总结或 LLM 生成内容。
优先使用结构化 JSON 事实层；Markdown 主要用于人工阅读、索引和人工核对。
不要依赖记忆或猜测。

## 当前真相层

- 结构化真相优先：
  - `output/hooks.json`
  - `output/scattered_logic.json`
  - `output/relics.json`
- Markdown 是渲染层：
  - `output/hooks.md`
  - `output/scattered_logic.md`
  - `output/relics.md`

## 文件清单

| 文件 | 内容 | 什么时候查 |
|---|---|---|
| `summary.md` | 类数量统计、hook 方法使用频率 | 开始任何新任务前先看一眼 |
| `inheritance.md` | 所有类的继承关系，按类别分组 | 需要知道某个类属于什么类别时 |
| `hooks.json` | 每个 power/relic/card 覆盖了哪些 hook 的结构化事实 | 实现任何 power/relic/card 前优先查 |
| `hooks.md` | `hooks.json` 的阅读版索引 | 需要人工浏览时 |
| `actions.md` | 每个 Action 的 update() 逻辑、子 Action 创建链 | 实现 Action 时 |
| `cards.md` | 每张卡的 use() 方法、Action 入队顺序 | 实现卡牌时 |
| `powers.md` | 每个 Power 的 hook 方法实现体 | 实现 Power 时 |
| `relics.json` | Relic hook / call / queue insertion 的结构化事实 | 实现 Relic / 检查 addToTop/addToBot 时优先查 |
| `relics.md` | `relics.json` 的阅读版索引 | 需要人工浏览完整方法体时 |
| `scattered_logic.json` | **关键！** 遗物/能力逻辑散落在引擎中的结构化索引 | 实现任何 relic/power 时优先查 |
| `scattered_logic.md` | `scattered_logic.json` 的阅读版索引 | 需要人工浏览时 |
| `completeness_checklist.md` | 实现进度追踪表（自身逻辑 + 引擎侧逻辑） | 每次开始新组件前检查 |
| `taint_report.md` | 逐行标注：哪些是纯逻辑、纯表现层、还是混合的 | 实现 Action 时**必看**，跳过前验证 |
| `call_graph.md` | 核心引擎类的方法调用序列 | 实现引擎调度逻辑时**必看** |
| `damage_pipeline.md` | 伤害计算链完整代码 | 涉及任何伤害计算时**必看** |

## 实现工作流程

### 规则一：先查后写

**在写任何一行 Rust 代码之前，必须先查对应的参考文件。**

不要凭记忆实现任何效果。即使你"知道"某个 power 做什么，也必须查 `powers.md` 确认 Java 原版的精确行为。

### 规则二：自底向上

实现顺序严格遵循依赖链：

```
1. ActionManager 调度器（查 call_graph.md → GameActionManager / ActionManager）
2. 原子 Actions（查 actions.md，逐个实现）
3. Hook 调度框架（查 call_graph.md → AbstractRoom / AbstractPlayer 的方法调用顺序）
4. Powers（查 powers.md + hooks.md）
5. Relics（查 relics.md + hooks.md）
6. Cards（查 cards.md）
```

### 规则三：交叉验证（含散落逻辑检查）

实现一个组件时，至少查两个文件，**并且必须查 `scattered_logic.json`**：

- **实现一个 Power**:
  1. 查 `hooks.json` → 确认它覆盖了哪些 hook
  2. 查 `powers.md` → 看每个 hook 方法的具体实现
  3. **查 `scattered_logic.json` → 看引擎中是否有 `hasPower("XxxPower")` 的检查**
  4. 如果有引擎侧检查 → 这些检查点的逻辑也必须在 Rust 中实现
  5. 如果涉及伤害 → 额外查 `damage_pipeline.md`

- **实现一张 Card**:
  1. 查 `cards.md` → 看 use() 里塞了哪些 Action、顺序是什么
  2. 对每个 Action → 查 `actions.md` 确认已实现且逻辑正确
  3. 如果 Action 未实现 → **先停下来去实现 Action**

- **实现一个 Relic**:
  1. 查 `hooks.json` → 确认它覆盖了哪些 hook
  2. 查 `relics.json` → 看每个 hook 的结构化 facts、calls、queue insertion
  3. 如需完整方法体，再查 `relics.md`
  4. **查 `scattered_logic.json` → 看引擎中是否有 `hasRelic("XxxRelic")` 的检查**
  5. **如果标注为 "NO hooks in own class — logic is ENTIRELY engine-side"，则遗物类本身可能只需要一个空壳，但引擎中的所有检查点必须实现**
  6. 查 `call_graph.md` → 确认 hook 被调用的时机和顺序

- **涉及伤害计算**:
  1. 查 `damage_pipeline.md` → 这是**唯一可信来源**
  2. 注意 modifier 的遍历顺序（先 power 还是先 relic）
  3. 注意取整方式（floor / round / ceil）

### 规则四：用 completeness_checklist.md 追踪进度

每实现完一个 relic 或 power，在 checklist 中标记两列：
- **Self Done**: 自身类逻辑已实现
- **Engine Done**: 引擎侧散落逻辑已实现

**两列都打勾才算实现完成。** 只完成自身类逻辑是最常见的遗漏来源。

### 规则五：不确定就 grep 原始 Java

如果提取信息不够详细，直接去 Java 源码目录用 grep 查。比如：

```bash
# 查某个 hook 在引擎中被调用的所有位置
grep -rn "\.atStartOfTurn(" /path/to/java/src/

# 查某个 Action 被谁创建
grep -rn "new DamageAction(" /path/to/java/src/

# 查某个字段在哪里被修改
grep -rn "\.block\s*=" /path/to/java/src/
```

## 关键注意事项

### ⚠️ 散落逻辑（最重要）

StS 中大量遗物和能力的真实逻辑**不在它们自己的 Java 类里**，而是在引擎代码中用
`if (player.hasRelic("XxxRelic"))` 硬编码实现的。

典型例子：
- 某些遗物类里只有 `flash()` 调用（控制图标闪烁），真正的游戏效果在引擎某处的 if 判断里
- 某些能力的部分效果通过 hook 方法实现，另一部分效果在伤害计算管线里用 `hasPower()` 检查

**`scattered_logic.json` / `scattered_logic.md` 列出了所有这种情况。** 如果一个遗物被标注为 "NO hooks in own class — logic is ENTIRELY engine-side"，
意味着翻译遗物类本身基本没用，真正的实现工作在引擎侧。

这是导致"以为实现了其实没实现"和"在引擎不同位置写重复逻辑"的根源。
每实现一个 relic/power，**必须同时检查 `scattered_logic.json`**。

### Action 队列语义

**注意：大部分 Action 的 `update()` 方法是表现层和逻辑混合的重灾区。**

查 `taint_report.md` 可以看到每个方法的逐行标注：
- `[LOGIC]` — 纯游戏逻辑，必须在 Rust 中实现
- `[PRES ]` — 纯表现层（动画/音效/UI），无头模拟器跳过
- `[MIXED]` — ⚠️ 同一行里既有逻辑又有表现，需要人工判断哪部分该保留

**处理 MIXED 方法的原则：**

1. 不要整个跳过。MIXED 方法里一定有需要的逻辑。
2. 不要整个保留。里面的动画/等待代码会干扰无头模拟器。
3. 逐行看标注。`[LOGIC]` 行抄过来，`[PRES]` 行扔掉，`[MIXED]` 行手动拆。
4. 特别注意 `duration` 模式：`if (this.duration < 0.5F)` 这种分支里通常藏着真正的执行逻辑，
   `duration` 本身是动画计时但分支条件控制了逻辑触发。无头模拟器应该立即执行这些分支。

StS 的 Action 队列有两种入队方式，语义完全不同：

StS 的 Action 队列有两种入队方式，语义完全不同：

- `addToBot(action)`: 加到队列尾部（大部分卡牌用这个）
- `addToTop(action)`: 加到队列头部（优先执行，常用于 power/relic 的触发效果）

在 `cards.md` 中，注意区分 `addToBot` 和 `addToTop` 的调用。顺序错误是最常见的结算 bug 来源。

### Hook 调用顺序

Hook 的遍历顺序取决于 relic/power 在列表中的位置：

- Relic: 按获取顺序遍历
- Power: 按施加顺序遍历

在 `call_graph.md` 的 AbstractPlayer/AbstractRoom 部分可以看到具体的 for 循环。

### 伤害计算链

伤害修正的应用顺序是：
1. base damage
2. atDamageGive (power modifiers, 攻击方)
3. atDamageReceive (power modifiers, 防御方)
4. atDamageFinalGive
5. atDamageFinalReceive

每一步的取整行为不同，**必须查 `damage_pipeline.md` 确认**，不要猜。

### 需要额外注意的 Java 怪癖

- StS 的很多 `int` 强转 `float` 再强转回 `int`，会丢精度。Rust 实现必须复现这个行为。
- 有些 Action 在 `isDone = true` 之前会跑多帧（`this.duration -= Gdx.graphics.getDeltaTime()`），
  无头模拟器应该立即执行，但要注意有些逻辑依赖 duration > 0 的中间状态。
- `AbstractDungeon.cardRandomRng`、`AbstractDungeon.miscRng` 等多个 RNG 实例，
  如果你需要种子级精确，必须确认每个操作用的是哪个 RNG。

## 当你不确定时

1. **永远不要猜。** 查文件，查不到就 grep 原始 Java。
2. 如果遇到 Java 中的隐式行为（比如父类构造函数的副作用），说明你遇到了提取器没覆盖的边界情况。
   此时应暂停实现，告知用户需要手动检查 Java 源码的哪个类的哪个方法。
3. 如果某个 Action/Power/Relic 在提取文件中找不到，可能是因为它的类名和效果名不一致。
   用 grep 在 Java 源码中搜关键词。
