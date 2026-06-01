# sts_simulator

[English README](README.md)

`sts_simulator` 是一个非官方的 Slay the Spire Rust 模拟器和 AI 搜索实验仓库。

当前主线是：

```text
模拟器状态 -> 合法动作 -> rollout/search -> value -> policy improvement
```

这个仓库现在不以 live 游戏 UI、prompt 工程、LLM 接管控制为主线。那些以后可以作为适配层存在，但当前重点是模拟器正确性、稳定状态边界、战斗搜索、以及整场战斗 outcome 评估。

## 当前状态

这是一个进行中的项目。当前维护的工作流是：

1. 从 Neow 开始运行确定性的模拟器 session
2. 由人类或受保护的自动辅助做战斗外决策
3. 保存稳定的 combat start
4. 用 Combat Search V2 搜索整场战斗轨迹
5. 把搜索 outcome 和整场战斗 baseline outcome 比较

搜索报告只是预算内证据。`Unresolved`、`BudgetExhausted` 或预算内找到胜利都不等于证明最优。

## 这个项目是什么

- Slay the Spire run/combat 状态转移的 Rust 复刻
- 合法动作生成和 apply-action 执行
- 用于手动和半自动跑局的终端 driver
- 精确 combat capture 和 benchmark artifact 基础设施
- 基于完整战斗轨迹的 Combat Search V2 实验
- 用于地图选择的 route-planner evidence

## 这个项目不是什么

- 不是 Mega Crit 官方项目
- 不是打磨完成的游戏客户端
- 现在不是 live CommunicationMod 替代品
- 不是 LLM teacher label 生成器
- 还不是稳定公共 API
- 不声称已经能最优游玩 Slay the Spire

## 快速开始

```powershell
cd D:\rust\sts_simulator
cargo test --quiet
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

开发时为了编译更快，可以用 debug build：

```powershell
cargo run --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

## 主要入口

| Binary | 用途 |
| --- | --- |
| `run_play_driver` | 手动或半自动模拟器跑局、保存 combat capture、保存整场战斗 baseline |
| `combat_search_v2_driver` | 从 start spec、combat capture 或 benchmark suite 跑整场战斗搜索 |
| `artifact_doctor` | 对 benchmark artifact 目录做只读审计 |

当前 binary 表面见 [src/bin/README.md](src/bin/README.md)。

## 手动跑局工作流

启动模拟器 session：

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

可选：写出诊断 trace：

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad --trace tools\artifacts\traces\seed521.trace.json
```

可选：自动保存每场战斗的起点：

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad --auto-capture-combat
```

常用 session 命令：

| 命令 | 含义 |
| --- | --- |
| `n` / `next` | 受保护地推进到下一个需要人类战略判断的边界 |
| `nr` | 和 `next` 类似，但允许 route planner 选择地图节点 |
| `rs` | 只显示 route 建议，不执行 |
| `rg` | route planner 选择并执行一个地图动作 |
| `sc` / `search-combat` | 从当前战斗边界运行 Combat Search V2 |
| `cap <case_id>` | 保存当前稳定 combat start |
| `baseline` | 在对应战斗结束后保存整场战斗 baseline |
| `deck`, `map`, `relics`, `potions` | 查看当前可见 run 面板 |
| `draw`, `discard`, `exhaust` | 查看战斗牌堆 |
| `details`, `raw` | 查看调试和内部字段 |
| `help` | 显示完整命令列表 |

奖励界面和地图预览会保留未领取奖励，直到真正选择下一层路径。从 reward 打开地图只是预览；用 `back` 或 `cancel` 返回，用 `go <x>` 或 `rg` 才会真正进入下一个房间。

## 战斗搜索工作流

从精确 combat capture 跑搜索：

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json
```

从 benchmark suite 跑搜索：

```powershell
cargo run --release --bin combat_search_v2_driver -- --benchmark-spec tools\artifacts\benchmarks\seed521_act1\benchmark.json
```

遇到困难战斗时显式提高预算：

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json --max-nodes 500000 --wall-ms 30000
```

药水分支默认不开；需要时显式打开：

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json --potion-policy semantic --max-potions-used 1
```

重要搜索输出概念：

- `Win` / `Loss` / `Unresolved` 表示报告的终局类别。
- `BudgetExhausted` 或 `DeadlineHit` 表示仍有未解决 frontier。
- `complete_trajectory_found=false` 表示在当前预算内没有找到可执行的完整胜利轨迹。
- 预算内找到完整胜利是有用证据，但不是最优性证明。

## Artifacts

默认 artifact 放在 `tools/artifacts/` 下，并被 git 忽略。

常见 artifact：

| Artifact | 作用 |
| --- | --- |
| `CombatCaptureV1` | 稳定战斗决策边界，用作搜索输入 |
| `CombatBaselineOutcomeV1` | 与 capture 匹配的整场战斗 baseline outcome |
| `BenchmarkSuiteV1` | 注册 capture 和可选 baseline 的 suite manifest |
| `SessionTraceV1` | 成功状态变更命令的诊断事实日志 |
| `SearchBenchmarkResultV1` | 一个或多个 case 上的搜索结果证据 |

Artifact 是 provenance 和评估证据，不是 teacher label。没有对应 benchmark context 和模拟器版本时，不能把它当作策略质量证明。

只读 artifact 审计：

```powershell
cargo run --release --bin artifact_doctor -- --root tools\artifacts\benchmarks --json
```

## 架构

| 目录 | 角色 |
| --- | --- |
| `src/content` | Java 游戏内容复刻；避免随意改动 |
| `src/state` | run、combat、map、event、reward、engine state |
| `src/engine` | 状态转移和 action handler |
| `src/runtime` | run/combat 执行时支持 |
| `src/sim` | 面向模拟器的 legal action、apply、search 边界 |
| `src/ai` | combat search、state key、route planner、value/rollout |
| `src/eval` | run-control、benchmark artifact、diagnostics、report |
| `src/bin` | 当前维护的命令入口 |
| `docs` | 当前说明、历史 audit、设计记录 |

仓库里仍可能有兼容旧路径的模块；新代码优先使用上表中的当前 ownership。

## 当前路线图

1. 保持稳定模拟器边界正确
2. 改进 Combat Search V2 的 value 和 rollout 行为
3. 处理特殊战斗阶段和高 fanout 分支，避免不可靠剪枝
4. 让 route planner 能服务低风险地图自动化
5. 加强 capture -> suite -> search -> baseline 比较闭环
6. 只有当 simulator/search evidence 层足够可靠后，再考虑 live-game adapter 或 LLM integration

## 验证命令

核心改动推送前建议运行：

```powershell
cargo fmt --check
cargo check --all-targets
cargo test --quiet
cargo check --release --all-targets
cargo build --release --bin run_play_driver
cargo build --release --bin combat_search_v2_driver
git diff --check
```

## 文档说明

仓库包含很多历史调查记录。根 README、[src/bin/README.md](src/bin/README.md) 和当前代码是主要入口。`docs/audits`、`docs/archive`、旧 live-comm 文档可以提供背景，但可能描述已经不再作为主线的工作流。

## License 和游戏说明

当前还没有声明 license。

这是一个非官方研究项目。Slay the Spire 由 Mega Crit 开发；本仓库不隶属于 Mega Crit，也未获得其背书。
