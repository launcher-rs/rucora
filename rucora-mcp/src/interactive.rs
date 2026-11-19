//! MCP 交互式客户端支持（Elicitation / Sampling）
//!
//! # 概述
//!
//! 提供对 MCP 客户端侧两种"服务器 → 客户端"反向请求的支持：
//!
//! - **Elicitation**：服务器请求客户端向最终用户收集输入
//!   （表单 elicitation 或 URL elicitation）
//! - **Sampling**：服务器请求客户端的 LLM 生成一段消息（"服务器借客户端的脑子一用"）
//!
//! 两者均为可选能力：未注册处理器时，elicitation 默认拒绝（Decline），
//! sampling 默认返回 method_not_found——与 rmcp 默认行为一致。
//!
//! # 使用示例
//!
//! ## Elicitation：服务器请求用户输入
//!
//! ```rust,no_run
//! use rmcp::model::{ElicitRequestParams, ElicitResult, ElicitationAction};
//! use rucora_mcp::McpClientBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = McpClientBuilder::new()
//!     .with_elicitation(|request| async move {
//!         match request {
//!             ElicitRequestParams::FormElicitationParams { message, .. } => {
//!                 // 实际场景：展示 UI / 读取控制台输入
//!                 println!("服务器请求输入：{message}");
//!                 Ok(ElicitResult::new(ElicitationAction::Accept)
//!                     .with_content(serde_json::json!({"confirmed": true})))
//!             }
//!             _ => Ok(ElicitResult::new(ElicitationAction::Decline)),
//!         }
//!     })
//!     .connect_streamable_http("http://127.0.0.1:8000/mcp")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Sampling：将 rucora 的 LLM Provider 接入服务器采样请求
//!
//! （该示例引用 `rucora_providers`，仅适用于已引入 provider 的上层应用）
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use rucora_mcp::McpClientBuilder;
//! use rucora_providers::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = Arc::new(OpenAiProvider::from_env()?);
//! let client = McpClientBuilder::new()
//!     .with_sampling_provider(provider, "gpt-5-mini")
//!     .connect_streamable_http("http://127.0.0.1:8000/mcp")
//!     .await?;
//! # Ok(())
//! # }
//! ```

// Sampling 相关类型在 rmcp 3.11 中被 SEP-2577 标记为弃用（未来由新方案替代），
// 但 sampling/createMessage 当前仍是唯一的采样请求实现路径，保持支持并静默弃用警告。
#![allow(deprecated)]

use std::sync::Arc;

use futures_util::future::BoxFuture;
use rmcp::{
    ServiceExt,
    model::{
        ClientCapabilities, ClientConfig, CreateMessageRequestParams, CreateMessageResult,
        CreateMessageRequestMethod, ElicitRequestParams, ElicitResult, ElicitationAction,
        ElicitationCapability, Implementation, SamplingCapability, SamplingMessage,
        SamplingMessageContentBlock, Role,
    },
    service::{RequestContext, RoleClient as RmcpRoleClient},
};
use rucora_core::{
    error::ProviderError,
    provider::{LlmProvider, types::ChatMessage},
};
use tracing::debug;

use crate::tool::McpClient;

/// Elicitation 处理器：服务器请求用户输入时回调。
pub type ElicitationHandler = Arc<
    dyn Fn(ElicitRequestParams) -> BoxFuture<'static, Result<ElicitResult, rmcp::ErrorData>>
        + Send
        + Sync,
>;

/// Sampling 处理器：服务器请求 LLM 生成消息时回调。
pub type SamplingHandler = Arc<
    dyn Fn(CreateMessageRequestParams) -> BoxFuture<'static, Result<CreateMessageResult, rmcp::ErrorData>>
        + Send
        + Sync,
>;

/// 交互式 MCP 客户端处理器。
///
/// 实现 `rmcp::ClientHandler`，将 elicitation / sampling 反向请求委托给
/// 注册的处理器；未注册时保持 rmcp 默认行为（elicitation 拒绝、sampling
/// 返回 method_not_found）。
#[derive(Clone)]
pub struct InteractiveClientHandler {
    client_name: String,
    client_version: String,
    elicitation: Option<ElicitationHandler>,
    sampling: Option<SamplingHandler>,
}

