//! Web 工具模块
//!
//! 提供网页浏览、HTTP 请求、Web 搜索等功能

pub mod browse;
pub mod fetch;
pub mod http;
pub mod search;

pub use browse::{BrowseTool, BrowserOpenTool};
pub use fetch::WebFetchTool;
pub use http::HttpRequestTool;
pub use search::{GithubTrendingTool, SerpapiTool, TavilyTool};
