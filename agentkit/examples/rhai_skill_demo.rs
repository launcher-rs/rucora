use agentkit_core::provider::{types::ChatRequest, LlmProvider};
use agentkit_runtime::ToolRegistry;
use rhai::{Dynamic, Engine};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 本示例：执行 skills/ai_news/SKILL.rhai。
    // 注意：ai_news 脚本参考 blockcell，依赖 call_tool("browse", ...) 等工具。
    // agentkit 当前没有 CDP，因此这里提供一个最小 browse 兼容实现（内部用 web_fetch 抓 HTML）。
    let tools_for_rhai = ToolRegistry::new()
        .register(agentkit::tools::BrowseTool::new())
        .register(agentkit::tools::WebFetchTool::new())
        .register(agentkit::tools::HttpRequestTool::new());

    let registrar: agentkit::skills::RhaiEngineRegistrar = Arc::new(move |engine: &mut Engine| {
        let tools_for_rhai = tools_for_rhai.clone();

        // is_error(x): bool
        // 约定：tool 返回 {success: bool, error?: string} 时，success=false 或存在 error 即认为失败。
        engine.register_fn("is_error", |x: Dynamic| -> bool {
            if x.is::<bool>() {
                return !x.cast::<bool>();
            }
            if x.is_map() {
                let m = x.clone().cast::<rhai::Map>();
                if let Some(v) = m.get("success") {
                    if v.is::<bool>() && !v.clone().cast::<bool>() {
                        return true;
                    }
                }
                if m.contains_key("error") {
                    return true;
                }
            }
            false
        });

        // is_map(x): bool
        // blockcell 风格脚本里常用：resp.is_map()
        engine.register_fn("is_map", |x: Dynamic| -> bool { x.is_map() });

        // arr_join(arr, delim): string
        engine.register_fn("arr_join", |arr: rhai::Array, delim: &str| -> String {
            let mut parts: Vec<String> = Vec::with_capacity(arr.len());
            for v in arr {
                parts.push(v.to_string());
            }
            parts.join(delim)
        });

        // call_tool(tool_name: &str, args: any) -> any
        // args 通常为 map。
        engine.register_fn(
            "call_tool",
            move |tool_name: &str, args: Dynamic| -> Dynamic {
                let Some(tool) = tools_for_rhai.get(tool_name) else {
                    return rhai::serde::to_dynamic(json!({
                        "success": false,
                        "error": format!("tool not found: {}", tool_name)
                    }))
                    .unwrap_or_else(|_| Dynamic::from(false));
                };

                let input: Value = rhai::serde::from_dynamic(&args)
                    .unwrap_or_else(|_| json!({"_raw": args.to_string()}));

                let output: Result<Value, String> = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { tool.call(input).await })
                        .map_err(|e| e.to_string())
                });

                match output {
                    Ok(v) => rhai::serde::to_dynamic(v).unwrap_or_else(|_| Dynamic::from(false)),
                    Err(e) => rhai::serde::to_dynamic(json!({"success": false, "error": e}))
                        .unwrap_or_else(|_| Dynamic::from(false)),
                }
            },
        );
    });

    // 从工作区 skills/ 目录加载 skills（支持 SKILL.md / SKILL.rhai）。
    let skills_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("skills");

    let skills = agentkit::skills::load_skills_from_dir_with_rhai(&skills_dir, Some(registrar))
        .await
        .expect("load skills failed");

    // 将 skills 暴露为 tools，然后组装成 ToolRegistry，再按名称调用（不依赖 LLM provider）。
    let mut tools = ToolRegistry::new();
    for tool in skills.as_tools() {
        tools = tools.register_arc(tool);
    }

    let tool = tools.get("ai_news").expect("missing tool: ai_news");
    let out = tool
        .call(json!({"user_input": "获取今日AI资讯，优先网易科技。"}))
        .await;

    match out {
        Ok(v) => {
            // 完整链路（可选）：如果脚本返回 instruction，则交给 LLM 生成最终新闻列表。
            // 默认不依赖 provider，避免环境未配置导致示例失败。
            let use_llm = std::env::var("RHAI_SKILL_DEMO_USE_LLM")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            if use_llm {
                if let Some(inst) = v.get("instruction").and_then(|x| x.as_str()) {
                    // OpenAI-compatible provider（例如 Ollama /v1）
                    let base_url = std::env::var("OPENAI_BASE_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
                    let api_key =
                        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "ollama".to_string());
                    let model =
                        std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "qwen2.5:14b".to_string());

                    let provider = agentkit::provider::OpenAiProvider::new(base_url, api_key)
                        .with_default_model(model);

                    let resp = provider
                        .chat(ChatRequest::from_user_text(inst.to_string()).with_max_tokens(1200))
                        .await;

                    match resp {
                        Ok(r) => println!("{}", r.message.content),
                        Err(e) => {
                            eprintln!("provider 调用失败（已降级为打印 instruction）：{}", e);
                            println!("{}", inst);
                        }
                    }
                    return;
                }
            }

            // 降级：直接打印脚本结果（包含 instruction/diagnostic 等）。
            println!("{}", v)
        }
        Err(e) => eprintln!("运行失败：{}", e),
    }
}
