//! 高级计算器工具
//!
//! 支持 25 种数学函数：算术运算、对数指数、聚合统计、百分位数等

use async_trait::async_trait;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};

/// 高级计算器工具
///
/// 支持 25 种数学函数，包括：
/// - 算术运算: add, subtract, multiply, divide, pow, sqrt, abs, modulo, round
/// - 对数指数: log, ln, exp, factorial
/// - 聚合统计: sum, average, count, min, max, range
/// - 统计分析: median, mode, variance, stdev, percentile
/// - 实用函数: percentage_change, clamp
pub struct CalculatorTool;

impl CalculatorTool {
    /// 创建新的计算器工具实例
    pub fn new() -> Self {
        Self
    }

    /// 执行计算
    fn calculate(&self, function: &str, args: &Value) -> Result<f64, String> {
        match function {
            // 算术运算
            "add" => {
                let values = get_values(args)?;
                Ok(values.iter().sum())
            }
            "subtract" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("subtract 需要至少一个数值".to_string());
                }
                let first = values[0];
                let rest: f64 = values[1..].iter().sum();
                Ok(first - rest)
            }
            "multiply" => {
                let values = get_values(args)?;
                Ok(values.iter().product())
            }
            "divide" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("divide 需要至少一个数值".to_string());
                }
                let first = values[0];
                let mut result = first;
                for &v in &values[1..] {
                    if v == 0.0 {
                        return Err("除数不能为零".to_string());
                    }
                    result /= v;
                }
                Ok(result)
            }
            "pow" => {
                let a = get_f64(args, "a")?;
                let b = get_f64(args, "b")?;
                Ok(a.powf(b))
            }
            "sqrt" => {
                let x = get_f64(args, "x")?;
                if x < 0.0 {
                    return Err("不能对负数开平方".to_string());
                }
                Ok(x.sqrt())
            }
            "abs" => {
                let x = get_f64(args, "x")?;
                Ok(x.abs())
            }
            "modulo" => {
                let a = get_f64(args, "a")?;
                let b = get_f64(args, "b")?;
                if b == 0.0 {
                    return Err("模数不能为零".to_string());
                }
                Ok(a % b)
            }
            "round" => {
                let x = get_f64(args, "x")?;
                let decimals = get_u64(args, "decimals").unwrap_or(0) as i32;
                let multiplier = 10f64.powi(decimals);
                Ok((x * multiplier).round() / multiplier)
            }

            // 对数指数
            "log" => {
                let x = get_f64(args, "x")?;
                let base = get_f64(args, "base").unwrap_or(10.0);
                if x <= 0.0 {
                    return Err("对数参数必须为正数".to_string());
                }
                if base <= 0.0 || base == 1.0 {
                    return Err("对数底数必须为正数且不等于1".to_string());
                }
                Ok(x.log(base))
            }
            "ln" => {
                let x = get_f64(args, "x")?;
                if x <= 0.0 {
                    return Err("自然对数参数必须为正数".to_string());
                }
                Ok(x.ln())
            }
            "exp" => {
                let x = get_f64(args, "x")?;
                Ok(x.exp())
            }
            "factorial" => {
                let x = get_f64(args, "x")?;
                if x < 0.0 {
                    return Err("阶乘参数不能为负数".to_string());
                }
                if x > 170.0 {
                    return Err("阶乘参数过大".to_string());
                }
                let n = x as u64;
                let result = (1..=n).fold(1.0, |acc, i| acc * i as f64);
                Ok(result)
            }

            // 聚合统计
            "sum" => {
                let values = get_values(args)?;
                Ok(values.iter().sum())
            }
            "average" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("average 需要至少一个数值".to_string());
                }
                Ok(values.iter().sum::<f64>() / values.len() as f64)
            }
            "count" => {
                let values = get_values(args)?;
                Ok(values.len() as f64)
            }
            "min" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("min 需要至少一个数值".to_string());
                }
                Ok(values.iter().fold(f64::INFINITY, |a, &b| a.min(b)))
            }
            "max" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("max 需要至少一个数值".to_string());
                }
                Ok(values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)))
            }
            "range" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("range 需要至少一个数值".to_string());
                }
                let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                Ok(max - min)
            }

            // 统计分析
            "median" => {
                let mut values = get_values(args)?;
                if values.is_empty() {
                    return Err("median 需要至少一个数值".to_string());
                }
                values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let len = values.len();
                if len % 2 == 0 {
                    Ok((values[len / 2 - 1] + values[len / 2]) / 2.0)
                } else {
                    Ok(values[len / 2])
                }
            }
            "mode" => {
                let values = get_values(args)?;
                if values.is_empty() {
                    return Err("mode 需要至少一个数值".to_string());
                }
                use std::collections::HashMap;
                let mut counts: HashMap<i64, usize> = HashMap::new();
                for v in &values {
                    let key = (*v * 1_000_000.0).round() as i64; // 处理浮点数精度
                    *counts.entry(key).or_insert(0) += 1;
                }
                let max_count = counts.values().max().copied().unwrap_or(0);
                let mode_key = counts
                    .iter()
                    .find(|&(_, c)| *c == max_count)
                    .map_or(0, |(k, _)| *k);
                Ok(mode_key as f64 / 1_000_000.0)
            }
            "variance" => {
                let values = get_values(args)?;
                if values.len() < 2 {
                    return Err("variance 需要至少两个数值".to_string());
                }
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance =
                    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                Ok(variance)
            }
            "stdev" => {
                let values = get_values(args)?;
                if values.len() < 2 {
                    return Err("stdev 需要至少两个数值".to_string());
                }
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let variance =
                    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
                Ok(variance.sqrt())
            }
            "percentile" => {
                let mut values = get_values(args)?;
                if values.is_empty() {
                    return Err("percentile 需要至少一个数值".to_string());
                }
                let p = get_f64(args, "p")?;
                if !(0.0..=100.0).contains(&p) {
                    return Err("百分位数必须在 0-100 之间".to_string());
                }
                values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let index = (p / 100.0 * (values.len() - 1) as f64).round() as usize;
                Ok(values[index.min(values.len() - 1)])
            }

            // 实用函数
            "percentage_change" => {
                let a = get_f64(args, "a")?;
                let b = get_f64(args, "b")?;
                if a == 0.0 {
                    return Err("percentage_change 的第一个参数不能为零".to_string());
                }
                Ok(((b - a) / a) * 100.0)
            }
            "clamp" => {
                let x = get_f64(args, "x")?;
                let min_val = get_f64(args, "min_val")?;
                let max_val = get_f64(args, "max_val")?;
                Ok(x.clamp(min_val, max_val))
            }

            _ => Err(format!("未知的函数: {function}")),
        }
    }
}

