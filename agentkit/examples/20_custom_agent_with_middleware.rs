//! AgentKit 自定义 Agent 使用中间件示例
//!
//! 展示如何创建自定义 Agent 类型，并集成中间件系统。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 20_custom_agent_with_middleware
//! ```
//!
//! ## 功能演示
//!
//! 1. **自定义 Agent** - 创建带时间戳功能的 Agent
//! 2. **自定义中间件** - 实现时间戳、日志、限流中间件
//! 3. **中间件集成** - 展示如何将中间件集成到自定义 Agent
//! 4. **实际使用** - 演示自定义 Agent 的实际运行效果

use agentkit::agent::execution::DefaultExecution;
use agentkit::middleware::{
    LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareChain, RateLimitMiddleware,
};
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit_core::agent::{AgentContext, AgentDecision, AgentError, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::ChatRequest;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ═══════════════════════════════════════════════════════════
// 自定义中间件
// ═══════════════════════════════════════════════════════════

/// 时间戳中间件
#[derive(Clone)]
struct TimestampMiddleware;

#[async_trait]
impl Middleware for TimestampMiddleware {
    fn name(&self) -> &str {
        "timestamp"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        info!("middleware.timestamp: 收到请求时间 {}", timestamp);
        info!("middleware.timestamp: 原始输入 \"{}\"", input.text);
        Ok(())
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        info!("middleware.timestamp: 返回响应时间 {}", timestamp);
        if let Some(content) = output.value.as_object_mut() {
            content.insert("timestamp".to_string(), json!(timestamp));
        }
        Ok(())
    }
}

/// 输入验证中间件
#[derive(Clone)]
struct InputValidationMiddleware {
    max_length: usize,
}

impl InputValidationMiddleware {
    fn new(max_length: usize) -> Self {
        Self { max_length }
    }
}

#[async_trait]
impl Middleware for InputValidationMiddleware {
    fn name(&self) -> &str {
        "input_validation"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        if input.text.is_empty() {
            return Err(AgentError::Message("输入不能为空".to_string()));
        }
        if input.text.len() > self.max_length {
            return Err(AgentError::Message(format!(
                "输入过长：最大 {} 字符，当前 {} 字符",
                self.max_length,
                input.text.len()
            )));
        }
        info!("middleware.validation: 输入验证通过，长度 {}", input.text.len());
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════
// 自定义 Agent
// ═══════════════════════════════════════════════════════════

pub struct TimestampAgent<P> {
    #[allow(dead_code)]
    provider: Arc<P>,
    #[allow(dead_code)]
    system_prompt: Option<String>,
    execution: DefaultExecution,
}

impl<P> TimestampAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    pub fn builder() -> TimestampAgentBuilder<P> {
        TimestampAgentBuilder::new()
    }
}

#[async_trait]
impl<P> Agent for TimestampAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: ChatRequest {
                messages: context.messages.clone(),
                model: Some("gpt-4o-mini".to_string()),
                ..Default::default()
            },
        }
    }

    fn name(&self) -> &str {
        "timestamp_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("带时间戳功能的自定义 Agent")
    }

    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        self.execution.run(self, input).await
    }
}

// ═══════════════════════════════════════════════════════════
// 构建器
// ═══════════════════════════════════════════════════════════

pub struct TimestampAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    middleware_chain: MiddlewareChain,
}

impl<P> TimestampAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            middleware_chain: MiddlewareChain::new(),
        }
    }

    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_middleware_chain(mut self, middleware_chain: MiddlewareChain) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    pub fn with_middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    pub fn build(self) -> TimestampAgent<P> {
        let provider = Arc::new(self.provider.expect("Provider is required"));
        let system_prompt = self.system_prompt.clone();
        let execution = DefaultExecution::new(
            provider.clone(),
            "gpt-4o-mini",
            agentkit::agent::ToolRegistry::new(),
        )
        .with_system_prompt_opt(self.system_prompt)
        .with_middleware_chain(self.middleware_chain);

        TimestampAgent {
            provider,
            system_prompt,
            execution,
        }
    }
}

impl<P> Default for TimestampAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════
// 主函数
// ═══════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   自定义 Agent 使用中间件示例         ║");
    info!("╚════════════════════════════════════════╝\n");

    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 创建自定义 Agent（带中间件）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 创建自定义 Agent（带中间件）");
    info!("═══════════════════════════════════════\n");

    info!("1.1 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    info!("1.2 创建自定义 Agent（带中间件）...");
    let agent = TimestampAgent::builder()
        .provider(provider)
        .system_prompt("你是一个有用的助手，会在响应中自动添加时间戳。")
        .with_middleware(TimestampMiddleware)
        .with_middleware(LoggingMiddleware::new())
        .with_middleware(RateLimitMiddleware::new(60))
        .with_middleware(MetricsMiddleware::new())
        .with_middleware(InputValidationMiddleware::new(1000))
        .build();

    info!("✓ 自定义 Agent 创建成功");
    info!("  Agent 名称：{}", agent.name());
    info!("  Agent 描述：{:?}", agent.description());
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 测试自定义 Agent
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 测试自定义 Agent");
    info!("═══════════════════════════════════════\n");

    info!("2.1 测试简单对话...");
    let task1 = "你好";
    info!("  输入：\"{}\"", task1);

    match agent.run(task1.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("  输出：\"{}\"", text);
            }
            info!("  完整输出：{:?}", output.value);
        }
        Err(e) => {
            info!("  Agent 运行出错：{}", e);
        }
    }
    info!("");

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 自定义 Agent 使用中间件总结：\n");
    info!("1. 组合 DefaultExecution 获得中间件支持");
    info!("2. 实现 Agent trait 的 think() 和 run() 方法");
    info!("3. 创建 Builder 模式支持中间件配置");
    info!("4. 使用 with_middleware() 或 with_middleware_chain() 添加中间件\n");

    Ok(())
}
