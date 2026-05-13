//! 日期时间工具
//!
//! 获取时间、日期、农历、生肖、星座等信息

use async_trait::async_trait;
use chrono::{Datelike, Local, NaiveDate, Timelike};
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory, types::ToolContext},
};
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
            (11, 1..=22) => "天蝎座",
            (11, _) => "射手座",
            (12, 1..=21) => "射手座",
            (12, _) => "摩羯座",
            _ => "未知",
        }
    }

    /// 获取详细的时间信息（JSON 格式）
    pub fn get_detailed_info(&self) -> Value {
        let now = Local::now();
        let year = now.year();
        let month = now.month();
        let day = now.day();
        let year_index = ((year - 4) % 60) as usize;

        let heavenly_stems = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
        let earthly_branches = [
            "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
        ];
        let zodiacs = [
            "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
        ];

        json!({
            "timestamp": now.timestamp(),
            "iso": now.to_rfc3339(),
            "local": now.format("%Y-%m-%d %H:%M:%S").to_string(),
            "date": {
                "year": year,
                "month": month,
                "day": day,
                "weekday": now.weekday().to_string(),
                "weekday_num": now.weekday().number_from_monday()
            },
            "time": {
                "hour": now.hour(),
                "minute": now.minute(),
                "second": now.second()
            },
            "lunar": {
                "stem": heavenly_stems[year_index % 10],
                "branch": earthly_branches[year_index % 12],
                "zodiac": zodiacs[year_index % 12]
            },
            "constellation": Self::get_constellation(month, day)
        })
    }

    /// 计算两个日期之间的天数。
    pub fn days_between(&self, date1: &str, date2: &str) -> Result<i32, ToolError> {
        let d1 = NaiveDate::parse_from_str(date1, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;
        let d2 = NaiveDate::parse_from_str(date2, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;

        Ok((d2 - d1).num_days() as i32)
    }

    /// 获取指定日期的详细信息。
    pub fn get_date_detail(&self, date: &str) -> Result<String, ToolError> {
        let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| ToolError::Message(format!("解析日期失败：{e}")))?;

        let year = naive_date.year();
        let year_index = ((year - 4) % 60) as usize;
        let heavenly_stems = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
        let earthly_branches = [
            "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
        ];
        let zodiacs = [
            "鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪",
        ];

        let weekday = match naive_date.weekday().number_from_monday() {
            1 => "一",
            2 => "二",
            3 => "三",
            4 => "四",
            5 => "五",
            6 => "六",
            7 => "日",
            _ => "未知",
        };

        Ok(format!(
            "公历：{naive_date}, 农历：{}{}年, 生肖：{}, 星座：{}, 星期：{}",
            heavenly_stems[year_index % 10],
            earthly_branches[year_index % 12],
            zodiacs[year_index % 12],
            Self::get_constellation(naive_date.month(), naive_date.day()),
            weekday
        ))
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
        Some("获取当前日期时间信息、查询指定日期详情、计算两个日期之间的天数")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "description": "输出格式：text（文本）或 json（详细 JSON）",
                    "enum": ["text", "json"],
                    "default": "text"
                },
                "action": {
                    "type": "string",
                    "description": "操作类型：now（当前时间）、detail（指定日期详情）、days_between（日期差）",
                    "enum": ["now", "detail", "days_between"],
                    "default": "now"
                },
                "date": {
                    "type": "string",
                    "description": "日期字符串，格式：YYYY-MM-DD（用于 detail）"
                },
                "date1": {
                    "type": "string",
                    "description": "第一个日期字符串，格式：YYYY-MM-DD（用于 days_between）"
                },
                "date2": {
                    "type": "string",
                    "description": "第二个日期字符串，格式：YYYY-MM-DD（用于 days_between）"
                }
            }
        })
    }

    async fn call(&self, input: Value, _context: &ToolContext) -> Result<Value, ToolError> {
        let format = input
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("now");

        match action {
            "now" => match format {
                "json" => Ok(self.get_detailed_info()),
                _ => Ok(json!({
                    "info": self.get_time_info()
                })),
            },
            "detail" => {
                let date = input
                    .get("date")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::Message("detail 操作需要 date 参数".to_string()))?;
                Ok(json!({
                    "date": date,
                    "detail": self.get_date_detail(date)?
                }))
            }
            "days_between" => {
                let date1 = input.get("date1").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::Message("days_between 操作需要 date1 参数".to_string())
                })?;
                let date2 = input.get("date2").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::Message("days_between 操作需要 date2 参数".to_string())
                })?;

                Ok(json!({
                    "date1": date1,
                    "date2": date2,
                    "days": self.days_between(date1, date2)?
                }))
            }
            other => Err(ToolError::Message(format!("未知 action：{other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constellation() {
        let _tool = DatetimeTool::new();

        // 测试几个已知日期
        assert_eq!(DatetimeTool::get_constellation(1, 15), "摩羯座");
        assert_eq!(DatetimeTool::get_constellation(3, 21), "白羊座");
        assert_eq!(DatetimeTool::get_constellation(6, 22), "巨蟹座");
        assert_eq!(DatetimeTool::get_constellation(12, 25), "摩羯座");
    }

    #[test]
    fn test_time_info() {
        let tool = DatetimeTool::new();
        let info = tool.get_time_info();

        // 验证包含关键信息
        assert!(info.contains("当前时间"));
        assert!(info.contains("公历"));
        assert!(info.contains("农历"));
        assert!(info.contains("生肖"));
        assert!(info.contains("星座"));
        assert!(info.contains("星期"));
    }

    #[test]
    fn test_days_between() {
        let tool = DatetimeTool::new();
        assert_eq!(
            tool.days_between("2026-05-01", "2026-05-09")
                .expect("应能计算日期差"),
            8
        );
    }

    #[test]
    fn test_date_detail() {
        let tool = DatetimeTool::new();
        let detail = tool
            .get_date_detail("2026-05-09")
            .expect("应能查询日期详情");
        assert!(detail.contains("公历"));
        assert!(detail.contains("星期"));
    }
}
