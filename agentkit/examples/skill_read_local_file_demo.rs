use agentkit_core::{
    agent::Agent,
    provider::types::{ChatMessage, Role},
};
use agentkit_runtime::{ToolCallingAgent, ToolRegistry};
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
    let tools: ToolRegistry = skills
        .as_tool_registry()
        .register(agentkit::tools::CmdExecTool::new());

    let agent = ToolCallingAgent::new(provider, tools)
        .with_system_prompt(
            "你是一个严谨的助手。你可以调用 skills/ 目录中加载的技能（例如 weather）。\n\
weather skill 会通过 cmd_exec 执行 SKILL.md 里给出的 curl 命令来获取真实天气信息。\n\
当用户询问天气（例如：北京今天怎么样）时，请优先调用 weather 获取真实结果后再回答。\n\
如果结果中包含 success=false（例如命令执行失败），请用中文说明失败原因（引用 error/stderr），并建议用户稍后重试。\n\
无论如何都请使用中文回答。",
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
