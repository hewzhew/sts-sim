# Frozen Case Panel V0a Review Draft

Status: historical review draft. Frozen Panel V0a remains supported; the manual key-card and
root-action probes described as interventions were retired on 2026-07-12.

Purpose: 给外援评审第一把实验尺子。上一版同时描述了 Frozen Case Panel 和 Fresh Run Panel，作为路线图可以，但第一版实现仍偏大。本版收窄为：

```text
Frozen Case Panel V0a only
```

Fresh Run Panel 暂时只作为后续方向，不进入 V0a 实现范围。

## 1. 为什么先只做 Frozen Case

当前最紧迫的问题是 combat search 实验，而不是整局策略体检。

最近的 Awakened One case 暴露了典型问题：

```text
Demon Form 被识别为 key setup card。
Root action role duel 显示 Demon Form first 能打穿 Awakened One 一阶段。
但最终状态是 player_hp=0, AwakenedOne half_dead=true, GameOver(Defeat)。
```

这说明我们需要比较：

```text
同一个精确 combat state 下，不同 search config 的表现。
```

如果第一刀同时做 Fresh Run Panel，会把 reward、shop、event、route、combat search 混在一起。那样结果变化后，很难判断到底是谁导致的。

所以 V0a 只回答一个问题：

```text
在固定 combat case 上，新 search lane 是否比旧 search lane 更好，或者至少没有明显退化？
```

它不回答：

```text
当前整局策略会不会再次生成这个卡组？
reward/shop/event 策略是否更好？
这条 run 为什么输？
```

## 2. 三个概念必须分开

### 2.1 Lane

Lane 是正常搜索配置，可以未来被部署到 runner portfolio。

例子：

```text
baseline
key_setup_bias
baseline_with_potions
```

Lane 的特点：

```text
不强制指定第一步
不篡改手牌/牌堆/HP
不依赖某张具体牌名
可以在同类 case 上重复运行
```

V0a 里的 lane 名称必须绑定到显式 search config。

```text
baseline 不是“当前默认搜索”。
baseline 必须是一个命名配置，并在输出里记录 config summary。
```

### 2.2 Probe

Probe 是诊断工具，只观察或解释。

例子：

```text
key_card_decision_microscope
combat_case_review ladder
key_card_lifecycle
```

Probe 可以帮助人理解，但它本身不是 deployable search lane。

### 2.3 Intervention

Intervention 是强制反事实实验。

例子：

```text
move Demon Form to opening hand
force Demon Form first
force Whirlwind first
counterfactual HP
```

必须写死这条边界：

```text
Forced-root action duel is an intervention, not a deployable lane.
```

如果 intervention 跑赢了，只说明“这个 root child / 这个反事实值得研究”，不说明 runner 应该以后照做。

## 3. Frozen Case Panel V0a 范围

### 3.1 Case 数量

第一版只用 3 个 frozen cases。

候选：

```text
Awakened One low-HP setup / phase-transition case
Collector Act 2 boss pressure case
Gremlin Leader or multi-enemy pressure case
```

选择标准：

```text
能稳定复现
覆盖一个明确搜索难点
case 文件已经存在或可以低成本保存
不要求代表最新 reward/shop 策略
```

### 3.2 Lane 数量

第一版只比较 2 个 lane：

```text
baseline
key_setup_bias
```

不把 `root_action_role_duel` 放进常规 lane。它保留为手动 intervention。

V0a 对两个 lane 的边界：

```text
baseline:
  显式命名的 search config。
  不能等同于“当前代码默认值”。
  panel 输出必须记录它的 config summary。

key_setup_bias:
  以 baseline 为基准，只改变 action ordering。
  不改变合法动作。
  不新增剪枝。
  不强制 root action。
  不写 Awakened One / Demon Form 这类 boss/card 专用规则。
```

### 3.3 输出文件

V0a 只输出两个文件：

```text
panel_rows.jsonl
panel_table.md
```

不做：

```text
HTML report
长 human summary
自动解释
自动建议下一步修什么
自动调参
```

第一刀实现范围也固定为：

```text
3 frozen cases × 2 lanes
panel_rows.jsonl
panel_table.md
```

不做自动发现 case，不做 Fresh Run，不接 runner。

## 4. 必须从 V0a 就保存 provenance

Frozen case 如果没有来源，很快会变成幽灵样本。

V0a 每行最少保存：

```text
case_id
case_path
case_origin_seed
captured_at_commit
captured_policy_notes
reviewed_at_commit
lane
search_config_summary
command
```

说明：

```text
case_origin_seed: 生成这个 case 的原 seed；没有就填 unknown。
captured_at_commit: 保存 case 时的 commit；没有就填 unknown，但字段必须存在。
captured_policy_notes: 简短说明当时策略背景，可以先手填。
reviewed_at_commit: 本次 panel 运行 commit。
search_config_summary: 稳定 JSON/短字符串；必须足够区分 baseline 这类命名 config。
command: 实际运行命令，方便复现。
```

## 5. Panel row 字段

建议 V0a 每个 case/lane 一行：

```text
case_id
case_path
case_origin_seed
captured_at_commit
reviewed_at_commit
lane
search_config_summary
complete_win
outcome_tier
final_hp
turns
potions_used
first_action_key
first_action_role
key_card_played
key_card_first_play_step
living_enemy_count
total_enemy_hp
half_dead_enemy_count
phase_pending_enemy_player_died
nodes_expanded
elapsed_ms
deadline_hit
tool_status
```

