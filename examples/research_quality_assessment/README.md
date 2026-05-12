# 研究质量评分示例 (research_quality_assessment)

## 原理

研究质量评分示例展示了 rucora 框架中的评分系统。该系统可以自动评估研究质量并生成改进建议，帮助 LLM 自主判断研究是否完成以及如何改进。

## 核心组件

- **ResearchQualityScore**: 研究质量评分结构
  - `info_quality`: 信息质量评分
  - `completeness`: 完整性评分
  - `confidence`: 置信度评分
  - `overall`: 综合评分

- **ResearchSuggestion**: 研究改进建议
  - `NeedMoreInfo`: 信息不足
  - `NeedMoreSources`: 来源单一
  - `NeedNewAngle`: 需要新角度
  - `NeedValidation`: 需要验证
  - `Sufficient`: 已达标

- **ResearchQualityAssessor**: 质量评估器
  - `assess()`: 评估研究质量
  - `suggest()`: 生成改进建议
  - `should_continue()`: 判断是否继续搜索
  - `get_next_search_hint()`: 获取下一轮搜索提示

- **ScoringConfig**: 评分配置

## 评分维度

1. **信息质量**: 基于相关性分数和高信息量内容占比
2. **完整性**: 基于主题关键词覆盖率
3. **置信度**: 综合信息质量、完整性、来源多样性
4. **综合评分**: 加权综合上述三个维度

## 意义

1. **自动判断**: 无需人工干预，LLM 可自主决定是否继续研究
2. **智能建议**: 根据评分结果提供具体的改进方向
3. **成本控制**: 避免过度搜索，提高资源利用效率
4. **质量保证**: 确保研究达到最低质量标准

## 运行

```bash
cargo run --example research_quality_assessment
```

## 使用示例

```rust
use rucora_core::research::ResearchQualityAssessor;

// 创建评估器
let assessor = ResearchQualityAssessor::with_default("Rust 异步编程");

// 评估质量
let score = assessor.assess(&info_pieces, &citations, 3);

// 生成建议
let suggestion = assessor.suggest(&score);

// 判断是否继续
if assessor.should_continue(&score, 3, 10) {
    let next_hint = assessor.get_next_search_hint(&score, "Rust");
}
```

## 扩展

可以通过 `ScoringConfig` 自定义评分阈值，适应不同场景的质量要求。