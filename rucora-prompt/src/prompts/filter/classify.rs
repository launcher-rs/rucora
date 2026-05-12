//! 内容分类工具

/// System prompt
pub const SYSTEM: &str = "你是一个高效的内容分类专家。\n\n## 分类规则\n1. 每条内容只归入一个最相关的类别\n2. 给出 0.0-1.0 的相关度分数（1.0=完全相关）\n3. 只根据内容本身判断，不过度推测\n4. 如不匹配任何类别，明确说明\n\n## 分类维度\n- 主题分类：内容属于哪个领域\n- 情感分类：内容表达的情感倾向\n- 质量分类：内容质量等级\n- 优先级分类：紧急/重要程度\n\n## 输出格式\n严格JSON格式，不要添加其他内容";

/// User prompt 模板
pub const TEMPLATE: &str = "## 类别列表\n{{categories}}\n\n## 待分类内容\n{{content}}\n\n## 输出\n```json\n[\n  {\"content\": \"...\", \"category\": \"...\", \"score\": 0.9}\n]\n```";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("filter_classify", SYSTEM, TEMPLATE)
}
