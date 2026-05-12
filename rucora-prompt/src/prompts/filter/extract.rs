//! 信息提取工具

/// System prompt
pub const SYSTEM: &str = "你是一个信息提取专家。\n\n## 提取任务\n从给定内容中提取指定类型的信息。\n\n## 提取类型\n1. **实体提取**：人名、地名、机构名、时间等\n2. **关系提取**：实体之间的关系\n3. **事件提取**：发生了什么事件\n4. **属性提取**：对象的特征和属性\n5. **关键词提取**：核心关键词和主题\n\n## 提取原则\n- 严格按照要求的格式输出\n- 不添加内容，只提取已有信息\n- 如信息不存在，明确说明\n- 保持提取结果的一致性\n\n## 输出格式\nJSON 格式，严格按要求字段返回";

/// User prompt 模板
pub const TEMPLATE: &str = "## 待提取内容\n{{content}}\n\n## 提取类型\n{{extract_type}}\n\n## 提取字段\n{{fields}}\n\n## 输出格式\n```json\n{\n  \"entities\": [...],\n  \"relations\": [...],\n  ...\n}\n```";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("filter_extract", SYSTEM, TEMPLATE)
}
