//! AgentKit 中间件示例
//!
//! 展示中间件系统的概念和使用方法，以及如何将中间件内嵌到不同类型的 Agent 中。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 08_middleware
//! ```
//!
//! ## 功能演示
//!
//! 1. **中间件链** - 创建和配置中间件链
//! 2. **自定义中间件** - 实现认证、过滤、格式化中间件
//! 3. **SimpleAgent + 中间件** - 简单问答场景
//! 4. **ChatAgent + 中间件** - 多轮对话场景
//! 5. **ToolAgent + 中间件** - 工具调用场景
//! 6. **ReActAgent + 中间件** - 推理行动场景
//! 7. **ReflectAgent + 中间件** - 反思迭代场景

use agentkit::agent::{ChatAgent, ReActAgent, ReflectAgent, SimpleAgent, ToolAgent};
use agentkit::middleware::{
    CacheMiddleware, LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareChain,
    RateLimitMiddleware,
};
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::EchoTool;
use agentkit_core::agent::{AgentError, AgentInput, AgentOutput};
use agentkit_core::tool::types::{ToolCall, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ═══════════════════════════════════════════════════════════
// 自定义中间件示例
// ═══════════════════════════════════════════════════════════

/// 自定义认证中间件
///
/// 验证用户身份（示例实现）
#[derive(Clone)]
struct AuthMiddleware {
    api_key: String,
}

impl AuthMiddleware {
    fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    fn name(&self) -> &str {
        "auth"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        // 检查输入中是否包含 API Key
        if !self.api_key.is_empty() {
            info!("middleware.auth: 验证 API Key...");
            // 简化实现：实际应该验证 token 或 session
            if input.text.contains("UNAUTHORIZED") {
                return Err(AgentError::Message("认证失败：无效的 API Key".to_string()));
            }
        }
        Ok(())
    }

    async fn on_response(&self, _output: &mut AgentOutput) -> Result<(), AgentError> {
        info!("middleware.auth: 响应已验证");
        Ok(())
    }
}

/// 自定义敏感词过滤中间件
///
/// 过滤输入中的敏感词
#[derive(Clone)]
struct SensitiveWordFilterMiddleware {
    sensitive_words: Vec<String>,
}

impl SensitiveWordFilterMiddleware {
    fn new(sensitive_words: Vec<&str>) -> Self {
        Self {
            sensitive_words: sensitive_words.into_iter().map(String::from).collect(),
        }
    }
}

#[async_trait]
impl Middleware for SensitiveWordFilterMiddleware {
    fn name(&self) -> &str {
        "sensitive_word_filter"
    }

    async fn on_request(&self, input: &mut AgentInput) -> Result<(), AgentError> {
        let mut filtered = input.text.clone();
        for word in &self.sensitive_words {
            if filtered.contains(word.as_str()) {
                info!("middleware.sensitive_word_filter: 检测到敏感词 \"{}\"", word);
                filtered = filtered.replace(word.as_str(), "***");
            }
        }
        input.text = filtered;
        Ok(())
    }
}

/// 自定义响应格式化中间件
///
/// 格式化输出内容
#[derive(Clone)]
struct ResponseFormatMiddleware;

#[async_trait]
impl Middleware for ResponseFormatMiddleware {
    fn name(&self) -> &str {
        "response_format"
    }

    async fn on_response(&self, output: &mut AgentOutput) -> Result<(), AgentError> {
        // 在响应中添加格式化标记
        info!("middleware.response_format: 格式化响应");
        
        // 修改输出内容，添加标记
        if let Some(content) = output.value.as_object_mut() {
            content.insert("formatted".to_string(), json!(true));
        }
        
        Ok(())
    }
}

/// 工具调用日志中间件
///
/// 记录工具调用的输入输出
#[derive(Clone)]
struct ToolCallLoggingMiddleware;

#[async_trait]
impl Middleware for ToolCallLoggingMiddleware {
    fn name(&self) -> &str {
        "tool_call_logging"
    }

    async fn on_tool_call_before(&self, call: &mut ToolCall) -> Result<(), AgentError> {
        info!(
            "middleware.tool_call_before: 工具调用 \"{}\", 输入：{}",
            call.name, call.input
        );
        Ok(())
    }

    async fn on_tool_call_after(&self, result: &mut ToolResult) -> Result<(), AgentError> {
        info!(
            "middleware.tool_call_after: 工具结果 \"{}\", 输出：{}",
            result.tool_call_id, result.output
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 中间件系统示例             ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    let has_api_config = std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("OPENAI_BASE_URL").is_ok();

    if !has_api_config {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        info!("\n注意：以下 Agent 演示将跳过实际 API 调用\n");
    }

    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 创建中间件链
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 创建中间件链");
    info!("═══════════════════════════════════════\n");

    info!("1.1 创建中间件链...");
    let chain = MiddlewareChain::new()
        .with(LoggingMiddleware::new())
        .with(CacheMiddleware::new())
        .with(RateLimitMiddleware::new(100));

    info!("✓ 中间件链创建成功");
    info!("  中间件数量：{}\n", chain.len());

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 指标收集中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 指标收集中间件");
    info!("═══════════════════════════════════════\n");

    info!("2.1 创建指标中间件...");
    let metrics = MetricsMiddleware::new();
    info!("✓ 指标中间件创建成功\n");

    info!("2.2 初始指标:");
    info!("  请求计数：{}", metrics.get_request_count());
    info!("  响应计数：{}\n", metrics.get_response_count());

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 自定义中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 自定义中间件");
    info!("═══════════════════════════════════════\n");

    info!("3.1 创建认证中间件...");
    let auth_middleware = AuthMiddleware::new("sk-test-key");
    info!("✓ 认证中间件创建成功\n");

    info!("3.2 创建敏感词过滤中间件...");
    let filter_middleware = SensitiveWordFilterMiddleware::new(vec!["敏感词", "违规"]);
    info!("✓ 敏感词过滤中间件创建成功\n");

    info!("3.3 创建响应格式化中间件...");
    let format_middleware = ResponseFormatMiddleware;
    info!("✓ 响应格式化中间件创建成功\n");

    info!("3.4 创建工具调用日志中间件...");
    let tool_call_logging = ToolCallLoggingMiddleware;
    info!("✓ 工具调用日志中间件创建成功\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 4: SimpleAgent + 中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: SimpleAgent + 中间件");
    info!("═══════════════════════════════════════\n");

    if has_api_config {
        info!("4.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("4.2 创建带中间件的 SimpleAgent...");
        let simple_agent = SimpleAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt("你是一个翻译助手，负责将中文翻译成英文。")
            .temperature(0.3)
            .with_middleware(LoggingMiddleware::new())
            .with_middleware(RateLimitMiddleware::new(60))
            .build();
        info!("✓ SimpleAgent 创建成功（带日志和限流中间件）\n");

        info!("4.3 测试 SimpleAgent...");
        let task = "你好";
        info!("  输入：\"{}\"", task);

        match simple_agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过 SimpleAgent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 5: ChatAgent + 中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: ChatAgent + 中间件");
    info!("═══════════════════════════════════════\n");

    if has_api_config {
        info!("5.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("5.2 创建带中间件和对话历史的 ChatAgent...");
        let chat_agent = ChatAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt("你是友好的心理咨询助手。")
            .with_conversation(true)
            .with_middleware_chain(
                MiddlewareChain::new()
                    .with(LoggingMiddleware::new())
                    .with(CacheMiddleware::new())
                    .with(auth_middleware.clone())
            )
            .build();
        info!("✓ ChatAgent 创建成功（带日志、缓存、认证中间件）\n");

        info!("5.3 测试 ChatAgent - 第一轮...");
        let task1 = "我今天心情不好";
        info!("  输入：\"{}\"", task1);

        match chat_agent.run(task1.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");

        info!("5.4 测试 ChatAgent - 第二轮（保持上下文）...");
        let task2 = "因为工作压力大";
        info!("  输入：\"{}\"", task2);

        match chat_agent.run(task2.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过 ChatAgent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 6: ToolAgent + 中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 6: ToolAgent + 中间件");
    info!("═══════════════════════════════════════\n");

    if has_api_config {
        info!("6.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("6.2 创建带中间件的 ToolAgent...");
        let tool_agent = ToolAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt("你是一个友好的助手。")
            .tool(EchoTool)
            .with_middleware(LoggingMiddleware::new())
            .with_middleware(CacheMiddleware::new())
            .with_middleware(MetricsMiddleware::new())
            .with_middleware(filter_middleware.clone())
            .with_middleware(format_middleware.clone())
            .with_middleware(tool_call_logging.clone())
            .max_steps(5)
            .build();
        info!("✓ ToolAgent 创建成功（带多个中间件，包括工具调用日志）\n");

        info!("6.3 测试 ToolAgent...");
        let task = "你好，这是一个测试。";
        info!("  输入：\"{}\"", task);

        match tool_agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                    info!("  输出值：{:?}", output.value);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");

        info!("6.4 测试敏感词过滤...");
        let task2 = "这是一个敏感词测试，包含违规内容。";
        info!("  输入：\"{}\"", task2);

        match tool_agent.run(task2.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过 ToolAgent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 7: ReActAgent + 中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 7: ReActAgent + 中间件");
    info!("═══════════════════════════════════════\n");

    if has_api_config {
        info!("7.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("7.2 创建带中间件的 ReActAgent...");
        let react_agent = ReActAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt("你是一个善于推理的助手。请先思考，再行动。")
            .tool(EchoTool)
            .with_middleware_chain(
                MiddlewareChain::new()
                    .with(LoggingMiddleware::new())
                    .with(RateLimitMiddleware::new(30))
            )
            .max_steps(15)
            .build();
        info!("✓ ReActAgent 创建成功（带日志和限流中间件）\n");

        info!("7.3 测试 ReActAgent...");
        let task = "请重复这句话：Hello, ReAct!";
        info!("  输入：\"{}\"", task);

        match react_agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过 ReActAgent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 8: ReflectAgent + 中间件
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 8: ReflectAgent + 中间件");
    info!("═══════════════════════════════════════\n");

    if has_api_config {
        info!("8.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("8.2 创建带中间件的 ReflectAgent...");
        let reflect_agent = ReflectAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt("你是一个追求卓越的助手。请不断反思和改进你的答案。")
            .with_middleware(LoggingMiddleware::new())
            .with_middleware(ResponseFormatMiddleware)
            .max_iterations(3)
            .build();
        info!("✓ ReflectAgent 创建成功（带日志和格式化中间件）\n");

        info!("8.3 测试 ReflectAgent...");
        let task = "用一句话解释什么是人工智能。";
        info!("  输入：\"{}\"", task);

        match reflect_agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("  输出：\"{}\"", text);
                    info!("  输出值：{:?}", output.value);
                }
            }
            Err(e) => {
                info!("  Agent 运行出错：{}", e);
            }
        }
        info!("");
    } else {
        info!("⚠ 跳过 ReflectAgent 测试（未设置 API Key）\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 9: 中间件执行流程说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 9: 中间件执行流程");
    info!("═══════════════════════════════════════\n");

    info!("9.1 请求处理流程:");
    info!("  用户输入 → LoggingMiddleware → RateLimitMiddleware");
    info!("           → CacheMiddleware → AuthMiddleware");
    info!("           → SensitiveWordFilter → Agent 处理\n");

    info!("9.2 响应处理流程（逆序）:");
    info!("  Agent 输出 → ResponseFormat → AuthMiddleware");
    info!("             → CacheMiddleware → RateLimitMiddleware");
    info!("             → LoggingMiddleware → 用户\n");

    info!("9.3 错误处理流程（逆序）:");
    info!("  错误 → AuthMiddleware → CacheMiddleware");
    info!("       → RateLimitMiddleware → LoggingMiddleware");
    info!("       → 返回给用户\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 中间件系统总结：\n");

    info!("1. 支持的 Agent 类型:");
    info!("   - SimpleAgent: 简单问答");
    info!("   - ChatAgent: 多轮对话");
    info!("   - ToolAgent: 工具调用");
    info!("   - ReActAgent: 推理 + 行动");
    info!("   - ReflectAgent: 反思迭代\n");

    info!("2. 中间件链:");
    info!("   - 按顺序执行所有中间件");
    info!("   - 支持请求前、响应后、错误处理钩子");
    info!("   - 内嵌到所有 Agent 执行流程中\n");

    info!("3. 内置中间件:");
    info!("   - LoggingMiddleware: 日志记录");
    info!("   - RateLimitMiddleware: 请求限流");
    info!("   - CacheMiddleware: 响应缓存");
    info!("   - MetricsMiddleware: 指标收集\n");

    info!("4. 自定义中间件:");
    info!("   - 实现 Middleware trait");
    info!("   - 实现 name()、on_request()、on_response()、on_error()");
    info!("   - 可以修改输入和输出\n");

    info!("5. 与 Agent 集成:");
    info!("   - 方式 1: with_middleware_chain() 一次性设置");
    info!("   - 方式 2: with_middleware() 逐个添加");
    info!("   - 所有 Agent 类型都支持中间件\n");

    info!("6. 使用场景:");
    info!("   - 横切关注点：日志、监控、认证");
    info!("   - 请求预处理：参数验证、数据转换");
    info!("   - 响应后处理：格式化、压缩");
    info!("   - 错误处理：统一错误格式\n");

    Ok(())
}