字段说明：

```text
complete_win:
  真实完整胜利。

outcome_tier:
  粗分层，不是自然语言解释。

phase_pending_enemy_player_died:
  用于 Awakened One 这类 half_dead boss。

tool_status:
  ok / malformed_case / tool_failure / missing_case
```

## 6. outcome_tier V0a

失败不能只看 `total_enemy_hp`。Awakened One 的 `half_dead=true, player_hp=0` 已经证明 raw enemy HP 会误导。

V0a tier：

```text
complete_win
phase_complete_but_player_died
survived_to_deadline
died_after_progress
died_early
incomplete_or_unknown
malformed_or_tool_failure
```

暂定规则：

```text
complete_win:
  complete_win == true

phase_complete_but_player_died:
  terminal loss
  final_hp <= 0
  half_dead_enemy_count > 0

survived_to_deadline:
  no complete win
  deadline_hit == true
  final_hp > 0 if available

died_after_progress:
  terminal loss
  final_hp <= 0
  half_dead_enemy_count == 0
  best-effort meaningful progress exists
  V0a 只用保守证据，比如 turns >= 2 或 total_enemy_hp 相比初始下降

died_early:
  terminal loss
  little enemy progress or very short turns

incomplete_or_unknown:
  tool_status == ok
  但没有 complete win
  也没有明确 terminal loss / deadline / malformed 证据
  或关键状态字段不足以可靠分类

malformed_or_tool_failure:
  case missing / parse failed / review command failed
```

V0a 不把 `died_after_progress` / `died_early` 当作强归因。

```text
它们只是避免所有 non-win 混成一团的弱分层。
第一版不要为了 "meaningful progress" 写复杂规则。
```

Open question for reviewer:

```text
是否应该把 "phase_complete_but_player_died" 只用于 Awakened One，
还是泛化到所有 half_dead / multi-phase enemy？
```

## 7. Loss 比较原则

Panel V0a 不输出一个全局 score。

只允许做弱比较：

```text
complete_win 优先于 non-win。
phase_complete_but_player_died 不能被当作普通 enemy_hp=0 接近胜利。
deadline result 不能和 terminal loss 简单混排。
同一 outcome_tier 内才比较 final_hp / turns / enemy_hp。
```

禁止：

```text
某 lane 在单个 case 上 enemy_hp 更低 -> 自动认为更好。
某 intervention 赢了 -> 自动变成 runner lane。
```

## 8. Review-only 进入 runner 的门槛

一个 review-only lane / experimental lane 想进入 `branch_tiny` portfolio，至少需要：

```text
1. Frozen target cases 有收益。
2. Non-target frozen cases 没明显退化。
3. Fresh run smoke seeds 没明显更差。
4. 能解释收益来自什么事实，而不是只看单个胜负。
5. 先作为 opt-in experimental portfolio lane，不作为默认策略。
```

Intervention 不能直接进入 runner。它只能启发一个正常 lane 的设计。

## 9. Fresh Run Panel 暂不实现

Fresh Run Panel 仍然是后续必要方向，但不属于 V0a。

后续它会回答：

```text
当前 reward/shop/event/search 组合，从 Neow 开始会推进到哪里？
owner gap / combat gap / budget gap 谁在主导？
整局是否更稳定？
```

但在 Frozen Case Panel 稳定前，不实现 Fresh Run Panel。

## 10. V0a 预期工作流

实现前先写一个很短的 implementation note，列出：

```text
选中的 3 个 frozen cases
baseline 的 config summary
key_setup_bias 相对 baseline 的唯一差异
输出目录和命令形态
```

这份 note 只服务第一刀实现，不扩展 Fresh Run / HTML / 自动解释。

搜索改动流程：

```text
1. 修改或新增一个 search lane。
2. 跑 Frozen Case Panel V0a。
3. 看 panel_rows.jsonl 和 panel_table.md。
4. 如果 target case 好但 non-target case 明显坏，不接 runner。
5. 如果整体稳定，再考虑 Fresh Run smoke。
```

单 case 深挖流程：

```text
1. Panel 发现某 case/lane 差异值得看。
2. 手动跑 combat_case_review。
3. 必要时跑 key_card_decision_microscope / root_action_role_duel。
4. Intervention 只用于解释，不直接变 lane。
```

## 11. 给外援的审查问题

请重点审查：

1. 第一版只做 Frozen Case Panel V0a 是否足够收窄？
2. Lane / Probe / Intervention 三分法是否清楚？
3. `root_action_role_duel` 降级为 intervention 是否合理？
4. provenance 字段是否足够，是否还有必须从 V0a 保存的来源信息？
5. `outcome_tier` 是否过粗或过细？
6. `phase_complete_but_player_died` 的定义是否适合 Awakened One / 多阶段 boss？
7. `panel_rows.jsonl` + `panel_table.md` 是否足够作为第一版输出？
8. review-only 进入 runner 的门槛是否太严或不够严？

## 12. 暂定结论

V0a 的目标不是让搜索变强，而是建立最小实验尺子：

```text
3 个 case
2 个 lane
一张薄表
明确 provenance
明确 outcome_tier
```

这把尺子稳定后，再讨论 Fresh Run Panel 和 runner portfolio。
