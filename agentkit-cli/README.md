# agentkit-cli（官方 CLI，可选）

`agentkit-cli` 是用于**快速试用**的命令行工具：加载配置、加载 skills、执行 agent loop，并可选导出 trace。

## 安装/运行

推荐直接在 workspace 内运行：

```bash
cargo run -p agentkit-cli -- run --skill-dir skills --prompt "用一句话介绍 Rust" 
```

## 子命令

### `agentkit run`

参数：

- `--skill-dir <path>`：skills 目录（默认 `skills`）
- `--prompt <text>`：用户问题
- `--max-steps <n>`：最大 tool loop 步数
- `--stream <true|false>`：是否使用 streaming（默认 true）
- `--trace-path <file>`：可选导出 trace JSONL

配置来源：参考 `docs/cookbook_config.md`。
