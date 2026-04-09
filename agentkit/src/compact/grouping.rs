//! 消息分组模块
//!
//! 按 API 轮次对消息进行分组，而非按用户轮次。

use agentkit_core::provider::types::{ChatMessage, Role};

/// 按 API 轮次分组消息
///
/// 每个组代表一次 API 往返：
/// - 从用户消息开始
/// - 包含 assistant 的响应
/// - 包含所有相关的工具调用和结果
///
/// # 参数
/// * `messages` - 消息列表
///
/// # 返回值
/// 分组后的消息列表
pub fn group_messages_by_api_round(messages: &[ChatMessage]) -> Vec<Vec<ChatMessage>> {
    let mut groups: Vec<Vec<ChatMessage>> = Vec::new();
    let mut current_group: Vec<ChatMessage> = Vec::new();
    let mut last_role: Option<Role> = None;

    for msg in messages {
        // 当遇到用户消息且当前组不为空时，说明开始了新的 API 轮次
        // 或者当遇到 assistant 消息且上一条也是 assistant 时，开始新组
        if msg.role == Role::User && !current_group.is_empty() {
            // 保存当前组，开始新组
            groups.push(current_group);
            current_group = vec![msg.clone()];
            last_role = Some(msg.role.clone());
            continue;
        }

        // 如果是连续的 assistant 消息（工具调用等情况），开始新组
        if msg.role == Role::Assistant
            && last_role == Some(Role::Assistant)
            && !current_group.is_empty()
        {
            // 检查是否应该开始新组（不同的 assistant 响应）
            if should_start_new_group(&current_group, msg) {
                groups.push(current_group);
                current_group = vec![msg.clone()];
                last_role = Some(msg.role.clone());
                continue;
            }
        }

        current_group.push(msg.clone());
        last_role = Some(msg.role.clone());
    }

    // 添加最后一组
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

/// 判断是否应该开始新的组
fn should_start_new_group(current_group: &[ChatMessage], msg: &ChatMessage) -> bool {
    // 如果当前组最后一条是 assistant 消息，且新消息也是 assistant 消息
    // 则开始新组
    if let Some(last) = current_group.last()
        && last.role == Role::Assistant && msg.role == Role::Assistant {
            return true;
        }

    false
}

/// 选择要压缩的组
///
/// 保留最近的组，压缩较早的组
///
/// # 参数
/// * `groups` - 消息组列表
/// * `preserve_count` - 要保留的最近组数量
///
/// # 返回值
/// 要压缩的组
pub fn select_groups_to_compact(
    groups: &[Vec<ChatMessage>],
    preserve_count: usize,
) -> Vec<Vec<ChatMessage>> {
    if groups.len() <= preserve_count {
        return Vec::new();
    }

    let groups_to_compact = groups.len() - preserve_count;
    groups[..groups_to_compact].to_vec()
}

/// 将消息组转换为文本
///
/// # 参数
/// * `groups` - 消息组列表
///
/// # 返回值
/// 格式化的文本
pub fn groups_to_text(groups: &[Vec<ChatMessage>]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for (i, group) in groups.iter().enumerate() {
        let mut group_text = format!("=== 轮次 {} ===\n", i + 1);

        for msg in group {
            let role = match msg.role {
                Role::User => "用户",
                Role::Assistant => "助手",
                Role::System => "系统",
                Role::Tool => "工具",
            };

            group_text.push_str(&format!("[{}]: {}\n", role, msg.content));
        }

        parts.push(group_text);
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_messages() {
        let messages = vec![
            ChatMessage::user("你好"),
            ChatMessage::assistant("你好！有什么可以帮助你的吗？"),
            ChatMessage::user("帮我写个函数"),
            ChatMessage::assistant("好的，我来帮你写。"),
        ];

        let groups = group_messages_by_api_round(&messages);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_select_groups_to_compact() {
        let groups = vec![
            vec![ChatMessage::user("消息 1")],
            vec![ChatMessage::user("消息 2")],
            vec![ChatMessage::user("消息 3")],
            vec![ChatMessage::user("消息 4")],
        ];

        // 保留最后 2 组，压缩前 2 组
        let to_compact = select_groups_to_compact(&groups, 2);
        assert_eq!(to_compact.len(), 2);
    }

    #[test]
    fn test_groups_to_text() {
        let groups = vec![vec![
            ChatMessage::user("你好"),
            ChatMessage::assistant("你好！"),
        ]];

        let text = groups_to_text(&groups);
        assert!(text.contains("轮次 1"));
        assert!(text.contains("用户"));
        assert!(text.contains("助手"));
    }
}
