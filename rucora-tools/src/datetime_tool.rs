//! 日期时间工具
//!
//! 获取时间、日期、农历、生肖、星座等信息
//!
//! 注意：如需使用完整的农历、节气、法定假日等功能，可以集成 tyme4rs 库
//! 但由于 tyme4rs 1.4.1 API 有重大变化，暂时使用 chrono 实现基础功能

use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use chrono::{Datelike, Local, NaiveDate};
use serde_json::{Value, json};

/// 日期时间工具
///
/// 获取当前时间信息，包括公历、农历（干支纪年）、生肖、星座等
pub struct DatetimeTool;

impl DatetimeTool {
    /// 创建新的日期时间工具
    pub fn new() -> Self {
        Self
    }

    /// 获取时间信息
    pub fn get_time_info(&self) -> String {
        let now = Local::now();
        let mut info = Vec::new();

        // 公历时间
        let time_info = format!("当前时间：{}", now.format("%Y-%m-%d %H:%M:%S"));
        info.push(time_info);

        // 公历日期
        let year = now.year();
        let month = now.month();
        let day = now.day();
        info.push(format!("公历：{year}年{month}月{day}日"));

        // 农历干支纪年（简化计算）
        let heavenly_stems = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
        let earthly_branches = [
            "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
        ];
        let year_index = ((year - 4) % 60) as usize;
        let stem = heavenly_stems[year_index % 10];
        let branch = earthly_branches[year_index % 12];
        info.push(format!("农历：{stem}{branch}年"));

        // 生肖
        let zodiacs = [
            "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
        ];
        let zodiac = zodiacs[year_index % 12];
        info.push(format!("生肖：{zodiac}"));

        // 星座
        let constellation = Self::get_constellation(month, day);
        info.push(format!("星座：{constellation}"));

        // 星期
        let weekday = now.weekday();
        info.push(format!(
            "星期：{}",
            match weekday.number_from_monday() {
                1 => "一",
                2 => "二",
                3 => "三",
                4 => "四",
                5 => "五",
                6 => "六",
                7 => "日",
                _ => "未知",
            }
        ));

        info.join(", ")
    }

