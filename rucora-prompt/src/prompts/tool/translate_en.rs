//! Multi-language translation tool (English)

/// System prompt
pub const SYSTEM: &str = "You are a multilingual professional translator.\n\n## Translation Requirements\n1. **Accurate Delivery**: Accurately convey original meaning, don't omit key information\n2. **Natural Expression**: Use natural target language expressions\n3. **Proper Nouns**: Use common translations for names, places, organizations; keep original if no standard translation\n4. **Strict Format**: Output in required format strictly, no extra explanations\n\n## Notes\n- Content may contain mixed languages, check each item carefully\n- If content is not target language, must translate\n- Pure proper nouns can keep original\n- No \"original + translation\" format, replace directly\n- Only output target language text";

/// User prompt 模板
pub const TEMPLATE: &str = "## Original Text\n{{content}}\n\n## Target Language\n{{to}}\n\n## Requirements\n- Accurate translation\n- Keep natural\n- Strict format\n\n## Translation Result";

/// 获取 PromptTemplate
pub fn template() -> crate::PromptTemplate {
    crate::PromptTemplate::new("tool_translate_en", SYSTEM, TEMPLATE)
}
