# Agentic 自主研究示例 (rucora-deep-research-agentic)

## 原理

Agentic 自主研究示例展示了如何使用 `AgenticStrategy` 实现完全自主的研究流程。与传统预设流程不同，LLM 可以自主决定研究路径、选择搜索策略、评估信息质量。

## 核心组件

- **DefaultResearchEngine**: 默认研究引擎
- **AgenticStrategy**: 自主研究策略，让 LLM 主导研究流程
- **DeepResearchEngine trait**: 研究引擎接口

## 研究流程

1. **自主规划**: LLM 根据主题自主决定研究计划
2. **动态执行**: 根据每轮结果动态调整研究方向
3. **自我评估**: LLM 自主判断信息是否足够
4. **持续迭代**: 直到达到满意的研究深度

## 意义

1. **灵活性**: 摆脱预设流程的约束，适应复杂研究主题
2. **智能化**: LLM 可以识别知识盲区并主动填补
3. **类人化**: 模拟人类研究人员的思考过程

## 运行

```bash
cargo run --example rucora-deep-research-agentic
```

## 环境变量

需要设置以下环境变量：
- `OPENAI_API_KEY`: OpenAI API 密钥

## 适用场景

- 开放性研究主题
- 需要多角度探索的复杂问题
- 传统预设流程难以覆盖的领域

## 扩展

可以结合评分系统 (`ResearchQualityAssessor`) 来实现更精细的自主研究控制。