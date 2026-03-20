use agentkit::skills::SkillRegistry;
use agentkit_core::{
    agent::types::AgentInput,
    provider::types::{ChatMessage, Role},
    runtime::Runtime,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // provider：使用 OpenAI-compatible API（例如 Ollama /v1）
    let provider = agentkit::provider::OpenAiProvider::new("http://127.0.0.1:11434/v1", "ollama")
        .with_default_model("qwen2.5:14b");

    // 从工作区 skills/ 目录加载 skills（目前支持 weather）。
    let skills_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("skills");

    let skills = agentkit::skills::load_skills_from_dir(&skills_dir)
        .await
        .expect("load skills failed");

    // 将 skills 暴露为 tool schema，交给 ToolCallingAgent 让 LLM 决策调用。
    let mut tools = ToolRegistry::new();
    for tool in skills.as_tools() {
        tools = tools.register_arc(tool);
    }
    let tools: ToolRegistry = tools.register(agentkit::tools::CmdExecTool::new());

    let agent = DefaultRuntime::new(std::sync::Arc::new(provider), tools)
        .with_system_prompt(
            "你是一个严谨的助手。你可以调用外部工具读取本地文件内容。\n\
读取完成后请总结主要信息并给出文件路径。\n\
回答请使用中文。",
        )
        .with_max_steps(6);

    let prompt = "北京今天怎么样？请查询今天的天气并用中文回答。".to_string();

    let out = agent
        .run(agentkit_core::agent::types::AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: prompt,
                name: None,
            }],
            metadata: None,
        })
        .await;

    match out {
        Ok(out) => println!("{}", out.message.content),
        Err(e) => eprintln!("运行失败：{}", e),
    }
}
