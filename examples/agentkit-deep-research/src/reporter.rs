//! 研究报告生成与管理模块
//!
//! 负责：
//! - 将研究结果格式化为完整 Markdown 报告
//! - 自动保存报告到文件
//! - 显示进度和摘要

use crate::research_agent::PhaseResult;
use chrono::Local;
use std::path::{Path, PathBuf};
use tracing::info;

/// 报告生成器
pub struct Reporter {
    /// 报告输出目录
    output_dir: PathBuf,
}

impl Reporter {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }

    /// 使用默认目录（当前工作目录下的 reports/）
    pub fn default() -> Self {
        Self::new("reports")
    }

    /// 保存完整报告，返回文件路径
    pub fn save_report(&self, topic: &str, report_content: &str) -> anyhow::Result<PathBuf> {
        // 确保目录存在
        std::fs::create_dir_all(&self.output_dir)?;

        // 生成文件名（主题 + 时间戳）
        let safe_topic = sanitize_filename(topic);
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("research_{}_{}.md", safe_topic, timestamp);
        let path = self.output_dir.join(&filename);

        std::fs::write(&path, report_content)?;
        info!("💾 报告已保存: {}", path.display());

        Ok(path)
    }

    /// 保存阶段性中间结果（供调试和审查）
    pub fn save_phase_results(
        &self,
        topic: &str,
        phases: &[PhaseResult],
    ) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(&self.output_dir)?;

        let safe_topic = sanitize_filename(topic);
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("research_phases_{}_{}.md", safe_topic, timestamp);
        let path = self.output_dir.join(&filename);

        let mut content = format!("# 研究阶段记录：{topic}\n\n生成时间：{}\n\n", Local::now());

        for (i, phase) in phases.iter().enumerate() {
            let phase_name = match phase.phase {
                crate::research_agent::ResearchPhase::SearchAndGather => "阶段 1：搜索收集",
                crate::research_agent::ResearchPhase::DeepRead => "阶段 2：深度精读",
                crate::research_agent::ResearchPhase::Synthesize => "阶段 3：综合报告",
            };

            content.push_str(&format!(
                "---\n\n## {} (第 {} 阶段)\n\n**Token 消耗**: {}\n\n{}\n\n",
                phase_name,
                i + 1,
                phase.tokens,
                phase.content
            ));
        }

        std::fs::write(&path, &content)?;
        info!("💾 阶段记录已保存: {}", path.display());

        Ok(path)
    }

    /// 打印研究完成摘要
    pub fn print_summary(&self, topic: &str, phases: &[PhaseResult], report_path: &Path) {
        let total_tokens: u32 = phases.iter().map(|p| p.tokens).sum();
        let total_chars: usize = phases.iter().map(|p| p.content.len()).sum();

        println!("\n{}", "═".repeat(60));
        println!("  研究完成：{}", topic);
        println!("{}", "═".repeat(60));
        println!("  阶段数量：{}", phases.len());
        println!("  总 Token：{}", total_tokens);
        println!("  总字符数：{}", total_chars);
        println!("  报告路径：{}", report_path.display());
        println!("{}", "═".repeat(60));

        // 打印报告前 800 字作为预览
        if let Some(last_phase) = phases.last() {
            let preview: String = last_phase.content.chars().take(800).collect();
            println!("\n报告预览：\n\n{}", preview);
            if last_phase.content.len() > 800 {
                println!("\n... [报告较长，请查看完整文件] ...\n");
            }
        }
    }
}

/// 将主题转换为合法的文件名
fn sanitize_filename(topic: &str) -> String {
    topic
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '_',
            '\u{4e00}'..='\u{9fff}' => c, // 中文字符
            _ => '_',
        })
        .take(40)
        .collect()
}
