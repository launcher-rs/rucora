# TrendRadar 项目研究文档

## 项目概述

TrendRadar 是一个 Python 编写的热点追踪与分析系统，主要功能包括：
- 多平台热榜数据抓取（微博、知乎、抖音、B站等）
- RSS 订阅源聚合
- AI 驱动的热点分析与深度洞察
- 多渠道报告推送（飞书、钉钉、Telegram、邮件等）
- MCP 协议支持，可作为工具服务提供给其他 Agent

## 可借鉴的设计模式

### 1. 统一的 AI 客户端 (LiteLLM)

**位置**: `trendradar/ai/client.py`

```python
class AIClient:
    def __init__(self, config: Dict[str, Any]):
        self.model = config.get("MODEL", "deepseek/deepseek-chat")
        self.api_key = config.get("API_KEY")
        self.api_base = config.get("API_BASE", "")
        # ...

    def chat(self, messages: List[Dict[str, str]], **kwargs) -> str:
        response = completion(**params)
        return response.choices[0].message.content
```

**借鉴点**:
- 使用 LiteLLM 统一不同 AI 提供商的接口
- 支持 fallback 模型配置
- 统一的 chat 接口，屏蔽底层差异

**对 rucora 的建议**: 可以考虑集成 LiteLLM 作为 provider 的统一底层，支持更多模型。

---

### 2. Prompt 模板加载器

**位置**: `trendradar/ai/prompt_loader.py`

```python
def load_prompt_template(
    prompt_file: str,
    config_subdir: str = "",
    label: str = "AI",
) -> Tuple[str, str]:
    """加载 [system] / [user] 格式的提示词文件"""
    content = prompt_path.read_text(encoding="utf-8")
    # 解析 [system] 和 [user] 部分
    return system_prompt, user_prompt
```

**借鉴点**:
- 将 prompt 独立为配置文件
- 支持 [system] 和 [user] 分离
- 便于非开发人员调整 prompt

**对 rucora 的建议**: 可以为 DeepResearch 等模块添加 prompt 配置文件支持。

---

### 3. MCP Server 实现

**位置**: `mcp_server/`

TrendRadar 提供了完整的 MCP Server 实现，包括：
- `server.py`: MCP 协议服务器
- `tools/`: 各种工具实现（搜索、分析、通知等）
- `services/`: 数据服务层
- `utils/`: 工具函数

**核心工具**:
- `search_tools.py`: 智能新闻检索（支持关键词、模糊、实体搜索）
- `notification.py`: 多渠道通知推送
- `analytics.py`: 数据分析工具
- `storage_sync.py`: 存储同步

**借鉴点**:
- 清晰的 tool 定义规范
- 错误处理和参数验证
- 统一的工具注册和调用机制

**对 rucora 的建议**: rucora-mcp 模块可以参考其工具定义和错误处理方式。

---

### 4. 多渠道通知系统

**位置**: `mcp_server/tools/notification.py`

支持的通知渠道：
- 飞书 Webhook
- 钉钉 Webhook
- 企业微信
- Telegram Bot
- 邮件 (SMTP)
- NTFY
- Bark
- Slack
- 通用 Webhook

```python
_CHANNEL_REQUIREMENTS = {
    "feishu": ["FEISHU_WEBHOOK_URL"],
    "dingtalk": ["DINGTALK_WEBHOOK_URL"],
    "telegram": ["TELEGRAM_BOT_TOKEN", "TELEGRAM_CHAT_ID"],
    # ...
}
```

**借鉴点**:
- 渠道配置自动检测
- 统一的消息格式转换
- 支持 markdown 到各渠道格式的转换

**对 rucora 的建议**: 可以为 rucora-tools 添加统一的通知工具。

---

### 5. 模块化架构

TrendRadar 的模块划分：

```
trendradar/
├── ai/              # AI 分析模块
│   ├── analyzer.py    # 分析器
│   ├── client.py      # AI 客户端
│   ├── filter.py      # 智能筛选
│   ├── translator.py  # 翻译器
│   └── formatter.py   # 格式化
├── core/            # 核心模块
│   ├── analyzer.py    # 数据分析
│   ├── config.py      # 配置管理
│   ├── loader.py     # 配置加载
│   └── scheduler.py   # 调度器
├── crawler/         # 爬虫模块
├── report/          # 报告生成
├── storage/         # 存储模块
└── notification/    # 通知模块
```

**借鉴点**:
- 清晰的模块边界
- 统一的配置管理
- 功能模块间低耦合

---

### 6. 数据结构设计

**AIAnalysisResult** (`trendradar/ai/analyzer.py`):

```python
@dataclass
class AIAnalysisResult:
    core_trends: str              # 核心热点与舆情态势
    sentiment_controversy: str    # 舆论风向与争议
    signals: str                   # 异动与弱信号
    rss_insights: str             # RSS 深度洞察
    outlook_strategy: str         # 研判与策略建议
    standalone_summaries: Dict[str, str]  # 独立展示区概括
    # 元数据
    success: bool
    error: str
    total_news: int
    analyzed_news: int
```

**借鉴点**:
- 使用 dataclass 定义结构化结果
- 清晰的字段命名和文档
- 分离原始响应和解析结果

---

## 总结

TrendRadar 项目在以下方面值得 rucora 借鉴：

| 特性 | 借鉴内容 | 实现难度 |
|------|----------|----------|
| LiteLLM 集成 | 统一多 provider 接口 | 中 |
| Prompt 配置文件 | 独立 prompt 便于调整 | 低 |
| MCP 工具定义 | 标准化的 tool 规范 | 中 |
| 多渠道通知 | 统一的通知工具 | 中 |
| 模块化设计 | 清晰的模块边界 | 低 |
| 数据结构 | dataclass 定义结果 | 低 |

## 优先级建议

1. **高优先级**: Prompt 配置文件支持（快速实现，效果明显）
2. **中优先级**: 扩展 provider 支持（LiteLLM 集成）
3. **中优先级**: 多渠道通知工具
4. **低优先级**: MCP 工具规范参考