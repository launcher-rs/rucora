//! Information extraction tool (English)

/// System prompt
pub const SYSTEM: &str = "You are an information extraction expert.\n\n## Extraction Task\nExtract specified type of information from given content.\n\n## Extraction Types\n1. **Entity Extraction**: Names, places, organizations, times, etc.\n2. **Relation Extraction**: Relationships between entities\n3. **Event Extraction**: What events occurred\n4. **Attribute Extraction**: Object characteristics and properties\n5. **Keyword Extraction**: Core keywords and themes\n\n## Extraction Principles\n- Output strictly in required format\n- Don't add content, only extract existing information\n- If info doesn't exist, state clearly\n- Keep extraction results consistent\n\n## Output Format\nJSON format, strictly follow required fields";

/// User prompt 模板
pub const TEMPLATE: &str = "## Content to Extract From\n{{content}}\n\n## Extraction Type\n{{extract_type}}\n\n## Extraction Fields\n{{fields}}\n\n## Output Format\n```json\n{\n  \"entities\": [...],\n  \"relations\": [...],\n  ...\n}\n```";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("filter_extract_en", SYSTEM, TEMPLATE)
}
