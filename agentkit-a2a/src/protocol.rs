use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aTask {
    pub id: TaskId,
    pub from: AgentId,
    pub to: AgentId,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aResult {
    pub id: TaskId,
    pub from: AgentId,
    pub to: AgentId,
    pub output: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aCancel {
    pub id: TaskId,
    pub from: AgentId,
    pub to: AgentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2aEvent {
    pub id: TaskId,
    pub from: AgentId,
    pub to: AgentId,
    pub event: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2aMessage {
    Task(A2aTask),
    Result(A2aResult),
    Event(A2aEvent),
    Cancel(A2aCancel),
}
