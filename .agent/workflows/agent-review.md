---
description: Post-AI-generation review checklist. Use after Agent generates significant code changes to catch common blind spots before accepting.
---

# Agent Code Review Checklist

基于 Vibe Coding 文章的五层 review 框架，专注于 Agent 最容易出错的地方。

## L1: 问题定义（最重要，Agent 最弱）
- [ ] 这个功能/修复真的需要做吗？
- [ ] 有没有更简单的方案能达到 80% 效果？
- [ ] 做完后怎么验证它是有用的？

## L2: 架构设计（Agent 局部最优，需要全局视角）
- [ ] 新代码与现有系统的交互是否考虑完整？
- [ ] 数据流是否合理？（入口→处理→出口）
- [ ] 错误如何传播？是否被正确处理或至少被记录？

## L3: 关键决策把关
- [ ] 数据模型变更是否向后兼容？（新增字段默认值？）
- [ ] 有没有引入不必要的第三方依赖？
- [ ] 并发/一致性是否考虑？（State 在多线程下安全吗？）

## L4: 质量防线
- [ ] 边界条件：空输入、超大输入、负数、None/null
- [ ] 性能：循环里有 O(n²) 操作吗？有不必要的 clone 吗？
- [ ] 凌晨三点测试：出问题时日志能定位到这段代码吗？

## L5: 知识萃取
- [ ] Agent 用了什么我没见过的模式？值得学习吗？
- [ ] 这次修复暴露了哪些认知盲点？记录到 SKILL.md

## STS 项目专用检查
- [ ] `interop.rs` 中所有生命周期钩子是否正确接入？
- [ ] 新增的 `CardInstance` 字段是否添加到所有构造函数（schema.rs + loader.rs）？
- [ ] 修改 Rust 后是否跑了 `maturin develop --release`？
- [ ] Reward 系统变更是否考虑了获取顺序的影响？
