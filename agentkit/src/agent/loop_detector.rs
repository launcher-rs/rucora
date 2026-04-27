//! Agent 循环检测器
//!
//! 防止 Agent 陷入无限循环（重复调用相同工具、产生相同输出）。
//!
//! 参考实现: zeroclaw `loop_detector.rs`
//!
//! # 工作原理
//!
//! 每次工具调用后，记录 `(tool_name, args_hash, output_hash)` 三元组。
//! 当同一组合重复出现时，根据重复次数触发不同级别的响应：
//!
//! - `Warning`：重复次数超过 `max_repeats/2` 但小于 `max_repeats`，注入系统消息提示 LLM 调整策略
//! - `Block`：重复次数等于 `max_repeats`，用块消息替换工具输出
//! - `Break`：重复次数超过 `max_repeats`，终止整个 agent loop

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// 循环检测器配置
#[derive(Debug, Clone)]
pub struct LoopDetectorConfig {
    /// 是否启用循环检测
    pub enabled: bool,
    /// 滑动窗口大小（记录最近 N 次工具调用）
    pub window_size: usize,
    /// 最大允许重复次数（超过此数量触发 Break）
    pub max_repeats: usize,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            window_size: 20,
            max_repeats: 3,
        }
    }
}

/// 循环检测结果
#[derive(Debug, Clone, PartialEq)]
pub enum LoopDetectionResult {
    /// 正常，未检测到循环
    Ok,
    /// 警告：检测到重复模式，但未达到阈值。消息内容注入到历史。
    Warning(String),
    /// 阻止：重复达到半阈值，替换工具输出为阻止消息
    Block(String),
    /// 终止：重复超过阈值，中止整个 loop
    Break(String),
}

/// 工具调用记录（用于循环检测）
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ToolCallSignature {
    tool_name: String,
    args_hash: u64,
    output_hash: u64,
}

/// Agent 循环检测器
pub struct LoopDetector {
    config: LoopDetectorConfig,
    /// 最近 N 次工具调用的签名（滑动窗口）
    window: Vec<ToolCallSignature>,
    /// 各签名的出现次数（在当前窗口内）
    counts: HashMap<ToolCallSignature, usize>,
}

impl LoopDetector {
    /// 创建新的循环检测器
    pub fn new(config: LoopDetectorConfig) -> Self {
        Self {
            config,
            window: Vec::new(),
            counts: HashMap::new(),
        }
    }

    /// 记录一次工具调用，返回检测结果
    ///
    /// # 参数
    /// - `tool_name`: 工具名称
    /// - `args`: 工具调用参数
    /// - `output`: 工具输出（字符串）
    pub fn record(
        &mut self,
        tool_name: &str,
        args: &serde_json::Value,
        output: &str,
    ) -> LoopDetectionResult {
        if !self.config.enabled {
            return LoopDetectionResult::Ok;
        }

        let args_hash = hash_value(args);
        let output_hash = hash_str(output);

        let sig = ToolCallSignature {
            tool_name: tool_name.to_string(),
            args_hash,
            output_hash,
        };

        // 维护滑动窗口
        self.window.push(sig.clone());
        *self.counts.entry(sig.clone()).or_insert(0) += 1;

        // 当窗口超过大小时，移出最早的记录
        if self.window.len() > self.config.window_size {
            let oldest = self.window.remove(0);
            let count = self.counts.get_mut(&oldest).copied().unwrap_or(1);
            if count <= 1 {
                self.counts.remove(&oldest);
            } else {
                *self.counts.get_mut(&oldest).unwrap() -= 1;
            }
        }

        let repeat_count = *self.counts.get(&sig).unwrap_or(&0);
        let max = self.config.max_repeats;

        if repeat_count > max {
            let msg = format!(
                "工具 '{}' 产生了相同的参数和输出，连续重复 {} 次（超过阈值 {}），终止 agent loop。",
                tool_name, repeat_count, max
            );
            LoopDetectionResult::Break(msg)
        } else if repeat_count == max {
            let msg = format!(
                "工具 '{}' 重复调用 {} 次，输出相同。请换一种方法完成任务，不要继续重复调用。",
                tool_name, repeat_count
            );
            LoopDetectionResult::Block(msg)
        } else if repeat_count >= max / 2 + 1 && max >= 2 {
            let msg = format!(
                "[Loop Warning] 工具 '{}' 已重复产生相同输出 {} 次。请调整策略，避免循环。",
                tool_name, repeat_count
            );
            LoopDetectionResult::Warning(msg)
        } else {
            LoopDetectionResult::Ok
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn hash_value(v: &serde_json::Value) -> u64 {
    hash_str(&v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_repeat_returns_ok() {
        let mut detector = LoopDetector::new(LoopDetectorConfig::default());
        let result = detector.record("search", &serde_json::json!({"q": "rust"}), "result 1");
        assert_eq!(result, LoopDetectionResult::Ok);
    }

    #[test]
    fn repeat_triggers_warning_then_block_then_break() {
        let config = LoopDetectorConfig {
            enabled: true,
            window_size: 10,
            max_repeats: 3,
        };
        let mut detector = LoopDetector::new(config);
        let args = serde_json::json!({"q": "test"});
        let output = "same output";

        // 1st call: ok
        assert_eq!(
            detector.record("tool", &args, output),
            LoopDetectionResult::Ok
        );
        // 2nd call: warning (repeat_count=2 >= max/2+1=2)
        let r2 = detector.record("tool", &args, output);
        assert!(matches!(r2, LoopDetectionResult::Warning(_)));
        // 3rd call: block (repeat_count == max)
        let r3 = detector.record("tool", &args, output);
        assert!(matches!(r3, LoopDetectionResult::Block(_)));
        // 4th call: break (repeat_count > max)
        let r4 = detector.record("tool", &args, output);
        assert!(matches!(r4, LoopDetectionResult::Break(_)));
    }

    #[test]
    fn different_args_no_loop() {
        let mut detector = LoopDetector::new(LoopDetectorConfig::default());
        for i in 0..5 {
            let args = serde_json::json!({"q": i});
            let result = detector.record("search", &args, "result");
            assert_eq!(result, LoopDetectionResult::Ok);
        }
    }

    #[test]
    fn disabled_always_returns_ok() {
        let config = LoopDetectorConfig {
            enabled: false,
            ..Default::default()
        };
        let mut detector = LoopDetector::new(config);
        let args = serde_json::json!({});
        for _ in 0..10 {
            assert_eq!(
                detector.record("tool", &args, "out"),
                LoopDetectionResult::Ok
            );
        }
    }
}