impl rmcp::ClientHandler for InteractiveClientHandler {
    /// 返回客户端实现信息与能力声明。
    ///
    /// 仅在注册了对应处理器时声明 elicitation / sampling 能力，
    /// 避免服务器发送客户端无法处理的请求。
    fn get_info(&self) -> ClientConfig {
        let mut capabilities = ClientCapabilities::default();
        if self.elicitation.is_some() {
            capabilities.elicitation = Some(ElicitationCapability::default());
        }
        if self.sampling.is_some() {
            capabilities.sampling = Some(SamplingCapability::default());
        }
        ClientConfig::new(
            capabilities,
            Implementation::new(self.client_name.clone(), self.client_version.clone()),
        )
    }

    async fn create_elicitation(
        &self,
        request: ElicitRequestParams,
        _context: RequestContext<RmcpRoleClient>,
    ) -> Result<ElicitResult, rmcp::ErrorData> {
        match &self.elicitation {
            Some(handler) => handler(request).await,
            None => {
                debug!("mcp.elicitation.declined（未注册处理器，默认拒绝）");
                Ok(ElicitResult::new(ElicitationAction::Decline))
            }
        }
    }

    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RmcpRoleClient>,
    ) -> Result<CreateMessageResult, rmcp::ErrorData> {
        match &self.sampling {
            Some(handler) => handler(params).await,
            None => Err(rmcp::ErrorData::method_not_found::<CreateMessageRequestMethod>()),
        }
    }
}

/// 将 rucora 的 [`LlmProvider`] 适配为 MCP Sampling 处理器。
///
/// Sampling 请求中的 `SamplingMessage` 会被转换为 [`ChatMessage`]：
/// - 文本内容直接映射
/// - 工具调用/结果等非文本内容暂不支持（跳过并记录警告）
///
/// 返回结果映射：
/// - `FinishReason::Stop` → `endTurn`
/// - `FinishReason::Length` → `maxTokens`
/// - 其余 → `endTurn`
pub fn sampling_fn_from_provider<P>(provider: Arc<P>, model: impl Into<String>) -> SamplingHandler
where
    P: LlmProvider + Send + Sync + 'static,
{
    let model = model.into();
    Arc::new(move |params: CreateMessageRequestParams| {
        let provider = provider.clone();
        let model = model.clone();
        Box::pin(async move {
            let mut messages: Vec<ChatMessage> = Vec::new();
            if let Some(system) = &params.system_prompt {
                messages.push(ChatMessage::system(system.clone()));
            }
            for m in &params.messages {
                // 文本内容直接映射；工具调用/图像等非文本内容暂不支持，跳过
                let text = m
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        SamplingMessageContentBlock::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                match m.role {
                    Role::User => messages.push(ChatMessage::user(text)),
                    _ => messages.push(ChatMessage::assistant(text)),
                }
            }

            let mut request = rucora_core::provider::types::ChatRequest::new(messages);
            request.model = Some(model.clone());
            request.max_tokens = Some(params.max_tokens);
            request.temperature = params.temperature;

            let response = provider
                .chat(request)
                .await
                .map_err(|e| match e {
                    ProviderError::Message(msg) => rmcp::ErrorData::internal_error(msg, None),
                    other => rmcp::ErrorData::internal_error(other.to_string(), None),
                })?;

            debug!(
                model = %model,
                text_len = response.text().len(),
                "mcp.sampling.completed"
            );

            let stop_reason = match &response.finish_reason {
                Some(rucora_core::provider::types::FinishReason::Length) => {
                    CreateMessageResult::STOP_REASON_END_MAX_TOKEN
                }
                _ => CreateMessageResult::STOP_REASON_END_TURN,
            };

            Ok(CreateMessageResult::new(
                SamplingMessage::assistant_text(response.text()),
                model,
            )
            .with_stop_reason(stop_reason))
        })
    })
}

/// MCP 客户端构建器。
///
/// 支持注册 Elicitation / Sampling 处理器，并通过 Streamable HTTP 或
/// stdio 子进程连接 MCP 服务器。
///
/// # 示例
///
/// ```rust,no_run
/// use rmcp::model::{ElicitResult, ElicitationAction};
/// use rucora_mcp::McpClientBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = McpClientBuilder::new()
///     .with_client_info("my-agent", "1.0.0")
///     .with_elicitation(|_request| async move {
///         Ok(ElicitResult::new(ElicitationAction::Decline))
///     })
///     .connect_streamable_http("http://127.0.0.1:8000/mcp")
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct McpClientBuilder {
    client_name: String,
    client_version: String,
    elicitation: Option<ElicitationHandler>,
    sampling: Option<SamplingHandler>,
}

