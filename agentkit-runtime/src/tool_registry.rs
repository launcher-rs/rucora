use std::collections::HashMap;
use std::sync::Arc;

use agentkit_core::tool::Tool;
use agentkit_core::tool::types::ToolDefinition;

/// Tool 注册表：集中管理所有可用 tools。
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
        self
    }

    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.insert(tool.name().to_string(), tool);
        self
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}
