//! 压缩提示词模板模块

/// 基础压缩提示词模板
pub const BASE_COMPACT_PROMPT: &str = r#"你的任务是对对话进行详细摘要，以便后续继续开发工作而不丢失上下文。

请包含以下部分：

1. **主要请求和意图**：详细捕获用户的明确请求和意图
2. **关键技术概念**：列出所有重要的技术概念、技术和框架
3. **文件和代码段**：
   - 枚举具体检查和修改的文件
   - 包含完整代码片段
   - 说明为什么这个文件重要
   - 描述对这个文件的更改
4. **错误和修复**：
   - 列出所有遇到的错误
   - 描述如何修复每个错误
   - 记录用户的反馈（如果有）
5. **问题解决**：记录解决的问题和正在进行的调试工作
6. **所有用户消息**：列出所有非工具结果的用户消息
7. **待处理任务**：概述明确要求处理的待处理任务
8. **当前工作**：详细描述最近正在处理的工作
9. **可选下一步**：列出与最近工作相关的下一步

请基于以上结构提供详细的摘要。

输出格式示例：

<analysis>
[你的思考过程，确保全面准确地涵盖所有要点]
</analysis>

<summary>
1. 主要请求和意图：
   [详细描述]

2. 关键技术概念：
   - [概念 1]
   - [概念 2]
   - [...]

3. 文件和代码段：
   - [文件名 1]
     - [为什么这个文件重要]
     - [对这个文件的更改]
     - [重要代码片段]
   - [...]

4. 错误和修复：
   - [错误 1 的详细描述]
     - [如何修复]
     - [用户反馈]
   - [...]

5. 问题解决：
   [描述已解决的问题和正在进行的调试]

6. 所有用户消息：
   - [详细的非工具用户消息]
   - [...]

7. 待处理任务：
   - [任务 1]
   - [任务 2]
   - [...]

8. 当前工作：
   [精确描述当前工作]

9. 可选下一步：
   [与最近工作相关的下一步]
</summary>

请根据目前的对话提供摘要，遵循此结构并确保精确和全面。
"#;

/// 部分压缩提示词（仅压缩最近的消息）
pub const PARTIAL_COMPACT_PROMPT: &str = r#"你的任务是对对话的**最近部分**进行摘要——即早期保留上下文之后的消息。早期的消息保持完整，不需要总结。

专注于总结最近消息中讨论、学习和完成的内容。

请包含以下部分：

1. **最近的请求和意图**：捕获用户最近的明确请求
2. **最近的技术概念**：列出最近讨论的技术概念
3. **最近的文件和代码**：最近检查和修改的文件
4. **最近的错误和修复**：最近遇到的错误及修复方法
5. **最近的进展**：最近解决的问题和完成的工作
6. **待处理任务**：当前待处理的任务
7. **当前工作**：最近正在处理的工作
8. **下一步**：下一步要采取的行动

请基于以上结构提供简洁的摘要。
"#;

/// 压缩指令（可添加到基础提示词）
pub const COMPACT_INSTRUCTIONS: &str = r#"
以下是额外的压缩指令：

{instructions}

请在创建摘要时遵循这些指令。
"#;

/// 生成压缩提示词
pub fn generate_compact_prompt(instructions: Option<&str>) -> String {
    match instructions {
        Some(instr) => {
            let mut prompt = BASE_COMPACT_PROMPT.to_string();
            prompt.push_str(&COMPACT_INSTRUCTIONS.replace("{instructions}", instr));
            prompt
        }
        None => BASE_COMPACT_PROMPT.to_string(),
    }
}

/// 生成部分压缩提示词
pub fn generate_partial_compact_prompt(instructions: Option<&str>) -> String {
    match instructions {
        Some(instr) => {
            let mut prompt = PARTIAL_COMPACT_PROMPT.to_string();
            prompt.push_str(&COMPACT_INSTRUCTIONS.replace("{instructions}", instr));
            prompt
        }
        None => PARTIAL_COMPACT_PROMPT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_compact_prompt() {
        let prompt = generate_compact_prompt(None);
        assert!(prompt.contains("主要请求和意图"));
        assert!(prompt.contains("关键技术概念"));
        
        let prompt_with_instructions = generate_compact_prompt(Some("只关注 Rust 代码更改"));
        assert!(prompt_with_instructions.contains("只关注 Rust 代码更改"));
    }
    
    #[test]
    fn test_generate_partial_compact_prompt() {
        let prompt = generate_partial_compact_prompt(None);
        assert!(prompt.contains("最近部分"));
        assert!(prompt.contains("最近的消息"));
    }
}
