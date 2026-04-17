//! Hook（钩子）优先级系统
//!
//! 本模块提供增强的 Hook 系统，支持：
//! - 优先级排序（priority: i32）
//! - Void 钩子（并行 fire-and-forget，只读观察）
//! - Modifying 钩子（按优先级顺序执行，可修改数据或取消操作）
//!
//! 参考实现: zeroclaw `HookHandler` trait

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent::{AgentError, AgentInput, AgentOutput},
    provider::types::{ChatMessage, ChatResponse},
    tool::types::ToolResult,
};

/// Hook 执行结果
#[derive(Debug, Clone)]
pub enum HookResult<T> {
    /// 继续执行，可能包含修改后的数据
    Continue(T),
    /// 取消操作，包含原因
    Cancel(String),
}

impl<T> HookResult<T> {
    /// 获取 Continue 中的值，如果是 Cancel 则返回 None
    pub fn into_option(self) -> Option<T> {
        match self {
            HookResult::Continue(v) => Some(v),
            HookResult::Cancel(_) => None,
        }
    }

    /// 检查是否是 Continue
    pub fn is_continue(&self) -> bool {
        matches!(self, HookResult::Continue(_))
    }

    /// 检查是否是 Cancel
    pub fn is_cancel(&self) -> bool {
        matches!(self, HookResult::Cancel(_))
    }

    /// 映射 Continue 值
    pub fn map<F, U>(self, f: F) -> HookResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            HookResult::Continue(v) => HookResult::Continue(f(v)),
            HookResult::Cancel(msg) => HookResult::Cancel(msg),
        }
    }
}

/// Hook 优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HookPriority(pub i32);

impl HookPriority {
    /// 最高优先级（最先执行）
    pub const HIGHEST: Self = Self(i32::MAX);
    /// 高优先级
    pub const HIGH: Self = Self(100);
    /// 默认优先级
    pub const NORMAL: Self = Self(0);
    /// 低优先级
    pub const LOW: Self = Self(-100);
    /// 最低优先级（最后执行）
    pub const LOWEST: Self = Self(i32::MIN);
}

impl Default for HookPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl From<i32> for HookPriority {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

/// Void Hook trait（只读观察型钩子）
///
/// Void 钩子是 fire-and-forget 类型的钩子，用于观察事件而不修改数据。
/// 多个 Void 钩子可以并行执行。
#[async_trait]
pub trait VoidHook: Send + Sync {
    /// 钩子名称
    fn name(&self) -> &str;

    /// 钩子优先级（数值越大优先级越高）
    fn priority(&self) -> HookPriority {
        HookPriority::NORMAL
    }

    /// 会话开始钩子
    async fn on_session_start(&self, _session_id: &str) {}

    /// 会话结束钩子
    async fn on_session_end(&self, _session_id: &str) {}

    /// LLM 输入钩子
    async fn on_llm_input(&self, _messages: &[ChatMessage], _model: &str) {}

    /// LLM 输出钩子
    async fn on_llm_output(&self, _response: &ChatResponse) {}

    /// 工具调用后钩子
    async fn on_after_tool_call(&self, _tool: &str, _result: &ToolResult, _duration_ms: u64) {}

    /// Agent 步骤完成钩子
    async fn on_step_complete(&self, _step: usize, _output: &AgentOutput) {}

    /// 错误钩子
    async fn on_error(&self, _error: &AgentError) {}
}

/// Modifying Hook trait（修改型钩子）
///
/// Modifying 钩子可以修改数据或取消操作。钩子按优先级顺序执行，
/// 每个钩子的输出作为下一个钩子的输入。
#[async_trait]
pub trait ModifyingHook: Send + Sync {
    /// 钩子名称
    fn name(&self) -> &str;

    /// 钩子优先级（数值越大优先级越高）
    fn priority(&self) -> HookPriority {
        HookPriority::NORMAL
    }

    /// 模型解析前钩子
    ///
    /// 可以修改 provider 和 model 名称
    async fn before_model_resolve(
        &self,
        provider: String,
        model: String,
    ) -> HookResult<(String, String)> {
        HookResult::Continue((provider, model))
    }

    /// Prompt 构建前钩子
    ///
    /// 可以修改系统 prompt
    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        HookResult::Continue(prompt)
    }

    /// LLM 调用前钩子
    ///
    /// 可以修改消息列表和模型参数
    async fn before_llm_call(
        &self,
        messages: Vec<ChatMessage>,
        model: String,
    ) -> HookResult<(Vec<ChatMessage>, String)> {
        HookResult::Continue((messages, model))
    }

    /// 工具调用前钩子
    ///
    /// 可以修改工具名称和参数
    async fn before_tool_call(
        &self,
        name: String,
        args: Value,
    ) -> HookResult<(String, Value)> {
        HookResult::Continue((name, args))
    }

