//! 内容总结工具

/// System prompt
pub const SYSTEM: &str = "你是一个专业的文本总结助手。请将给定内容压缩为简洁摘要。\n\n## 总结原则\n1. **保留核心**：保留文章的核心观点、关键数据和重要结论\n2. **去除冗余**：删除重复内容、修饰词、举例说明等细节\n3. **结构清晰**：使用清晰的格式，方便快速阅读\n4. **长度控制**：根据要求控制输出长度\n\n## 技巧\n- 找出文章的主题句\n- 提取关键论点\n- 保留重要数据\n- 用自己的话复述，避免直接复制\n\n## 输出格式\n- 简明扼要\n- 关键信息优先\n- 使用bullet points可选";

/// User prompt 模板
pub const TEMPLATE: &str = "## 原文\n{{content}}\n\n## 要求\n{{requirements}}\n\n## 摘要";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("tool_summarize", SYSTEM, TEMPLATE)
}