    /// 根据月日获取星座
    fn get_constellation(month: u32, day: u32) -> &'static str {
        match (month, day) {
            (1, 1..=19) => "摩羯座",
            (1, _) => "水瓶座",
            (2, 1..=18) => "水瓶座",
            (2, _) => "双鱼座",
            (3, 1..=20) => "双鱼座",
            (3, _) => "白羊座",
            (4, 1..=19) => "白羊座",
            (4, _) => "金牛座",
            (5, 1..=20) => "金牛座",
            (5, _) => "双子座",
            (6, 1..=21) => "双子座",
            (6, _) => "巨蟹座",
            (7, 1..=22) => "巨蟹座",
            (7, _) => "狮子座",
            (8, 1..=22) => "狮子座",
            (8, _) => "处女座",
            (9, 1..=22) => "处女座",
            (9, _) => "天秤座",
            (10, 1..=23) => "天秤座",
            (10, _) => "天蝎座",
            (11, 1..=21) => "天蝎座",
            (11, _) => "射手座",
            (12, 1..=21) => "射手座",
            (12, _) => "摩羯座",
            _ => "未知",
        }
    }

    /// 获取两个日期之间的天数
    pub fn days_between(&self, date1: &str, date2: &str) -> Result<i32, ToolError> {
        let d1 = NaiveDate::parse_from_str(date1, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;
        let d2 = NaiveDate::parse_from_str(date2, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;

        Ok((d2 - d1).num_days() as i32)
    }

    /// 获取指定日期的详细信息
    pub fn get_date_detail(&self, date: &str) -> Result<String, ToolError> {
        let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;

        let mut info = Vec::new();

        // 公历
        info.push(format!("公历：{naive_date}"));

        // 干支纪年
        let year = naive_date.year();
        let heavenly_stems = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
        let earthly_branches = [
            "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
        ];
        let year_index = ((year - 4) % 60) as usize;
        let stem = heavenly_stems[year_index % 10];
        let branch = earthly_branches[year_index % 12];
        info.push(format!("农历：{stem}{branch}年"));

        // 生肖
        let zodiacs = [
            "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
        ];
        info.push(format!("生肖：{}", zodiacs[year_index % 12]));

        // 星座
        info.push(format!(
            "星座：{}",
            Self::get_constellation(naive_date.month(), naive_date.day())
        ));

        // 星期
        let weekday = naive_date.weekday();
        info.push(format!(
            "星期：{}",
            match weekday.number_from_monday() {
                1 => "一",
                2 => "二",
                3 => "三",
                4 => "四",
                5 => "五",
                6 => "六",
                7 => "日",
                _ => "未知",
            }
        ));

        Ok(info.join(", "))
    }
}

impl Default for DatetimeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DatetimeTool {
    fn name(&self) -> &str {
        "datetime"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "获取当前时间日期信息，包括公历、农历（干支）、生肖、星座等。也支持查询指定日期详情和计算日期差。",
        )
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "操作类型：'now'（获取当前时间）, 'detail'（查询指定日期详情）, 'days_between'（计算两个日期之间的天数）",
                    "enum": ["now", "detail", "days_between"]
                },
                "date": {
                    "type": "string",
                    "description": "日期字符串，格式：YYYY-MM-DD（用于 detail 操作）"
                },
                "date1": {
                    "type": "string",
                    "description": "第一个日期字符串，格式：YYYY-MM-DD（用于 days_between 操作）"
                },
                "date2": {
                    "type": "string",
                    "description": "第二个日期字符串，格式：YYYY-MM-DD（用于 days_between 操作）"
                }
            },
            "required": ["action"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("now");

        match action {
            "now" => {
                let time_info = self.get_time_info();
                Ok(json!({
                    "action": "now",
                    "time_info": time_info,
                    "success": true
                }))
            }
            "detail" => {
                let date = input
                    .get("date")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::Message("缺少必需的 'date' 字段".to_string()))?;

                let detail = self.get_date_detail(date)?;
                Ok(json!({
                    "action": "detail",
                    "date": date,
                    "detail": detail,
                    "success": true
                }))
            }
            "days_between" => {
                let date1 = input
                    .get("date1")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::Message("缺少必需的 'date1' 字段".to_string()))?;
                let date2 = input
                    .get("date2")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::Message("缺少必需的 'date2' 字段".to_string()))?;

                let days = self.days_between(date1, date2)?;
                Ok(json!({
                    "action": "days_between",
                    "date1": date1,
                    "date2": date2,
                    "days": days,
                    "success": true
                }))
            }
            _ => Err(ToolError::Message(format!("未知的操作类型：{action}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_tool_now() {
        let tool = DatetimeTool::new();
        let info = tool.get_time_info();
        println!("时间信息：{}", info);
        assert!(info.contains("当前时间"));
        assert!(info.contains("生肖") || info.contains("星座"));
    }

    #[test]
    fn test_datetime_tool_detail() {
        let tool = DatetimeTool::new();
        let result = tool.get_date_detail("2026-03-23");
        assert!(result.is_ok());
        let detail = result.unwrap();
        println!("日期详情：{}", detail);
        assert!(detail.contains("公历"));
    }

    #[test]
    fn test_datetime_tool_days_between() {
        let tool = DatetimeTool::new();
        let days = tool.days_between("2026-01-01", "2026-12-31");
        assert!(days.is_ok());
        println!("天数差：{}", days.unwrap());
    }

    #[tokio::test]
    async fn test_datetime_tool_call() {
        let tool = DatetimeTool::new();

        // 测试获取当前时间
        let result = tool.call(json!({"action": "now"})).await;
        assert!(result.is_ok());
        println!("当前时间：{}", result.unwrap());

        // 测试查询指定日期
        let result = tool
            .call(json!({
                "action": "detail",
                "date": "2026-10-01"
            }))
            .await;
        assert!(result.is_ok());
        println!("日期详情：{}", result.unwrap());
    }
}
