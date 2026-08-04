//! #[rucora_guard] 宏集成测试：验证生成代码能匹配真实的 InjectionGuard trait。

use rucora_core::{InjectionGuard, ScanResult};
use rucora_macros::rucora_guard;

/// 简单的长度限制守卫
#[rucora_guard(name = "length-limit")]
fn check_length(content: &str, source: &str) -> ScanResult {
    let _ = source;
    let too_long = content.len() > 5;
    ScanResult {
        is_safe: !too_long,
        threats: vec![],
        cleaned_content: None,
        original_length: content.len(),
    }
}

#[test]
fn guard_macro_generates_working_impl() {
    let guard = LengthLimitGuard;

    let safe = guard.scan("abc", "test");
    assert!(safe.is_safe);
    assert_eq!(safe.original_length, 3);

    let blocked = guard.scan("this is too long", "test");
    assert!(!blocked.is_safe);
    assert_eq!(blocked.original_length, 16);
}
