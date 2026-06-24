# sts_simulator

[English README](README.md)

`sts_simulator` 是一个非官方的 Slay the Spire Rust 模拟器和 AI 搜索实验仓库。

当前主线：

```text
模拟器 -> 状态表示 -> search/rollout -> value -> policy improvement
```

项目现在不以旧 watch UI、Workbench、DecisionFrame、prompt 工程或 LLM 接管控制为主线。这些以后可以作为适配层回来，但不能定义模拟器真相或搜索质量。

## 当前工作流

当前维护的闭环是：

1. 从 Neow 开始运行确定性的模拟器 campaign
2. 在明确预算下同时保留若干战斗外分支
3. 在这些分支内部用 Combat Search V2 搜整场战斗轨迹
4. 分支失败时检查 checkpoint、最终 boss 战斗和 outcome dataset
5. 比较整局 outcome 和 sibling branch，而不是逐步动作模仿

Autopilot、route planner、card reward policy、trace、搜索托管战斗都是便利工具或证据工具，不是 teacher label。

## 快速开始

运行当前 campaign 工作流：

```powershell
cd D:\rust\sts_simulator
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Rounds 1
.\tools\campaign.ps1 -Inspect
```

调试 binary 时可以直接构建主 campaign driver：

```powershell
cd D:\rust\sts_simulator
cargo build --profile fast-run --bin branch_campaign_driver
```

需要自己玩或手动观察模拟器时，仍然使用 REPL：

```powershell
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
cargo run --profile fast-run --bin run_play_driver -- --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
```

常用 session 命令：

| 命令 | 含义 |
| --- | --- |
| `ar` | 用受保护 route/card/search 辅助自动推进到需要人类输入的位置 |
| `n` | 不允许 route planner 的受保护推进 |
| `nr` | 允许 route planner 的受保护推进 |
| `rs` / `rg` | 查看 route 建议 / 执行一次 route 选择 |
| `sc` | 从当前战斗边界运行 combat search |
| `sd` | 查看或调整搜索默认预算 |
| `mark <name>` | 在记录 trace 时保存 replay 书签 |
| `q` | 干净退出 |

从书签恢复：

```powershell
cargo run --profile fast-run --bin run_play_driver -- --goto <name> --search-wall-ms 100
```

当前玩法说明见 [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md)。

## 主要入口

| Binary | 用途 |
| --- | --- |
| `branch_campaign_driver` | 当前自动分支 campaign、checkpoint 检查、outcome 导出和 continuation 实验 |
| `run_play_driver` | 手动和半自动模拟器跑局、trace、bookmark、capture、baseline |
| `combat_search_v2_driver` | 从 start spec、combat capture 或 benchmark suite 跑整场战斗搜索 |
| `artifact_doctor` | 只读审计 benchmark artifact 目录 |

Binary 细节见 [src/bin/README.md](src/bin/README.md)。

## 当前文档

先读：

- [docs/CURRENT_DIRECTION.md](docs/CURRENT_DIRECTION.md)
- [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md)
- [docs/AUTOPILOT_BOUNDARY.md](docs/AUTOPILOT_BOUNDARY.md)

旧文档已经移动到：

```text
docs_legacy/2026-06-03_pre_rewrite/docs/
```

旧文档只用于考古。里面可能会提到已经退役或降级的 LLM、live-comm、watch UI、Workbench 或过时命令。

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

核心代码改动建议运行：

```powershell
cargo fmt --check
cargo check --all-targets
cargo test --quiet
cargo check --release --all-targets
cargo build --profile fast-run --bin branch_campaign_driver
cargo build --release --bin run_play_driver
cargo build --release --bin combat_search_v2_driver
git diff --check
```

纯文档改动至少运行：

```powershell
git diff --check
```

## License 和游戏说明

当前还没有声明 license。

这是一个非官方研究项目。Slay the Spire 由 Mega Crit 开发；本仓库不隶属于 Mega Crit，也未获得其背书。