impl Default for CalculatorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "执行高级数学计算。支持 25 种函数：\
             算术: add, subtract, multiply, divide, pow, sqrt, abs, modulo, round; \
             对数指数: log, ln, exp, factorial; \
             聚合: sum, average, count, min, max, range; \
             统计: median, mode, variance, stdev, percentile; \
             实用: percentage_change, clamp",
        )
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "function": {
                    "type": "string",
                    "description": "要执行的计算函数",
                    "enum": [
                        "add", "subtract", "multiply", "divide", "pow", "sqrt",
                        "abs", "modulo", "round", "log", "ln", "exp", "factorial",
                        "sum", "average", "count", "min", "max", "range",
                        "median", "mode", "variance", "stdev", "percentile",
                        "percentage_change", "clamp"
                    ]
                },
                "values": {
                    "type": "array",
                    "items": { "type": "number" },
                    "description": "数值数组，用于聚合函数 (sum, average, min, max 等)"
                },
                "a": {
                    "type": "number",
                    "description": "第一个操作数，用于 pow, modulo, percentage_change"
                },
                "b": {
                    "type": "number",
                    "description": "第二个操作数，用于 pow, modulo, percentage_change"
                },
                "x": {
                    "type": "number",
                    "description": "输入值，用于 sqrt, abs, log, ln, exp, factorial, round"
                },
                "base": {
                    "type": "number",
                    "description": "对数底数（可选，默认为 10），用于 log"
                },
                "decimals": {
                    "type": "integer",
                    "description": "小数位数，用于 round"
                },
                "p": {
                    "type": "number",
                    "description": "百分位数 (0-100)，用于 percentile"
                },
                "min_val": {
                    "type": "number",
                    "description": "最小值，用于 clamp"
                },
                "max_val": {
                    "type": "number",
                    "description": "最大值，用于 clamp"
                }
            },
            "required": ["function"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let function = input
            .get("function")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 'function' 参数".to_string()))?;

        match self.calculate(function, &input) {
            Ok(result) => Ok(json!({
                "result": result,
                "function": function
            })),
            Err(e) => Err(ToolError::Message(e)),
        }
    }
}

// 辅助函数
fn get_values(args: &Value) -> Result<Vec<f64>, String> {
    args.get("values")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "缺少 'values' 参数".to_string())?
        .iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| "values 数组必须包含数值".to_string())
        })
        .collect()
}

fn get_f64(args: &Value, key: &str) -> Result<f64, String> {
    args.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("缺少或无效的 '{key}' 参数"))
}

fn get_u64(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("缺少或无效的 '{key}' 参数"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let calc = CalculatorTool::new();

        // add
        let result = calc.calculate("add", &json!({"values": [1.0, 2.0, 3.0]}));
        assert_eq!(result.unwrap(), 6.0);

        // multiply
        let result = calc.calculate("multiply", &json!({"values": [2.0, 3.0, 4.0]}));
        assert_eq!(result.unwrap(), 24.0);

        // pow
        let result = calc.calculate("pow", &json!({"a": 2.0, "b": 3.0}));
        assert_eq!(result.unwrap(), 8.0);

        // sqrt
        let result = calc.calculate("sqrt", &json!({"x": 16.0}));
        assert_eq!(result.unwrap(), 4.0);
    }

    #[test]
    fn test_statistics() {
        let calc = CalculatorTool::new();

        // average
        let result = calc.calculate("average", &json!({"values": [1.0, 2.0, 3.0, 4.0, 5.0]}));
        assert_eq!(result.unwrap(), 3.0);

        // median
        let result = calc.calculate("median", &json!({"values": [1.0, 3.0, 5.0, 7.0, 9.0]}));
        assert_eq!(result.unwrap(), 5.0);

        // max
        let result = calc.calculate("max", &json!({"values": [3.0, 1.0, 4.0, 1.0, 5.0]}));
        assert_eq!(result.unwrap(), 5.0);
    }

    #[test]
    fn test_edge_cases() {
        let calc = CalculatorTool::new();

        // 负数平方根
        let result = calc.calculate("sqrt", &json!({"x": -1.0}));
        assert!(result.is_err());

        // 除零
        let result = calc.calculate("divide", &json!({"values": [10.0, 0.0]}));
        assert!(result.is_err());

        // clamp
        let result = calc.calculate(
            "clamp",
            &json!({"x": 15.0, "min_val": 0.0, "max_val": 10.0}),
        );
        assert_eq!(result.unwrap(), 10.0);
    }
}
