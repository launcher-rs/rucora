# Agentkit 待办事项

本文件记录从 Zeroclaw 架构分析中提炼的待改进任务。

> 参考报告: `docs/ZEROCLAW_ARCHITECTURE_ANALYSIS.md`

---

## P0 已完成

- [x] **Credential 清洗** — 工具输出返回 LLM 前自动脱敏敏感 KV
- [x] **孤儿 Tool 消息清理** — 每次循环前清理无配对的 tool 消息
- [x] **Type-safe Observability** — 待实施（将字符串事件改为枚举类型）

## P1 已完成

- [x] **Loop Detector** — 滑动窗口哈希检测循环调用
- [x] **Context Overflow 内联恢复** — 快速裁剪 + 紧急删除两阶段恢复
- [x] **History Atomic Pruning** — 原子化裁剪 assistant + tool 组（实现于 `emergency_history_trim`）
- [x] **Memory Namespace** — 支持命名空间、重要性评分、GDPR 导出、程序记忆存储
- [x] **Tool Filter Groups** — 工具分 always / dynamic 可见性组

## P2 已完成

- [x] **Hook Priority System** — Hook 优先级排序 + void/modifying 钩子区分
- [x] **RuntimeAdapter 抽象** — 跨平台运行时抽象（shell/fs 访问、内存预算等）
- [x] **Pure Interface Layer** — 纯接口层分离（无实现、无重依赖）
- [x] **Dual-track Metrics** — ObserverEvent（事件）与 ObserverMetric（指标）分离

## 其他待办

- [ ] 完善 Loop Detector 并发路径的 Warning 消息注入
- [ ] 为 Context Overflow 恢复添加单元测试
- [ ] 评估是否引入 `context_length` 到 `ErrorCategory` 枚举
