# 故障排除指南

> 常见问题和解决方案

## 快速诊断

### 第一步：检查环境

```bash
# 检查 Rust 版本
rustc --version  # 需要 1.70+

# 检查 Python（如果使用 Skills）
python3 --version  # 需要 Python 3.x

# 检查环境变量
echo $OPENAI_API_KEY  # 或 OPENAI_BASE_URL
```

### 第二步：查看错误类型

| 错误类型 | 说明 | 解决方案 |
|----------|------|----------|
| `SkillLoadError` | 技能加载失败 | 检查配置文件格式 |
| `SkillExecuteError` | 技能执行失败 | 检查脚本和环境 |
| `AgentError` | Agent 错误 | 检查配置和输入 |
| `ProviderError` | Provider 错误 | 检查 API Key 和网络 |

## 常见问题

### Q1: "未找到脚本实现" 错误

**错误信息**:
```
无法执行技能 "weather-query"
原因：未找到脚本实现文件
技能目录：skills/weather-query
期望文件：SKILL.py, SKILL.js, SKILL.sh 之一
```

**原因**:
- 技能目录中缺少脚本文件
- 脚本文件命名不正确

**解决方案**:

1. 创建脚本文件
```bash
mkdir -p skills/weather-query
cat > skills/weather-query/SKILL.py << 'EOF'
#!/usr/bin/env python3
import sys
import json

try:
    input_data = json.loads(sys.stdin.read())
    city = input_data.get("city", "Beijing")
    print(json.dumps({
        "success": True,
        "city": city,
        "weather": "Sunny +25°C"
    }, ensure_ascii=False))
except Exception as e:
    print(json.dumps({
        "success": False,
        "error": str(e)
    }, ensure_ascii=False))
EOF
```

2. 设置执行权限
```bash
chmod +x skills/weather-query/SKILL.py
```

3. 验证文件存在
```bash
ls -la skills/weather-query/
```

### Q2: "OPENAI_API_KEY 未设置" 错误

**错误信息**:
```
⚠ 未设置 OPENAI_API_KEY
   请运行：export OPENAI_API_KEY=sk-your-key
```

**解决方案**:

**使用 OpenAI**:
```bash
export OPENAI_API_KEY=sk-your-key
# 验证
echo $OPENAI_API_KEY  # 应该显示 sk-xxx
```

**使用 Ollama（本地）**:
```bash
export OPENAI_BASE_URL=http://localhost:11434
# 验证 Ollama 运行
curl http://localhost:11434/api/tags
```

**在代码中设置**:
```rust
use rucora::provider::OpenAiProvider;

// 方式 1：从环境变量
let provider = OpenAiProvider::from_env()?;

// 方式 2：直接指定
let provider = OpenAiProvider::new("sk-your-key")?;

// 方式 3：使用自定义地址
let provider = OpenAiProvider::with_base_url(
    "http://localhost:11434",
    None  // 不需要 API Key
)?;
```

### Q3: 技能加载失败

**错误信息**:
```
跳过技能 "skills/ai_news": 解析错误：SKILL.md 必须以 --- 开始
```

**原因**:
- SKILL.md 的 YAML Frontmatter 格式不正确
- 配置文件语法错误

**解决方案**:

**正确的 SKILL.md 格式**:
```markdown
---
name: ai_news
description: AI 新闻聚合技能
version: 1.0.0
tags:
  - news
  - ai
---

# AI 新闻技能说明

这里是技能的详细说明...
```

**或使用 skill.yaml（推荐）**:
```yaml
skill:
  name: ai_news
  description: AI 新闻聚合技能
  version: 1.0.0
  tags:
    - news
    - ai

input_schema:
  type: object
  properties:
    limit:
      type: integer
      default: 10
```

**验证配置文件**:
```bash
# 检查 YAML 语法
python3 -c "import yaml; yaml.safe_load(open('skills/ai_news/skill.yaml'))"

# 检查 JSON 语法
python3 -c "import json; json.load(open('skills/ai_news/skill.json'))"
```

### Q4: 工具调用失败

**错误信息**:
```
工具执行错误：tool error: 脚本输出为空
```

**原因**:
- 脚本没有输出
- 脚本输出不是有效的 JSON
- 脚本执行超时

**解决方案**:

1. 确保脚本输出 JSON
```python
#!/usr/bin/env python3
import sys
import json

try:
    # 读取输入
    input_data = json.loads(sys.stdin.read())
    
    # 处理逻辑
    result = {"success": True, "data": "result"}
    
    # 输出 JSON
    print(json.dumps(result, ensure_ascii=False))
    
except Exception as e:
    # 错误也要输出 JSON
    print(json.dumps({
        "success": False,
        "error": str(e)
    }, ensure_ascii=False))
```

2. 增加超时时间
```yaml
# skill.yaml
execution:
  timeout: 60  # 增加到 60 秒
```

3. 调试脚本
```bash
# 手动测试脚本
echo '{"city": "Beijing"}' | python3 skills/weather-query/SKILL.py

# 查看输出
# 应该看到：{"success": true, "city": "Beijing", ...}
```

