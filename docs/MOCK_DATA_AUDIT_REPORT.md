# AgentKit 模拟数据审计与改进计划

> **审计日期**: 2026年4月14日  
> **触发事件**: `agentkit-deep-research` 运行后报告提示"搜索工具返回的是模拟数据"  
> **审计目标**: 识别所有使用模拟数据、占位符的工具和代码，制定改进计划

---

## 一、问题发现

在运行 `agentkit-deep-research` 示例研究"中东局势"时，生成的报告中出现了以下提示：

> "由于搜索工具返回的是模拟数据，本报告基于综合知识整理而成。实际应用中建议结合实时新闻来源。"

经排查，根因在于 **`WebSearchTool` 和 `WebScraperTool` 是完全的模拟实现**，返回硬编码的假数据。

---

## 二、模拟数据清单

### 🔴 高优先级：核心工具模拟实现

#### 1. WebSearchTool - 完全模拟

| 属性 | 详情 |
|------|------|
| **文件** | `agentkit-tools/src/web_search.rs` |
| **行号** | 第 43-75 行 |
| **方法** | `async fn search(&self, query: &str)` |
| **状态** | 🔴 完全模拟，返回硬编码假数据 |

**问题代码**:
```rust
/// 执行搜索（模拟实现）
async fn search(&self, query: &str) -> Result<Vec<SearchResult>, ToolError> {
    // 模拟搜索结果
    // 真实实现中可以调用 Google Custom Search API、Bing Search API 等
    let mock_results = vec![
        SearchResult {
            title: format!("{query} - 官方文档"),
            url: "https://example.com/official".to_string(),
            snippet: format!("{query}的官方文档，提供详细的技术说明和 API 参考。"),
        },
        SearchResult {
            title: format!("{query} 入门教程"),
            url: "https://example.com/tutorial".to_string(),
            snippet: format!("{query}的完整入门教程，适合初学者系统学习。"),
        },
        // ... 更多 example.com 的假数据
    ];

    Ok(mock_results.into_iter().take(self.max_results).collect())
}
```

**影响范围**:
- 所有使用 `WebSearchTool` 的功能都返回假数据
- `agentkit-deep-research` 示例的核心功能受损
- 任何依赖搜索的 Agent 都会收到虚假信息

---

#### 2. WebScraperTool - 完全模拟

| 属性 | 详情 |
|------|------|
| **文件** | `agentkit-tools/src/web_search.rs` |
| **行号** | 第 221-235 行 |
| **方法** | `async fn call(&self, input: Value)` |
| **状态** | 🔴 完全模拟，返回硬编码假网页内容 |

**问题代码**:
```rust
// 模拟抓取结果
// 真实实现中可以使用 reqwest 等库抓取网页
let mock_content = format!(
    r#"网页内容摘要：
URL: {url}
标题：示例网页
内容：这是一个模拟的网页内容。在实际实现中，这里会显示从网络抓取的真实网页内容。
可以使用 reqwest 库发送 HTTP 请求，然后解析 HTML 获取所需信息。"#
);

Ok(json!({
    "url": url,
    "content": mock_content,
    "success": true
}))
```

**影响范围**:
- 所有使用 `WebScraperTool` 抓取网页的功能都返回假数据
- 无法获取真实网页内容

---

### 🟡 中优先级：测试和示例中的 Mock

#### 3. MockProvider - 测试和示例用

| 属性 | 详情 |
|------|------|
| **文件** | 多个位置 |
| **状态** | 🟡 合理的测试实践，无需修复 |

**位置清单**:

| 文件路径 | 用途 |
|---------|------|
| `agentkit/examples/10_custom_provider.rs` | 示例代码，演示如何自定义 Provider |
| `agentkit/src/agent/chat.rs` | 单元测试 |
| `agentkit/src/agent/react.rs` | 单元测试 |
| `agentkit/src/agent/tool.rs` | 单元测试 |
| `agentkit/src/agent/reflect.rs` | 单元测试 |
| `agentkit/src/agent/simple.rs` | 单元测试 |
| `agentkit-core/tests/contract_provider.rs` | 契约测试 |

**评估**: 这些 MockProvider 是**正常的设计模式**，用于测试和示例，不应视为问题。

---

### 🟢 低优先级：文档中的占位符

#### 4. example.com 引用

**位置**: 多处文档和注释中使用 `example.com` 作为示例 URL。

| 文件 | 用途 |
|------|------|
| `agentkit-tools/src/web.rs` | 文档注释示例 URL |
| `agentkit-tools/src/http.rs` | 文档注释示例 URL |
| `agentkit-tools/src/browser.rs` | 文档注释示例 URL |
| `docs/skill_yaml_spec.md` | 文档示例域名 |
| `docs/TROUBLESHOOTING.md` | 文档示例代理 URL |

**评估**: 仅用于文档和注释，不影响功能，无需修复。

---

## 三、真实工具清单（无需修改）

经过审计，以下工具**已有真实实现**，不使用模拟数据：

