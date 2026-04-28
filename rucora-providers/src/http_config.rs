//! HTTP 客户端配置的共享常量和工具函数。
//!
//! 该模块提供：
//! - 默认的请求超时配置
//! - 连接超时配置
//! - HTTP 客户端构建工具

use std::time::Duration;

use reqwest::header::HeaderMap;

/// 默认请求超时时间（秒）
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;

/// 默认连接超时时间（秒）
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 15;

/// 构建带有超时配置的 HTTP 客户端
///
/// # 参数
///
/// - `headers`: 默认的 HTTP 请求头
///
/// # 返回
///
/// 配置好超时的 `reqwest::Client` 实例
pub fn build_client(headers: HeaderMap) -> reqwest::Client {
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .build()
        .expect("reqwest client build failed")
}

/// 构建带有自定义超时的 HTTP 客户端
///
/// # 参数
///
/// - `headers`: 默认的 HTTP 请求头
/// - `request_timeout_secs`: 请求超时时间（秒）
/// - `connect_timeout_secs`: 连接超时时间（秒）
///
/// # 返回
///
/// 配置好超时的 `reqwest::Client` 实例
pub fn build_client_with_timeout(
    headers: HeaderMap,
    request_timeout_secs: u64,
    connect_timeout_secs: u64,
) -> reqwest::Client {
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(request_timeout_secs))
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .build()
        .expect("reqwest client build failed")
}
