# sts_simulator

[English README](README.md)

`sts_simulator` 是一个非官方的 Slay the Spire Rust 模拟器和 AI 搜索实验仓库。

当前主线：

```text
simulator correctness
  -> Rust-owned campaign application
  -> source/output/continuation lifecycle
  -> search/rollout evidence when needed
```

项目现在不以旧 watch UI、Workbench、DecisionFrame、prompt 工程或 LLM 接管控制为主线。这些以后可以作为适配层回来，但不能定义模拟器真相或搜索质量。

## 当前工作流

当前维护的闭环是：

1. 继续或检查时先解析 source artifact
2. 每次 run/continue 都分配新的 output artifact
3. 用小而明确的 round budget 运行新 campaign 或继续 source

Autopilot、route planner、card reward policy、trace、搜索托管战斗都是便利工具或证据工具，不是 teacher label。

campaign 系统由 typed Rust application boundary 拥有。PowerShell wrapper 只是本地 source/output/continuation launcher，不是架构本身。见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 快速开始

检查架构、CLI 行为或 artifact 语义时，优先直接调用 Rust campaign surface：

```powershell
cd D:\rust\sts_simulator
cargo run --profile fast-run --bin branch_campaign_driver -- campaign run --preset quick --seed 1 --rounds 0
```

需要当前短别名时再用本地 launcher：

```powershell
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
.\tools\campaign.ps1 -From latest -Inspect
```

把 wrapper command 当成 launch alias，不要当成架构。branch-tiny panel、
combat case review、手动 REPL、search driver 和验证命令见
[docs/RUNBOOK.md](docs/RUNBOOK.md)。

## 主要入口

| Binary | 用途 |
| --- | --- |
| `branch_campaign_driver` | 当前自动分支 campaign、checkpoint 检查、outcome 导出和 continuation 实验 |
| `branch_tiny` | 轻量 owner-audit runner，负责 run capsule、frontier continuation 和 gap-panel 诊断 |
| `run_play_driver` | 手动和半自动模拟器跑局、trace、bookmark、capture、baseline |
| `combat_search_v2_driver` | 从 start spec、combat capture 或 benchmark suite 跑整场战斗搜索 |
| `combat_case_review` | 检查 branch-tiny combat gap 保存下来的 combat case |

Binary 细节见 [src/bin/README.md](src/bin/README.md)。

## 当前文档

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): 当前 ownership boundary 和设计规则。
- [docs/RUNBOOK.md](docs/RUNBOOK.md): 当前本地命令和验证方式。

退役文档不保留在工作区里污染搜索结果。需要考古时请查 git history。

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

## 验证

当前验证命令见 [docs/RUNBOOK.md](docs/RUNBOOK.md)。只有改动面存在稳定结构契约时，才运行或新增有针对性的测试。

## License 和游戏说明

当前还没有声明 license。

这是一个非官方研究项目。Slay the Spire 由 Mega Crit 开发；本仓库不隶属于 Mega Crit，也未获得其背书。
