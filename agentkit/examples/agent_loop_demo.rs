use agentkit_core::{
    agent::types::{AgentInput, AgentOutput},
    error::{AgentError, ProviderError},
    provider::LlmProvider,
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
    runtime::Runtime,
};
use async_trait::async_trait;
use std::sync::Mutex;

/// 一个最小可运行的示例：演示“自定义 Runtime”。
///
/// ## “可插拔 AgentLoop”是什么意思？（通俗解释）
///
/// 你可以把 `Runtime` 想象成一台“执行引擎”，它把很多通用能力都打包好了，比如：
/// - 维护对话消息（messages）的输入输出
/// - 统一对接 LLM（provider）
/// - 注册/管理工具（tools）以及工具调用结果（tool_results）
/// - 做一些通用的策略/审计/Trace 等（取决于你在 runtime 里启用的组件）
///
/// 但“这台机器怎么转起来”，也就是：
/// - 每一步该发什么 prompt/请求给 LLM
/// - LLM 返回后要不要继续下一步
/// - 遇到 tool call 要不要执行工具、怎么执行、执行完如何把结果塞回下一轮
/// - 失败时如何重试、最大步数如何控制、何时终止
///
/// 这些**执行流程**由 `Runtime::run()` 决定。
///
/// ## 为什么要这样设计？
///
/// 不同应用场景需要不同的“循环策略”，例如：
/// - 只想问一次就结束（Single-shot），不需要工具、也不需要多轮推理
/// - 标准 tool-calling：LLM 先决定要不要调用工具，调用后再把结果喂回去继续
/// - ReAct / Plan-and-Execute：先规划再执行，可能有多阶段、多轮
/// - 严格的安全/成本控制：限制步数、限制工具、限制 token、超时就中止
///
/// 如果把这些都写死在一个地方，代码会越来越难维护；而可插拔 loop 让你可以：
/// - 为不同产品/不同实验快速切换执行策略
/// - 用最小的 loop 复现问题、做单元测试、做基准测试
/// - 在不改动 agent 其他部分的情况下，独立演进“执行流程”
///
/// 运行方式：
/// - `cargo run -p agentkit --example agent_loop_demo`
///
/// 该示例不依赖任何外部 API Key：
/// - 使用一个本地 `TestProvider` 来模拟 LLM 行为
/// - 使用一个自定义 `AgentLoop` 替换默认的 tool-calling loop
#[tokio::main]
async fn main() {
    let provider = TestProvider {
        calls: Mutex::new(0),
    };

    let rt = SingleShotRuntime { provider };

    let out = rt
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "你好".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await
        .expect("runtime.run 失败");

    println!("assistant: {}", out.message.content);
}

/// 一个用于演示的 provider：
/// - 每次调用都会把 `calls` +1
/// - 返回固定的 assistant 消息
struct TestProvider {
    calls: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for TestProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut c = self.calls.lock().unwrap();
        *c += 1;

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("ok(calls={})", *c),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

pub struct SingleShotRuntime<P> {
    provider: P,
}

#[async_trait]
impl<P> Runtime for SingleShotRuntime<P>
where
    P: LlmProvider,
{
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        let req = ChatRequest {
            messages: input.messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: input.metadata,
        };

        let resp = self
            .provider
            .chat(req)
            .await
            .map_err(|e| AgentError::Message(e.to_string()))?;

        Ok(AgentOutput {
            message: resp.message,
            tool_results: vec![],
        })
    }
}
