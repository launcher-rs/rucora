//! Prompt 注入防护扫描器 trait（纯接口层）
//!
//! 本模块只定义注入防护的接口和类型，不包含具体实现。
//! 实现位于 agentkit crate 的 injection_guard_impl 模块。

use serde::{Deserialize, Serialize};

/// 威胁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatType {
    /// Prompt 注入（试图忽略先前指令）
    PromptInjection,
    /// 规则规避（试图绕过安全限制）
    DisregardRules,
    /// 信息隐藏（试图向用户隐瞒信息）
    ConcealInfo,
    /// 权限绕过（试图获取未授权访问）
    BypassRestrictions,
    /// 秘密文件读取（尝试读取敏感配置文件）
    ReadSecrets,
    /// 数据外泄（通过 curl/wget 外传数据）
    ExfilCurl,
    /// 隐藏 Unicode 字符（可能用于隐藏恶意内容）
    HiddenUnicode,
    /// 角色伪装（伪装成系统指令）
    RoleImpersonation,
}

impl ThreatType {
    /// 获取威胁等级（1-5，5 最严重）
    pub fn severity(&self) -> u8 {
        match self {
            Self::HiddenUnicode => 5,
            Self::ReadSecrets => 5,
            Self::ExfilCurl => 5,
            Self::PromptInjection => 4,
            Self::RoleImpersonation => 4,
            Self::BypassRestrictions => 4,
            Self::ConcealInfo => 3,
            Self::DisregardRules => 3,
        }
    }

    /// 获取人类可读的描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::PromptInjection => "检测到 Prompt 注入攻击",
            Self::DisregardRules => "检测到规则规避尝试",
            Self::ConcealInfo => "检测到信息隐藏尝试",
            Self::BypassRestrictions => "检测到权限绕过尝试",
            Self::ReadSecrets => "检测到敏感文件读取尝试",
            Self::ExfilCurl => "检测到数据外泄尝试",
            Self::HiddenUnicode => "检测到隐藏 Unicode 字符",
            Self::RoleImpersonation => "检测到角色伪装尝试",
        }
    }
}

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 是否安全（未检测到威胁）
    pub is_safe: bool,
    /// 检测到的威胁列表
    pub threats: Vec<Threat>,
    /// 清理后的内容（移除了危险部分）
    pub cleaned_content: Option<String>,
    /// 原始内容长度
    pub original_length: usize,
}

/// 单个威胁详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    /// 威胁类型
    pub threat_type: ThreatType,
    /// 匹配到的文本片段
    pub matched_text: String,
    /// 在原文中的起始位置（字符偏移）
    pub start_pos: usize,
    /// 在原文中的结束位置（字符偏移）
    pub end_pos: usize,
    /// 威胁等级
    pub severity: u8,
}

/// 注入防护扫描器 trait（纯接口）
///
/// 具体实现位于 agentkit crate。
pub trait InjectionGuard: Send + Sync {
    /// 扫描内容并检测威胁
    fn scan(&self, content: &str, source: &str) -> ScanResult;

    /// 快速扫描（静态方法风格）
    fn quick_scan(&self, content: &str, source: &str) -> ScanResult {
        self.scan(content, source)
    }
}

/// 可扫描内容的 trait
///
/// 为 String 和 &str 提供便捷的扫描方法
pub trait ContentScannable {
    /// 扫描内容安全性
    fn scan_for_injection(&self, guard: &dyn InjectionGuard, source: &str) -> ScanResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_type_severity() {
        assert_eq!(ThreatType::HiddenUnicode.severity(), 5);
        assert_eq!(ThreatType::PromptInjection.severity(), 4);
        assert_eq!(ThreatType::ConcealInfo.severity(), 3);
    }

    #[test]
    fn test_threat_type_description() {
        assert!(ThreatType::PromptInjection.description().contains("注入"));
        assert!(ThreatType::ReadSecrets.description().contains("敏感文件"));
    }
}