    /// Agent 输入处理钩子
    ///
    /// 可以修改用户输入
    async fn on_input_received(&self, input: AgentInput) -> HookResult<AgentInput> {
        HookResult::Continue(input)
    }

    /// Agent 输出生成钩子
    ///
    /// 可以修改输出
    async fn on_output_generated(&self, output: AgentOutput) -> HookResult<AgentOutput> {
        HookResult::Continue(output)
    }
}

/// 钩子注册表
#[derive(Default)]
pub struct HookRegistry {
    void_hooks: Vec<Arc<dyn VoidHook>>,
    modifying_hooks: Vec<Arc<dyn ModifyingHook>>,
}

impl HookRegistry {
    /// 创建新的钩子注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 Void 钩子
    pub fn register_void(&mut self, hook: Arc<dyn VoidHook>) {
        self.void_hooks.push(hook);
        // 按优先级排序（高优先级在前）
        self.void_hooks
            .sort_by_key(|h| std::cmp::Reverse(h.priority()));
    }

    /// 注册 Modifying 钩子
    pub fn register_modifying(&mut self, hook: Arc<dyn ModifyingHook>) {
        self.modifying_hooks.push(hook);
        // 按优先级排序（高优先级在前）
        self.modifying_hooks
            .sort_by_key(|h| std::cmp::Reverse(h.priority()));
    }

    /// 执行 Void 钩子（并行）
    pub async fn run_void<F, Fut>(&self, f: F)
    where
        F: Fn(&dyn VoidHook) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ()> + Send,
    {
        use futures_util::future::join_all;

        let futures: Vec<_> = self.void_hooks.iter().map(|hook| f(&**hook)).collect();
        join_all(futures).await;
    }

    /// 执行 Modifying 钩子（顺序，高优先级先执行）
    pub async fn run_modifying<T, F, Fut>(
        &self,
        initial: T,
        f: F,
    ) -> HookResult<T>
    where
        T: Clone,
        F: Fn(&dyn ModifyingHook, T) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = HookResult<T>> + Send,
    {
        let mut current = initial;

        for hook in &self.modifying_hooks {
            match f(&**hook, current.clone()).await {
                HookResult::Continue(v) => current = v,
                HookResult::Cancel(msg) => return HookResult::Cancel(msg),
            }
        }

        HookResult::Continue(current)
    }

    /// 获取 Void 钩子数量
    pub fn void_hook_count(&self) -> usize {
        self.void_hooks.len()
    }

    /// 获取 Modifying 钩子数量
    pub fn modifying_hook_count(&self) -> usize {
        self.modifying_hooks.len()
    }

    /// 清空所有钩子
    pub fn clear(&mut self) {
        self.void_hooks.clear();
        self.modifying_hooks.clear();
    }
}

/// 组合钩子（同时实现 VoidHook 和 ModifyingHook）
pub struct CombinedHook {
    void_hooks: Vec<Arc<dyn VoidHook>>,
    modifying_hooks: Vec<Arc<dyn ModifyingHook>>,
}

impl CombinedHook {
    /// 创建新的组合钩子
    pub fn new() -> Self {
        Self {
            void_hooks: Vec::new(),
            modifying_hooks: Vec::new(),
        }
    }

    /// 添加 Void 钩子
    pub fn add_void(mut self, hook: Arc<dyn VoidHook>) -> Self {
        self.void_hooks.push(hook);
        self
    }

    /// 添加 Modifying 钩子
    pub fn add_modifying(mut self, hook: Arc<dyn ModifyingHook>) -> Self {
        self.modifying_hooks.push(hook);
        self
    }

    /// 构建钩子注册表
    pub fn build(self) -> HookRegistry {
        let mut registry = HookRegistry::new();
        for hook in self.void_hooks {
            registry.register_void(hook);
        }
        for hook in self.modifying_hooks {
            registry.register_modifying(hook);
        }
        registry
    }
}

/// 日志 Void Hook（示例实现）
pub struct LoggingVoidHook {
    name: String,
    priority: HookPriority,
}

impl LoggingVoidHook {
    /// 创建新的日志钩子
    pub fn new() -> Self {
        Self {
            name: "logging".to_string(),
            priority: HookPriority::NORMAL,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: HookPriority) -> Self {
        self.priority = priority;
        self
    }
}

impl Default for LoggingVoidHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VoidHook for LoggingVoidHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> HookPriority {
        self.priority
    }

    async fn on_session_start(&self, session_id: &str) {
        tracing::info!(session_id, "hook.session.start");
    }

    async fn on_session_end(&self, session_id: &str) {
        tracing::info!(session_id, "hook.session.end");
    }

    async fn on_llm_input(&self, messages: &[ChatMessage], model: &str) {
        tracing::debug!(
            message_count = messages.len(),
            model,
            "hook.llm.input"
        );
    }

