---
description: Future improvements for bottled_ai integration and differential testing pipeline
---

# Future Improvements Roadmap

This document tracks planned but **not yet implemented** improvements for the bottled_ai ↔ Rust simulator integration.

## 1. Labeling / Annotation System
- Mark key decision points in JSONL logs (e.g., "should have played Bash instead of Defend here")
- Tag combat turns with outcome quality (optimal / suboptimal / critical mistake)
- Use annotations for supervised learning training data

## 2. Human Takeover Mode
- Allow human player to intervene when AI makes bad decisions
- Seamlessly switch between AI and human control mid-run
- Record the "human override" as training signal

## 3. Backtracking / Replay from Checkpoint
- Save game state snapshots at key moments
- Replay from a checkpoint with different decisions
- Compare outcome trees (what-if analysis)

## 4. Trend Prediction
- Use state sequence data to predict combat win probability curves
- Track HP/deck strength trajectory across a run
- Identify inflection points (when a run becomes doomed)

## 5. Reference Decision Comparison
- Run Rust MCTS on each combat state from the JSONL log
- Compare bottled_ai's choice vs MCTS-optimal choice
- Quantify decision quality gap

## 6. Multi-language Support
- Build card name → card ID mapping table (Chinese → English → internal ID)
- Support Japanese, Korean, and other STS localizations
- Allow bottled_ai to work with any game language

## 7. Rust MCTS as bottled_ai Combat Backend
- Replace Python `get_best_battle_action()` with Rust MCTS via PyO3
- 100x+ speed improvement for combat path search
- Enable deeper search (100K+ paths vs 11K)

## 8. 怪物数据架构迁移（小 JSON → 全 Rust）
- **Phase 1**: `MonsterId` enum + 最小 JSON（仅 HP/数值/ascension）+ Java↔Rust 映射表
- **Phase 2**: `hardcoded_take_turn()` — 翻译 Java `takeTurn()` 到 Rust，替代 `resolve_intent()` 执行链
- **Phase 3**: 验证 — 重跑 full_turn_verify_test，确认 divergence 下降
- **Phase 4**: 全 Rust 常量 — JSON 归档为参考文件
- 审计工具：`tests/audit_json_vs_java.py`、`tests/extract_ids.py`

## 9. 统一 ID 体系（跨实体类型）
- 怪物、卡牌、药水、遗物都需要统一的 Rust canonical ID 命名规则
- 规则：游戏显示名 PascalCase 化，去冠词（The/A），大小用下划线后缀
- 建立完整的 Rust ID ↔ Java ID 映射表（每种实体类型一份）
- 之前卡牌/药水/遗物的名字不对应教训：名字不一致是 runtime bug 的主要来源

## 10. JSON 数值自动验证
- build-time 脚本从 Java 源码 regex 提取数值（HP、damage、block、ascension scaling）
- 与 JSON 数值交叉比对，发现不一致时报告
- 参考工具：`tests/audit_json_vs_java.py` 的升级版

## 11. 行为选择迁移（behavior_model → 全 hardcoded）
- 当前 57/66 怪物已有 hardcoded AI（getMove），剩余 9 个用 JSON behavior_model
- 将剩余 9 个也迁移到 `hardcoded_ai.rs`（每个仅 3-8 行）
- 之后 JSON 的 behavior_model 字段可完全移除

