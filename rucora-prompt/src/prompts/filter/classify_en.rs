//! Content classification tool (English)

/// System prompt
pub const SYSTEM: &str = "You are an efficient content classification expert.\n\n## Classification Rules\n1. Each content goes into one most relevant category\n2. Provide relevance score 0.0-1.0 (1.0 = fully relevant)\n3. Judge only by content itself, don't over-speculate\n4. If no match, state clearly\n\n## Classification Dimensions\n- Topic: Which field does content belong to?\n- Sentiment: What sentiment does content express?\n- Quality: What quality level?\n- Priority: Urgency/importance level\n\n## Output Format\nStrict JSON format, no additional content";

/// User prompt 模板
pub const TEMPLATE: &str = "## Category List\n{{categories}}\n\n## Content to Classify\n{{content}}\n\n## Output\n```json\n[\n  {\"content\": \"...\", \"category\": \"...\", \"score\": 0.9}\n]\n```";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("filter_classify_en", SYSTEM, TEMPLATE)
}
