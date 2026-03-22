use std::path::PathBuf;
use std::sync::Arc;

use agentkit::config::AgentkitConfig;
use agentkit::runtime::trace::write_trace_jsonl;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::agent::types::AgentInput;
use agentkit_core::channel::types::ChannelEvent;
use agentkit_core::runtime::Runtime;
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "agentkit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(long, default_value = "skills")]
        skill_dir: PathBuf,

        #[arg(long)]
        prompt: Option<String>,

        #[arg(long, default_value_t = 6)]
        max_steps: usize,

        #[arg(long)]
        trace_path: Option<String>,

        #[arg(long, default_value_t = true)]
        stream: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            skill_dir,
            prompt,
            max_steps,
            trace_path,
            stream,
        } => {
            // 加载配置（失败时返回错误）
            let profile = match AgentkitConfig::load().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("错误：加载配置失败 - {}", e);
                    std::process::exit(1);
                }
            };

            // 构建 provider（失败时返回错误）
            let provider = match AgentkitConfig::build_provider(&profile) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("错误：构建 provider 失败 - {}", e);
                    std::process::exit(1);
                }
            };

            // 加载 skills
            let skills = match agentkit::skills::load_skills_from_dir(&skill_dir).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("错误：加载 skills 失败 - {}", e);
                    std::process::exit(1);
                }
            };

            // 构建工具注册表
            let mut tools = ToolRegistry::new();
            for tool in skills.as_tools() {
                tools = tools.register_arc(tool);
            }
            tools = tools.register(agentkit::tools::CmdExecTool::new());

            let prompt = prompt.unwrap_or_else(|| "用一句话介绍 Rust".to_string());
            let input = AgentInput::new(prompt.clone());

            if stream {
                let agent = DefaultRuntime::new(Arc::new(provider), tools)
                    .with_system_prompt(prompt)
                    .with_max_steps(max_steps);

                let mut events: Vec<ChannelEvent> = Vec::new();
                let mut s = agent.run_stream(input);

                while let Some(item) = s.next().await {
                    match item {
                        Ok(ev) => {
                            if let ChannelEvent::TokenDelta(delta) = &ev {
                                print!("{}", delta.delta);
                            }
                            events.push(ev);
                        }
                        Err(e) => {
                            eprintln!("agent 错误：{}", e);
                            break;
                        }
                    }
                }
                println!();

                if let Some(path) = trace_path {
                    if let Err(e) = write_trace_jsonl(&path, &events).await {
                        eprintln!("警告：写入 trace 失败 - {}", e);
                    } else {
                        eprintln!("trace 已保存：{} (events={})", path, events.len());
                    }
                }
            } else {
                let agent =
                    DefaultRuntime::new(Arc::new(provider), tools).with_max_steps(max_steps);

                match agent.run(input).await {
                    Ok(out) => {
                        // 从 output.value 中提取 content
                        if let Some(content) = out.value.get("content").and_then(|v| v.as_str()) {
                            println!("{}", content);
                        } else {
                            println!("{}", out.value);
                        }
                    }
                    Err(e) => {
                        eprintln!("agent 错误：{}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    Ok(())
}
