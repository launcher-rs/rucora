# Windows Python 执行修复

## 问题

在 Windows 上，`python3` 命令不存在，导致脚本执行失败。

## 修复

### 1. Python 命令自动选择

```rust
async fn execute_python(
    &self,
    script_path: &Path,
    input: &Value,
    timeout: u64,
) -> Result<SkillResult, SkillExecuteError> {
    // 尝试 python 或 python3
    let python_cmd = if cfg!(windows) { "python" } else { "python3" };
    
    let mut child = Command::new(python_cmd)
        .arg(script_path)
        // ...
}
```

### 2. Node.js 命令自动选择

```rust
async fn execute_javascript(
    &self,
    script_path: &Path,
    input: &Value,
    timeout: u64,
) -> Result<SkillResult, SkillExecuteError> {
    // 尝试 node 或 nodejs
    let node_cmd = if cfg!(windows) { "node" } else { "nodejs" };
    
    let mut child = Command::new(node_cmd)
        .arg(script_path)
        // ...
}
```

## 修改的文件

- `agentkit/src/skills/loader.rs` - 添加平台特定的命令选择

## 验证

```bash
cargo check --workspace
# ✅ Finished
```

## 测试命令

Windows:
```bash
echo '{"city": "Beijing"}' | python skills/weather/SKILL.py
```

Linux/Mac:
```bash
echo '{"city": "Beijing"}' | python3 skills/weather/SKILL.py
```

## 预期输出

```json
{
  "success": true,
  "city": "Beijing",
  "weather": "Sunny +25°C",
  "message": "Beijing 的天气：Sunny +25°C"
}
```
