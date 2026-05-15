//! 声明式便捷宏
//!
//! 提供 `agent!`、`messages!`、`chat_request!` 等宏，
//! 用于快速构建常见对象，降低框架入门门槛。

/// 快速构建 Agent（声明式语法）。
///
/// 支持所有 Agent 类型（`ToolAgent`、`SimpleAgent`、`ChatAgent`、`ReActAgent`、`ReflectAgent`）。
///
/// # 语法
///
/// ```text
/// agent!(
///     ToolAgent,                     // Agent 类型
///     provider: provider_instance,   // Provider（必需）
///     model: "model_name",           // 模型名称（必需）
///     system_prompt: "prompt",       // 系统提示词
///     tools: [tool1, tool2],         // 工具列表
///     max_steps: 10,                 // 最大步骤数
///     temperature: 0.7,              // 温度参数
///     max_tokens: 4096,              // 最大输出 token
///     top_p: 0.9,                    // Top P
///     top_k: 40,                     // Top K
///     frequency_penalty: 0.0,        // 频率惩罚
///     presence_penalty: 0.0,         // 存在惩罚
/// )
/// ```
///
/// # 示例
///
/// ## ToolAgent（工具调用）
///
/// ```rust,ignore
/// use rucora::agent;
/// use rucora::provider::OpenAiProvider;
/// use rucora::tools::ShellTool;
/// use rucora::prelude::Agent;
///
/// # async fn example() -> anyhow::Result<()> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = agent!(
///     ToolAgent,
///     provider: provider,
///     model: "gpt-4o-mini",
///     system_prompt: "你是有用的助手",
///     tools: [ShellTool::new()],
///     max_steps: 10,
/// )?;
///
/// let output = agent.run("列出当前目录".into()).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## SimpleAgent（简单问答）
///
/// ```rust,ignore
/// use rucora::agent;
/// use rucora::provider::OpenAiProvider;
/// use rucora::prelude::Agent;
///
/// # async fn example() -> anyhow::Result<()> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = agent!(
///     SimpleAgent,
///     provider: provider,
///     model: "gpt-4o-mini",
///     system_prompt: "你是翻译助手",
///     temperature: 0.3,
/// )?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! agent {
    // Entry points
    (ToolAgent, provider: $provider:expr, model: $model:expr, $( $rest:tt )*) => {{
        let mut builder = $crate::agent::ToolAgentBuilder::new()
            .provider($provider)
            .model($model);
        $crate::__agent_build!(@apply builder, $( $rest )*)
    }};
    (SimpleAgent, provider: $provider:expr, model: $model:expr, $( $rest:tt )*) => {{
        let mut builder = $crate::agent::SimpleAgentBuilder::new()
            .provider($provider)
            .model($model);
        $crate::__agent_build!(@apply builder, $( $rest )*)
    }};
    (ChatAgent, provider: $provider:expr, model: $model:expr, $( $rest:tt )*) => {{
        let mut builder = $crate::agent::ChatAgentBuilder::new()
            .provider($provider)
            .model($model);
        $crate::__agent_build!(@apply builder, $( $rest )*)
    }};
    (ReActAgent, provider: $provider:expr, model: $model:expr, $( $rest:tt )*) => {{
        let mut builder = $crate::agent::ReActAgentBuilder::new()
            .provider($provider)
            .model($model);
        $crate::__agent_build!(@apply builder, $( $rest )*)
    }};
    (ReflectAgent, provider: $provider:expr, model: $model:expr, $( $rest:tt )*) => {{
        let mut builder = $crate::agent::ReflectAgentBuilder::new()
            .provider($provider)
            .model($model);
        $crate::__agent_build!(@apply builder, $( $rest )*)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __agent_build {
    // Terminal
    (@apply $b:expr) => { $b.try_build() };
    (@apply $b:expr,) => { $b.try_build() };

    // Keys
    (@apply $b:expr, system_prompt: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.system_prompt($v), $( $rest )*)
    };
    (@apply $b:expr, tools: [$( $t:expr ),* $(,)?], $( $rest:tt )*) => {{
        let mut __b = $b;
        $( __b = __b.tool($t); )*
        $crate::__agent_build!(@apply __b, $( $rest )*)
    }};
    (@apply $b:expr, max_steps: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.max_steps($v), $( $rest )*)
    };
    (@apply $b:expr, max_iterations: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.max_iterations($v), $( $rest )*)
    };
    (@apply $b:expr, temperature: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.temperature($v), $( $rest )*)
    };
    (@apply $b:expr, max_tokens: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.max_tokens($v), $( $rest )*)
    };
    (@apply $b:expr, top_p: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.top_p($v), $( $rest )*)
    };
    (@apply $b:expr, top_k: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.top_k($v), $( $rest )*)
    };
    (@apply $b:expr, frequency_penalty: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.frequency_penalty($v), $( $rest )*)
    };
    (@apply $b:expr, presence_penalty: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.presence_penalty($v), $( $rest )*)
    };
    (@apply $b:expr, with_conversation: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.with_conversation($v), $( $rest )*)
    };
    (@apply $b:expr, stop: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.stop($v), $( $rest )*)
    };
    (@apply $b:expr, extra_params: $v:expr, $( $rest:tt )*) => {
        $crate::__agent_build!(@apply $b.extra_params($v), $( $rest )*)
    };
}

