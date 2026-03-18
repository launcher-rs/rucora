use agentkit_core::{
    agent::types::{AgentInput, AgentOutput},
    error::AgentError,
    provider::{
        LlmProvider,
        types::{ChatMessage, ChatRequest, Role},
    },
    runtime::Runtime,
    tool::Tool,
};
use async_trait::async_trait;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

/// 自定义 runtime：最小的 llm + tool loop 实现（不依赖 agentkit-runtime）。
///
/// 核心逻辑：
/// - 把 `tools` 的 schema 注册给 provider（`ChatRequest.tools`）
/// - 调用 `provider.chat()`
/// - 若模型返回 `tool_calls`：依次执行工具，并把结果以 `Role::Tool` 消息回灌到 messages
/// - 重复直到没有 tool_calls 或达到步数上限
///
/// 注意：
/// - 本示例依赖 provider 正确填充 `ChatResponse.tool_calls`（OpenAI-compatible API 通常支持）
/// - 该 runtime 只演示最小闭环：不含重试/超时/并发/安全沙箱等
pub struct LlmToolRuntime<P> {
    /// LLM provider。
    provider: P,
    /// 可用工具注册表。
    tools: ToolsRegistry,
    /// 可选系统提示词。
    system_prompt: Option<String>,
    /// 最大循环步数，避免无限工具调用。
    max_steps: usize,
}

impl<P> LlmToolRuntime<P> {
    /// 创建一个 LlmToolRuntime。
    pub fn new(provider: P, tools: ToolsRegistry) -> Self {
        Self {
            provider,
            tools,
            system_prompt: None,
            max_steps: 8,
        }
    }

    /// 设置系统提示词。
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置最大循环步数。
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// 执行单个工具调用。
    async fn execute_tool_call(
        &self,
        call: &agentkit_core::tool::types::ToolCall,
    ) -> Result<agentkit_core::tool::types::ToolResult, AgentError> {
        let tool = self.tools.get(&call.name).ok_or_else(|| {
            AgentError::Message(format!("未找到工具: {} (tool_call_id={})", call.name, call.id))
        })?;

        let out = tool
            .call(call.input.clone())
            .await
            .map_err(|e| AgentError::Message(format!("工具执行失败: {}", e)))?;

        Ok(agentkit_core::tool::types::ToolResult {
            tool_call_id: call.id.clone(),
            output: out,
        })
    }

    /// 把工具结果转换为 tool 消息（回灌给模型）。
    fn tool_result_to_message(
        result: &agentkit_core::tool::types::ToolResult,
        tool_name: &str,
    ) -> ChatMessage {
        // 约定：tool 消息 content 用 JSON 字符串承载，便于 provider/模型统一解析。
        let payload = json!({
            "tool_call_id": result.tool_call_id,
            "output": result.output
        });

        ChatMessage {
            role: Role::Tool,
            content: payload.to_string(),
            name: Some(tool_name.to_string()),
        }
    }
}

/// 最小工具注册表。
///
/// 说明：
/// - 该类型仅用于演示“自定义 runtime 不依赖 agentkit-runtime”。
/// - 实际项目中你可以实现更复杂的 registry（例如按 category 索引、支持别名、统计、限流等）。
#[derive(Default, Clone)]
pub struct ToolsRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolsRegistry {
    /// 创建一个空注册表。
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具。
    ///
    /// 注意：如果同名工具重复注册，后者会覆盖前者。
    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
        self
    }

    /// 按名称查找工具。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// 获取工具定义列表（用于注册到 provider）。
    pub fn definitions(&self) -> Vec<agentkit_core::tool::types::ToolDefinition> {
        self.tools
            .values()
            .map(|tool| agentkit_core::tool::types::ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect()
    }
}

#[async_trait]
impl<P> Runtime for LlmToolRuntime<P>
where
    P: LlmProvider,
{
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        let mut messages = input.messages;
        if let Some(system_prompt) = &self.system_prompt {
            messages.insert(0, ChatMessage {
                role: Role::System,
                content: system_prompt.clone(),
                name: None,
            });
        }

        let tool_defs = self.tools.definitions();
        let mut tool_results: Vec<agentkit_core::tool::types::ToolResult> = Vec::new();

        for step in 0..self.max_steps {
            let req = ChatRequest {
                messages: messages.clone(),
                model: None,
                tools: Some(tool_defs.clone()),
                temperature: None,
                max_tokens: None,
                metadata: input.metadata.clone(),
            };

            let resp = self
                .provider
                .chat(req)
                .await
                .map_err(|e| AgentError::Message(format!("provider.chat 失败: {}", e)))?;

            // 追加 assistant 回复到对话历史。
            messages.push(resp.message.clone());

            // 没有工具调用：返回最终消息。
            if resp.tool_calls.is_empty() {
                return Ok(AgentOutput {
                    message: resp.message,
                    tool_results,
                });
            }

            // 执行工具调用并回灌。
            for call in resp.tool_calls.iter() {
                let result = self.execute_tool_call(call).await?;
                let tool_msg = Self::tool_result_to_message(&result, &call.name);
                tool_results.push(result);
                messages.push(tool_msg);
            }

            // 仅用于调试：避免 unused warning（也能让读者看清 step）。
            let _ = step;
        }

        Err(AgentError::Message(format!(
            "超过最大步数限制（max_steps={}），仍未结束工具调用流程",
            self.max_steps
        )))
    }
}

#[tokio::main]
async fn main() {
    // 说明：该示例演示“自定义 runtime 自己实现 llm+tool loop”，并且不依赖 agentkit-runtime。
    // 运行前准备：
    // - 启动 ollama（并启用 OpenAI-compatible /v1 接口）
    // - 或把 base_url 改成你的 OpenAI-compatible 服务

    // 这里使用 OpenAI-compatible provider（例如 Ollama /v1）。
    let provider = agentkit::provider::OpenAiProvider::new("http://127.0.0.1:11434/v1", "ollama")
        .with_default_model("qwen2.5:14b");

    // 注册工具：你可以把更多工具加入 registry。
    let tools = ToolsRegistry::new().register(agentkit::tools::EchoTool);

    let runtime = LlmToolRuntime::new(provider, tools)
        .with_system_prompt(
            "你是一个严谨的助手。你可以调用工具来完成任务。\n\
当用户明确要求你调用 echo 工具时，请返回 tool_call（name=echo, input={text:...}）。\n\
在拿到工具结果后，再用中文给出最终回答。",
        )
        .with_max_steps(6);

    let out = runtime
        .run(AgentInput {
            messages: vec![ChatMessage::user("请调用 echo 工具，把文本原样返回：hello runtime")],
            metadata: None,
        })
        .await;

    match out {
        Ok(out) => {
            println!("assistant: {}", out.message.content);
        }
        Err(e) => {
            eprintln!("运行失败：{e}");
        }
    }
}