### Q5: Python 启动失败

**错误信息**:
```
启动 Python 失败：The system cannot find the file specified.
请确保 Python 已安装并添加到 PATH
```

**解决方案**:

**Windows**:
```powershell
# 检查 Python 是否安装
python --version
python3 --version

# 如果未安装，从 python.org 安装
# 安装时勾选 "Add Python to PATH"

# 或者使用 py 命令
py --version
```

**修改代码使用正确的命令**:
```rust
// 在 loader.rs 中
let python_cmd = if cfg!(windows) { "python" } else { "python3" };
```

**Linux/Mac**:
```bash
# 安装 Python
# Ubuntu/Debian
sudo apt-get install python3

# CentOS/RHEL
sudo yum install python3

# Mac
brew install python3
```

### Q6: 依赖包缺失

**错误信息**:
```
ModuleNotFoundError: No module named 'requests'
```

**解决方案**:

```bash
# 安装依赖
pip3 install requests

# 或使用 requirements.txt
pip3 install -r requirements.txt
```

**在技能配置中声明依赖**:
```yaml
# skill.yaml
dependencies:
  python_packages:
    - requests
    - beautifulsoup4
```

### Q7: 网络连接失败

**错误信息**:
```
URLError: <urlopen error [Errno 11001] getaddrinfo failed>
```

**解决方案**:

1. 检查网络连接
```bash
ping wttr.in
curl https://wttr.in/Beijing
```

2. 检查防火墙设置
```bash
# Windows
netsh advfirewall show allprofiles

# Linux
sudo ufw status
```

3. 使用代理（如果需要）
```python
import urllib.request

proxies = {
    'http': 'http://proxy.example.com:8080',
    'https': 'https://proxy.example.com:8080',
}

opener = urllib.request.ProxyHandler(proxies)
```

### Q8: 权限不足

**错误信息**:
```
PermissionError: [Errno 13] Permission denied
```

**解决方案**:

```bash
# 设置脚本执行权限
chmod +x skills/weather-query/SKILL.py

# 设置目录权限
chmod 755 skills/weather-query

# Windows（管理员运行）
# 右键 -> 以管理员身份运行
```

## 调试技巧

### 启用详细日志

```rust
use tracing_subscriber::{FmtSubscriber, Level};

let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::DEBUG)  // 或 Level::TRACE
    .with_target(true)
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

### 查看工具调用

```rust
// 在 Agent 创建前
info!("可用工具：");
for tool in &tools {
    info!("  - {}: {}", tool.name(), tool.description().unwrap_or("无描述"));
}
```

### 检查技能配置

```bash
# 查看配置文件
cat skills/weather-query/skill.yaml

# 检查脚本文件
ls -la skills/weather-query/

# 测试脚本
echo '{"city": "Beijing"}' | python3 skills/weather-query/SKILL.py
```

### 使用调试器

```bash
# Rust 调试
cargo run --example hello_world 2>&1 | tee debug.log

# Python 调试
python3 -m pdb skills/weather-query/SKILL.py
```

## 获取帮助

### 第一步：查看文档

- [快速开始](quick_start.md)
- [用户指南](user_guide.md)
- [示例集合](cookbook.md)
- [常见问题](faq.md)

### 第二步：检查日志

```bash
# 保存日志
cargo run 2>&1 | tee app.log

# 搜索错误
grep -i error app.log
```

### 第三步：最小化复现

创建一个最小的复现代码：

```rust
// minimal.rs
use rucora::provider::OpenAiProvider;
use rucora::agent::DefaultAgent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .build();
    agent.run("你好").await?;
    Ok(())
}
```

### 第四步：提交 Issue

提供以下信息：

1. **环境信息**
   - Rust 版本
   - 操作系统
   - Python 版本（如果使用 Skills）

2. **错误信息**
   - 完整的错误输出
   - 日志文件

3. **复现步骤**
   - 最小化复现代码
   - 操作步骤

## 性能优化

### 减少技能加载时间

```rust
// 使用缓存
use rucora::skills::{SkillCache, CachedSkillLoader};
use std::time::Duration;

let cache = SkillCache::new(100, Some(Duration::from_secs(600)));
let mut loader = CachedSkillLoader::new(skills_dir, cache);
```

### 优化技能执行

```yaml
# skill.yaml
execution:
  timeout: 10      # 设置合理的超时
  cache: true      # 启用缓存
  retries: 1       # 设置重试次数
```

### 减少内存占用

```rust
// 按需加载配置
use rucora::skills::config::{SkillConfig, ConfigLoadOptions};

let options = ConfigLoadOptions::for_search();  // 只加载搜索信息
let config = SkillConfig::from_dir_with_options(&path, &options)?;
```

## 相关文档

- [Skill 配置规范](skill_yaml_spec.md)
- [Skill 配置示例](skill_yaml_examples.md)
- [Hello World 示例](../examples/hello_world.rs)