| 工具 | 文件 | 实现方式 |
|------|------|----------|
| `HttpRequestTool` | `agentkit-tools/src/http.rs` | 使用 `reqwest` 发送 HTTP 请求 |
| `WebFetchTool` | `agentkit-tools/src/web.rs` | 使用 `reqwest` 获取网页内容 |
| `BrowserOpenTool` | `agentkit-tools/src/browser.rs` | 调用系统默认浏览器 |
| `BrowseTool` | `agentkit-tools/src/browse.rs` | 支持 navigate/get_content |
| `TavilyTool` | `agentkit-tools/src/tavily_tool.rs` | 调用 Tavily Search API |
| `SerpapiTool` | `agentkit-tools/src/serpapi_tool.rs` | 调用 SerpAPI |
| `GithubTrendingTool` | `agentkit-tools/src/github_trending_tool.rs` | 爬取 GitHub 趋势页 |
| `EchoTool` | `agentkit-tools/src/echo.rs` | 回显输入（设计如此） |

---

## 四、改进计划

### 🟢 方案 A：实现真实的 WebSearchTool（推荐）

**目标**: 将 `WebSearchTool` 从模拟实现改为真实的搜索 API 集成。

**可选方案**:

| API | 成本 | 延迟 | 集成难度 | 推荐度 |
|-----|------|------|----------|--------|
| **Tavily API** | 免费额度 1000 次/月 | 低 | 低 | ⭐⭐⭐⭐⭐ |
| **SerpAPI** | 免费额度 100 次/月 | 中 | 低 | ⭐⭐⭐⭐ |
| **Google Custom Search** | 免费 100 次/天 | 低 | 中 | ⭐⭐⭐ |
| **Bing Search API** | 付费 | 低 | 中 | ⭐⭐⭐ |
| **DuckDuckGo (非官方)** | 免费 | 中 | 高 | ⭐⭐ |

**推荐方案**: 使用 **Tavily API** 作为默认实现，因为：
1. 项目已有 `TavilyTool` 实现，可参考
2. 专为 AI Agent 设计，返回结构化结果
3. 免费额度充足
4. API 简单，易于集成

**实施步骤**:
1. 在 `WebSearchTool` 中添加 Tavily API 集成
2. 添加 `TAVILY_API_KEY` 环境变量支持
3. 保留模拟实现作为 Fallback（当 API Key 未设置时）
4. 更新文档和示例

**预计工作量**: 2-3 小时

---

### 🟡 方案 B：实现真实的 WebScraperTool

**目标**: 将 `WebScraperTool` 从模拟实现改为真实的网页抓取。

**技术方案**:
1. 使用 `reqwest` 发送 HTTP GET 请求
2. 使用 `scraper` 或 `html2text` 解析 HTML
3. 提取网页正文内容
4. 处理编码和错误

**实施步骤**:
1. 实现 `fetch_url(url)` 方法
2. 添加 HTML 到文本的转换
3. 添加超时和错误处理
4. 更新文档

**预计工作量**: 3-4 小时

---

### 🔵 方案 C：使用已有工具替代

**目标**: 不修改 `WebSearchTool` 和 `WebScraperTool`，而是引导用户使用已有的真实工具。

**方案**:
1. 在 `WebSearchTool` 文档中注明："此为模拟实现，请使用 `TavilyTool` 或 `SerpapiTool` 进行真实搜索"
2. 在 `deep-research` 示例中默认使用 `TavilyTool` 而非 `WebSearchTool`
3. 为 `WebScraperTool` 添加类似的文档说明

**预计工作量**: 1 小时

---

## 五、推荐实施路线

### 第一阶段（立即实施）- 方案 C

1. 更新 `WebSearchTool` 和 `WebScraperTool` 的文档，明确标注"模拟实现"
2. 在 `agentkit-deep-research` 示例中改用 `TavilyTool` 和 `WebFetchTool`
3. 更新 README 说明

### 第二阶段（短期）- 方案 A

1. 为 `WebSearchTool` 集成 Tavily API
2. 添加 `TAVILY_API_KEY` 环境变量
3. 当 API Key 未设置时，回退到模拟实现并发出警告

### 第三阶段（中期）- 方案 B

1. 实现 `WebScraperTool` 的真实网页抓取功能
2. 使用 `reqwest` + `html2text`
3. 添加超时、重试、错误处理

---

## 六、总结

| 优先级 | 问题 | 影响 | 建议方案 | 工作量 |
|--------|------|------|----------|--------|
| 🔴 P0 | `WebSearchTool` 模拟数据 | 所有搜索功能返回假数据 | 方案 A: 集成 Tavily API | 2-3 小时 |
| 🔴 P0 | `WebScraperTool` 模拟数据 | 所有抓取功能返回假数据 | 方案 B: 实现真实抓取 | 3-4 小时 |
| 🟡 P1 | 文档标注不清 | 用户可能误以为是真实实现 | 方案 C: 更新文档 | 1 小时 |
| 🟢 P2 | MockProvider | 无影响（正常测试实践） | 无需修改 | - |

**核心建议**: 优先实施方案 C（文档更新），然后实施方案 A（Tavily 集成），最后实施方案 B（真实抓取）。这样可以确保用户立即知道如何正确使用真实工具，同时逐步完善内置工具的实现。

---

**报告编写时间**: 2026年4月14日  
**编写人员**: AI Code Assistant  
**下一步**: 根据推荐路线逐步实施改进