impl Default for McpClientBuilder {
    fn default() -> Self {
        Self {
            client_name: "rucora".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            elicitation: None,
            sampling: None,
        }
    }
}

impl McpClientBuilder {
    /// 创建构建器（默认客户端实现信息为 `rucora/<crate 版本>`）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置客户端实现信息（发送给 MCP 服务器的名称与版本）。
    pub fn with_client_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.client_name = name.into();
        self.client_version = version.into();
        self
    }

    /// 注册 Elicitation 处理器。
    ///
    /// 注册后会向服务器声明 elicitation 能力，服务器即可发起
    /// 表单 / URL elicitation 请求。
    pub fn with_elicitation<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(ElicitRequestParams) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ElicitResult, rmcp::ErrorData>> + Send + 'static,
    {
        self.elicitation = Some(Arc::new(move |params| Box::pin(f(params))));
        self
    }

    /// 注册 Sampling 处理器。
    ///
    /// 注册后会向服务器声明 sampling 能力，服务器即可请求客户端的
    /// LLM 生成消息。
    pub fn with_sampling<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(CreateMessageRequestParams) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<CreateMessageResult, rmcp::ErrorData>>
            + Send
            + 'static,
    {
        self.sampling = Some(Arc::new(move |params| Box::pin(f(params))));
        self
    }

    /// 快捷方式：用 rucora 的 [`LlmProvider`] 处理 Sampling 请求。
    ///
    /// 等价于 `with_sampling(sampling_fn_from_provider(provider, model))`。
    pub fn with_sampling_provider<P>(self, provider: Arc<P>, model: impl Into<String>) -> Self
    where
        P: LlmProvider + Send + Sync + 'static,
    {
        let handler = sampling_fn_from_provider(provider, model);
        self.with_sampling(move |params| handler(params))
    }

    fn build_handler(self) -> InteractiveClientHandler {
        InteractiveClientHandler {
            client_name: self.client_name,
            client_version: self.client_version,
            elicitation: self.elicitation,
            sampling: self.sampling,
        }
    }

    /// 通过 Streamable HTTP 连接 MCP 服务器。
    ///
    /// # Errors
    ///
    /// 连接或初始化握手失败时返回 `rmcp::ErrorData`。
    pub async fn connect_streamable_http(
        self,
        url: impl Into<String>,
    ) -> Result<McpClient<InteractiveClientHandler>, rmcp::ErrorData> {
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

        let url = url.into();
        let config = StreamableHttpClientTransportConfig::with_uri(url.clone());
        let transport = rmcp::transport::StreamableHttpClientTransport::from_config(config);
        debug!(url = %url, "mcp.interactive.connect_streamable_http.start");

        let handler = self.build_handler();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(format!("MCP 连接失败：{e}"), None))?;
        debug!(url = %url, "mcp.interactive.connect_streamable_http.connected");
        Ok(McpClient::new(service))
    }

    /// 通过 stdio 子进程连接本地 MCP 服务器。
    ///
    /// # Errors
    ///
    /// 进程启动失败或初始化握手失败时返回 `rmcp::ErrorData`。
    pub async fn connect_stdio(
        self,
        program: impl Into<String>,
        args: &[String],
    ) -> Result<McpClient<InteractiveClientHandler>, rmcp::ErrorData> {
        let program = program.into();
        let mut command = tokio::process::Command::new(&program);
        command.args(args);
        debug!(program = %program, "mcp.interactive.connect_stdio.start");

        let transport = rmcp::transport::TokioChildProcess::new(command)
            .map_err(|e| rmcp::ErrorData::internal_error(format!("启动 MCP 进程失败：{e}"), None))?;

        let handler = self.build_handler();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(format!("MCP 初始化失败：{e}"), None))?;
        debug!(program = %program, "mcp.interactive.connect_stdio.connected");
        Ok(McpClient::new(service))
    }
}