    async fn on_llm_output(&self, response: &ChatResponse) {
        tracing::debug!(
            content_len = response.message.content.len(),
            "hook.llm.output"
        );
    }

    async fn on_after_tool_call(&self, tool: &str, _result: &ToolResult, duration_ms: u64) {
        tracing::info!(
            tool_name = tool,
            duration_ms,
            "hook.tool_call.complete"
        );
    }

    async fn on_error(&self, error: &AgentError) {
        tracing::error!(error = %error, "hook.error");
    }
}

/// 验证 Modifying Hook（示例实现）
pub struct ValidationModifyingHook {
    name: String,
    priority: HookPriority,
    max_prompt_length: usize,
}

impl ValidationModifyingHook {
    /// 创建新的验证钩子
    pub fn new() -> Self {
        Self {
            name: "validation".to_string(),
            priority: HookPriority::HIGH, // 高优先级，尽早执行
            max_prompt_length: 10000,
        }
    }

    /// 设置最大 prompt 长度
    pub fn with_max_prompt_length(mut self, max: usize) -> Self {
        self.max_prompt_length = max;
        self
    }
}

impl Default for ValidationModifyingHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModifyingHook for ValidationModifyingHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> HookPriority {
        self.priority
    }

    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        if prompt.len() > self.max_prompt_length {
            return HookResult::Cancel(format!(
                "Prompt 长度 {} 超过最大限制 {}",
                prompt.len(),
                self.max_prompt_length
            ));
        }
        HookResult::Continue(prompt)
    }

    async fn before_tool_call(
        &self,
        name: String,
        args: Value,
    ) -> HookResult<(String, Value)> {
        // 示例：禁止调用某些危险工具
        let forbidden_tools = ["rm", "del", "delete"];
        if forbidden_tools.contains(&name.as_str()) {
            return HookResult::Cancel(format!("工具 '{}' 被禁止调用", name));
        }
        HookResult::Continue((name, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_priority() {
        assert!(HookPriority::HIGHEST > HookPriority::HIGH);
        assert!(HookPriority::HIGH > HookPriority::NORMAL);
        assert!(HookPriority::NORMAL > HookPriority::LOW);
        assert!(HookPriority::LOW > HookPriority::LOWEST);
    }

    #[test]
    fn test_hook_result() {
        let result: HookResult<i32> = HookResult::Continue(42);
        assert!(result.is_continue());
        assert!(!result.is_cancel());
        assert_eq!(result.into_option(), Some(42));

        let result: HookResult<i32> = HookResult::Cancel("error".to_string());
        assert!(!result.is_continue());
        assert!(result.is_cancel());
        assert_eq!(result.into_option(), None);
    }

    #[test]
    fn test_hook_result_map() {
        let result: HookResult<i32> = HookResult::Continue(21);
        let mapped = result.map(|x| x * 2);
        assert!(matches!(mapped, HookResult::Continue(42)));

        let result: HookResult<i32> = HookResult::Cancel("error".to_string());
        let mapped = result.map(|x| x * 2);
        assert!(matches!(mapped, HookResult::Cancel(_)));
    }

    #[tokio::test]
    async fn test_hook_registry_void() {
        let mut registry = HookRegistry::new();
        registry.register_void(Arc::new(LoggingVoidHook::new()));

        assert_eq!(registry.void_hook_count(), 1);

        // 使用 AtomicBool 来在闭包中共享状态
        let executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed_clone = executed.clone();
        registry
            .run_void(move |_hook| {
                executed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                async {}
            })
            .await;

        assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_registry_modifying() {
        let mut registry = HookRegistry::new();
        registry.register_modifying(Arc::new(ValidationModifyingHook::new()));

        // 使用一个静态的修改函数来避免生命周期问题
        async fn modify_string(s: String) -> HookResult<String> {
            HookResult::Continue(s + " modified")
        }

        let result = registry
            .run_modifying("test".to_string(), |_hook, s| modify_string(s))
            .await;

        assert!(matches!(result, HookResult::Continue(s) if s == "test modified"));
    }

    #[tokio::test]
    async fn test_hook_registry_cancel() {
        struct CancelHook;

        #[async_trait]
        impl ModifyingHook for CancelHook {
            fn name(&self) -> &str {
                "cancel"
            }

            async fn before_prompt_build(&self, _prompt: String) -> HookResult<String> {
                HookResult::Cancel("test cancel".to_string())
            }
        }

        // 直接测试 CancelHook 的行为，不通过 registry
        let hook = CancelHook;
        let result = hook.before_prompt_build("test".to_string()).await;

        assert!(matches!(result, HookResult::Cancel(msg) if msg == "test cancel"));

        // 测试 registry 可以正常注册
        let mut registry = HookRegistry::new();
        registry.register_modifying(Arc::new(CancelHook));
        assert_eq!(registry.modifying_hook_count(), 1);
    }
}
