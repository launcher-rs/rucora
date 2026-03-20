use std::path::PathBuf;
use std::sync::Arc;

use agentkit::config::AgentkitConfig;
use agentkit_core::agent::types::AgentInput;
use agentkit_core::provider::types::{ChatMessage, Role};
use agentkit_core::runtime::Runtime;
use agentkit_runtime::trace::write_trace_jsonl;
use agentkit_runtime::{ChannelEvent, DefaultRuntime, ToolRegistry};
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
async fn main() {
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
            let profile = AgentkitConfig::load().await.expect("load config failed");
            let provider = AgentkitConfig::build_provider(&profile).expect("build provider failed");

            let skills = agentkit::skills::load_skills_from_dir(&skill_dir)
                .await
                .expect("load skills failed");

            let mut tools = ToolRegistry::new();
            for tool in skills.as_tools() {
                tools = tools.register_arc(tool);
            }
            tools = tools.register(agentkit::tools::CmdExecTool::new());

            let prompt = prompt.unwrap_or_else(|| "用一句话介绍 Rust".to_string());
            let input = AgentInput {
                messages: vec![ChatMessage {
                    role: Role::User,
                    content: prompt.clone(),
                    name: None,
                }],
                metadata: None,
            };

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
                            eprintln!("agent error: {}", e);
                            break;
                        }
                    }
                }
                println!();

                if let Some(path) = trace_path {
                    write_trace_jsonl(&path, &events)
                        .await
                        .expect("write trace failed");
                    eprintln!("trace saved: {} (events={})", path, events.len());
                }
            } else {
                let agent = DefaultRuntime::new(Arc::new(provider), tools)
                    .with_max_steps(max_steps);

                let out = agent.run(input).await;
                match out {
                    Ok(out) => println!("{}", out.message.content),
                    Err(e) => eprintln!("agent error: {}", e),
                }
            }
        }
    }
}
