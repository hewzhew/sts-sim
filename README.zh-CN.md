# sts_simulator

[English README](README.md)

`sts_simulator` 是一个非官方的 Slay the Spire Rust 模拟器和 AI 搜索实验仓库。

这个仓库目前是研究和自动化工作区，不是已经整理成稳定 API 的库。当前目标是把模拟器状态、跑局决策、战斗搜索和实验 artifact 做到足够明确，让失败可以复现、复盘、改进，而不是靠终端日志猜。

## 当前主线

```text
typed simulator state
  -> typed non-combat owners and deck mutation bridges
  -> branch-tiny run capsules and seed panels
  -> combat cases for search review
  -> offline datasets and diagnostics when useful
```

当前设计方向是把策略、执行、诊断分开：

- owner 只产出 typed non-combat decision；
- runtime 执行 typed decision，不重新解析展示文本；
- combat search 只解决战斗内部问题；
- panel 和 review tool 输出证据，不充当 teacher label。

维护中的边界契约见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 快速开始

跑单个 owner-audit seed：

```powershell
cd D:\rust\sts_simulator
cargo run -p sts_oracle_tools --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

跑一个小 seed panel：

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

复盘保存下来的 combat case：

```powershell
cargo run -p sts_oracle_tools --bin combat_case_review -- --case <case.json> --ladder
```

当前维护的有界运行、续跑、combat search driver 和验证方式见
[docs/RUNBOOK.md](docs/RUNBOOK.md)。

## 主要入口

| Binary | 用途 |
| --- | --- |
| `branch_tiny` | 轻量跑局 runner，用于 owner 覆盖、run capsule、frontier continuation 和 combat case capture |
| `branch_panel` | Rust seed-panel scheduler，用于多 seed smoke/drain run |
| `combat_case_review` | saved combat case 的诊断 review ladder |
| `combat_search_v2` / `combat_search_v2_worker` | 轻量 CLI/help 前端与专用优化 worker，用于固定战斗搜索 |
| `rl_dataset_export` | 离线 decision sample 导出，用于 imitation/RL 实验 |

Binary 边界见 [src/bin/README.md](src/bin/README.md)。

## 文档地图

- [docs/README.md](docs/README.md): 当前文档索引。
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): ownership boundary 和设计规则。
- [docs/RUNBOOK.md](docs/RUNBOOK.md): 当前本地命令。
- [docs/TESTING.md](docs/TESTING.md): 测试归属和清理标准。
- [tools/README.md](tools/README.md): 离线工具边界和 artifact 规则。
- [src/ai/README.md](src/ai/README.md): AI 模块地图和清理方向。

退役文档不保留在工作区里污染搜索结果。需要考古时查 git history。

## 仓库结构

| 目录 | 角色 |
| --- | --- |
| `src/content` | Java 游戏内容复刻；避免随意改动 |
| `src/state` | run、combat、map、event、reward、engine state |
| `src/engine` | 状态转移和 action handler |
| `src/runtime` | run/combat 执行时支持 |
| `src/sim` | 面向模拟器的 legal action、apply、search 边界 |
| `src/ai` | policy、strategic facts、deck mutation、combat search、route/search work |
| `src/eval` | combat eval、run-control、learning adapter 的历史物理源码树；Cargo owner 由下列 crate 明确切分 |
| `src/bin` | 当前维护的命令入口 |
| `crates/sts_oracle_eval` | combat evaluation 与精确搜索编排的优化 Cargo owner |
| `crates/sts_oracle_run_control` | exact run session、决策应用与跑局证据的独立 Cargo owner |
| `crates/sts_oracle_learning_env` | 优化的 exact 单 episode learning env 与 opaque combat-root artifact owner |
| `crates/sts_oracle_learning` | model input 与批量 learning adapter 的下游 Cargo owner |
| `crates/sts_oracle_runtime` | branch 执行、持久化和服务编排的快速重建 Cargo owner |
| `crates/sts_combat_search_driver` | 轻量 combat-search 前端与 capability-scoped 优化 worker |
| `crates/sts_oracle_tools` | 无 library facade 的旧 oracle 命令适配器与集成契约宿主 |
| `learning` | 在线训练 caller、curriculum、seed 调度、模型和评估计账；不拥有模拟器机制 |
| `tools` | 离线脚本、dataset、panel 和生成 artifact |
| `docs` | 当前架构、runbook、测试说明和设计草稿 |

生成物应放在 ignored 位置，例如 `target/` 和 `tools/artifacts/`。

## 开发卫生

仓库中的 source、docs、PowerShell 脚本使用 LF 换行。在 Windows 上做机械改动后，检查小改动是否意外变成整文件 CRLF 重写：

```powershell
git diff --stat
git diff --ignore-space-at-eol --stat
git ls-files --eol $(git diff --name-only)
```

提交应该小而诚实。不要因为迁移麻烦就保留重复 policy 模块；边界准备好后，应该删除旧入口，而不是长期保留兼容层。

## 验证

只改文档时：

```powershell
git diff --check
```

核心代码改动从 [docs/RUNBOOK.md](docs/RUNBOOK.md) 里的命令开始。只有改动面存在稳定结构契约时，才运行或新增有针对性的测试。

## License 和游戏说明

当前还没有声明 license。

所移植依赖代码的许可证与固定上游版本见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

这是一个非官方研究项目。Slay the Spire 由 Mega Crit 开发；本仓库不隶属于 Mega Crit，也未获得其背书。
