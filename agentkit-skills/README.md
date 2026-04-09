# AgentKit Skills

Skills system for AgentKit.

## Overview

This crate provides the Skills system for AgentKit, enabling:
- YAML-based skill definitions
- Command template skills
- File operation skills
- Rhai script skills (optional)
- Skill loading from directories
- Skill to Tool adapter

## Installation

```toml
[dependencies]
agentkit-skills = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["skills"] }
```

## Usage

### Load Skills from Directory

```rust
use agentkit_skills::{SkillLoader, SkillExecutor, SkillTool};
use agentkit_tools::ToolRegistry;
use std::sync::Arc;

// Load skills from directory
let mut loader = SkillLoader::new("skills/");
let skills = loader.load_from_dir().await?;

// Create executor
let executor = Arc::new(SkillExecutor::new());

// Register skills as tools
let mut registry = ToolRegistry::new();
for skill in &skills {
    let tool = SkillTool::new(skill.clone(), executor.clone(), "skills/");
    registry = registry.register_arc(Arc::new(tool));
}
```

### Skill YAML Format

```yaml
name: my_skill
description: A custom skill for doing something useful
version: "1.0"

trigger:
  keywords:
    - "do something"
    - "help me"

parameters:
  - name: input
    type: string
    required: true
    description: The input to process

execution:
  type: command
  template: "echo {{input}}"
```

### Skills to Tools

```rust
use agentkit_skills::skills_to_tools;

// Convert all skills to tools
let tools = skills_to_tools(&skills, executor, "skills/");

// Use with agent
let agent = ToolAgent::builder()
    .provider(provider)
    .tools(tools)
    .build();
```

## Features

| Feature | Description |
|---------|-------------|
| `yaml` | YAML skill file support |
| `all` | Enable all features |

## Submodules

- `cache`: Skill caching
- `command_skills`: Command-based skills
- `config`: Skill configuration
- `file_skills`: File operation skills
- `integrator`: Skill integration utilities
- `loader`: Skill loading from directories
- `rhai_skills`: Rhai script skills
- `tool_adapter`: Skill to Tool adapter

## License

MIT