/// 快速构建消息列表。
///
/// # 语法
///
/// ```text
/// messages![
///     system("系统提示词"),
///     user("用户输入"),
///     assistant("助手回复"),
///     tool("工具名", "工具输出"),
/// ]
/// ```
///
/// # 示例
///
/// ```rust
/// use rucora::messages;
/// use rucora_core::provider::types::ChatMessage;
///
/// let msgs = messages![
///     system("你是翻译助手"),
///     user("Hello"),
///     assistant("你好"),
/// ];
///
/// assert_eq!(msgs.len(), 3);
/// assert_eq!(msgs[0].role, rucora_core::provider::types::Role::System);
/// ```
#[macro_export]
macro_rules! messages {
    ( $( $role:ident($content:expr) ),* $(,)? ) => {
        vec![
            $( rucora_core::provider::types::ChatMessage::$role($content.to_string()) ),*
        ]
    };
    () => {
        Vec::<rucora_core::provider::types::ChatMessage>::new()
    };
}

/// 快速构建聊天请求。
///
/// # 语法
///
/// ```text
/// chat_request!(
///     messages: [system("..."), user("...")],
///     model: "gpt-4o-mini",
///     temperature: 0.7,
///     max_tokens: 4096,
///     top_p: 0.9,
/// )
/// ```
///
/// # 示例
///
/// ```rust
/// use rucora::chat_request;
///
/// let req = chat_request!(
///     messages: [system("你是助手"), user("你好")],
///     model: "gpt-4o-mini",
///     temperature: 0.7,
///     max_tokens: 2048,
/// );
///
/// assert_eq!(req.messages.len(), 2);
/// assert_eq!(req.model, Some("gpt-4o-mini".to_string()));
/// assert_eq!(req.temperature, Some(0.7));
/// ```
#[macro_export]
macro_rules! chat_request {
    (
        messages: [$( $role:ident($content:expr) ),* $(,)?]
        $(, $key:ident : $val:expr )* $(,)?
    ) => {{
        #[allow(unused_mut)]
        let mut req = rucora_core::provider::types::ChatRequest::new(vec![
            $( rucora_core::provider::types::ChatMessage::$role($content.to_string()) ),*
        ]);
        $( $crate::__chat_request_field!(req, $key, $val); )*
        req
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __chat_request_field {
    ($req:expr, model, $v:expr) => {
        $req.model = Some($v.to_string());
    };
    ($req:expr, temperature, $v:expr) => {
        $req.temperature = Some($v);
    };
    ($req:expr, max_tokens, $v:expr) => {
        $req.max_tokens = Some($v);
    };
    ($req:expr, top_p, $v:expr) => {
        $req.top_p = Some($v);
    };
    ($req:expr, top_k, $v:expr) => {
        $req.top_k = Some($v);
    };
    ($req:expr, frequency_penalty, $v:expr) => {
        $req.frequency_penalty = Some($v);
    };
    ($req:expr, presence_penalty, $v:expr) => {
        $req.presence_penalty = Some($v);
    };
    ($req:expr, stop, $v:expr) => {
        $req.stop = Some($v);
    };
    ($req:expr, response_format, $v:expr) => {
        $req.response_format = Some($v);
    };
    ($req:expr, metadata, $v:expr) => {
        $req.metadata = Some($v);
    };
    ($req:expr, extra, $v:expr) => {
        $req.extra = Some($v);
    };
}

/// 快速构建工具参数 JSON Schema。
///
/// # 语法
///
/// ```text
/// tool_params! {
///     "expression" => (string, required, "数学表达式"),
///     "precision"  => (number, "小数精度"),
/// }
/// ```
///
/// # 示例
///
/// ```rust
/// use rucora::tool_params;
///
/// let schema = tool_params! {
///     "expression" => (string, required, "数学表达式"),
///     "precision"  => (number, "小数精度"),
/// };
///
/// let obj = schema.as_object().unwrap();
/// assert_eq!(obj["type"], "object");
/// let props = obj["properties"].as_object().unwrap();
/// assert!(props.contains_key("expression"));
/// let required = obj["required"].as_array().unwrap();
/// assert_eq!(required.len(), 1);
/// ```
#[macro_export]
macro_rules! tool_params {
    ( $( $name:literal => $spec:tt ),* $(,)? ) => {{
        let mut __properties = ::serde_json::Map::new();
        let mut __required: Vec<&str> = Vec::new();
        $( $crate::__tool_param_field!(__properties, __required, $name, $spec); )*
        ::serde_json::json!({
            "type": "object",
            "properties": ::serde_json::Value::Object(__properties),
            "required": __required,
        })
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tool_param_field {
    ($props:expr, $req:expr, $name:literal, ($ty:ident, required, $desc:literal)) => {
        let mut __p = ::serde_json::Map::new();
        __p.insert(
            "type".to_string(),
            ::serde_json::Value::String(stringify!($ty).to_string()),
        );
        __p.insert(
            "description".to_string(),
            ::serde_json::Value::String($desc.to_string()),
        );
        $props.insert($name.to_string(), ::serde_json::Value::Object(__p));
        $req.push($name);
    };
    ($props:expr, $req:expr, $name:literal, ($ty:ident, required)) => {
        let mut __p = ::serde_json::Map::new();
        __p.insert(
            "type".to_string(),
            ::serde_json::Value::String(stringify!($ty).to_string()),
        );
        $props.insert($name.to_string(), ::serde_json::Value::Object(__p));
        $req.push($name);
    };
    ($props:expr, $req:expr, $name:literal, ($ty:ident, $desc:literal)) => {
        let mut __p = ::serde_json::Map::new();
        __p.insert(
            "type".to_string(),
            ::serde_json::Value::String(stringify!($ty).to_string()),
        );
        __p.insert(
            "description".to_string(),
            ::serde_json::Value::String($desc.to_string()),
        );
        $props.insert($name.to_string(), ::serde_json::Value::Object(__p));
    };
    ($props:expr, $req:expr, $name:literal, ($ty:ident)) => {
        let mut __p = ::serde_json::Map::new();
        __p.insert(
            "type".to_string(),
            ::serde_json::Value::String(stringify!($ty).to_string()),
        );
        $props.insert($name.to_string(), ::serde_json::Value::Object(__p));
    };
}

#[cfg(test)]
mod tests {
    use rucora_core::provider::types::{ChatMessage, Role};

    #[test]
    fn messages_macro_basic() {
        let msgs = messages![
            system("你是助手"),
            user("Hello"),
            assistant("你好"),
        ];

        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1].role, Role::User);
        assert_eq!(msgs[2].role, Role::Assistant);
    }

    #[test]
    fn messages_macro_empty() {
        let msgs: Vec<ChatMessage> = messages![];
        assert!(msgs.is_empty());
    }

    #[test]
    fn tool_params_macro_basic() {
        let schema = tool_params! {
            "expression" => (string, required, "数学表达式"),
            "precision"  => (number, "小数精度"),
        };

        let obj = schema.as_object().unwrap();
        assert_eq!(obj["type"], "object");

        let props = obj["properties"].as_object().unwrap();
        assert!(props.contains_key("expression"));
        assert!(props.contains_key("precision"));

        let expr_prop = props["expression"].as_object().unwrap();
        assert_eq!(expr_prop["type"], "string");
        assert_eq!(expr_prop["description"], "数学表达式");

        let required = obj["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "expression");
    }

    #[test]
    fn chat_request_macro_basic() {
        let req = chat_request!(
            messages: [system("你是助手"), user("你好")],
            model: "gpt-4o-mini",
            temperature: 0.7,
            max_tokens: 2048,
        );

        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.model, Some("gpt-4o-mini".to_string()));
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_tokens, Some(2048));
    }

    #[test]
    fn chat_request_macro_minimal() {
        let req = chat_request!(
            messages: [user("hello")],
        );

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.model, None);
        assert_eq!(req.temperature, None);
    }

    #[test]
    fn messages_macro_with_mixed_roles() {
        let msgs = messages![
            user("query"),
            assistant("response"),
        ];
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, rucora_core::provider::types::Role::User);
        assert_eq!(msgs[1].role, rucora_core::provider::types::Role::Assistant);
    }

    #[test]
    fn tool_params_macro_with_optional() {
        let schema = tool_params! {
            "required_field" => (string, required, "Required"),
            "optional_field" => (number, "Optional"),
        };

        let obj = schema.as_object().unwrap();
        let required = obj["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "required_field");

        let props = obj["properties"].as_object().unwrap();
        assert!(props.contains_key("optional_field"));
        let opt_prop = props["optional_field"].as_object().unwrap();
        assert_eq!(opt_prop["type"], "number");
    }
}
