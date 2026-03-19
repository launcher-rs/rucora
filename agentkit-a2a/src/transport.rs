use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use crate::protocol::{A2aMessage, AgentId};

#[async_trait]
pub trait A2aTransport: Send + Sync {
    async fn send(&self, to: &AgentId, msg: A2aMessage) -> Result<(), String>;
    async fn register(&self, id: AgentId) -> Result<mpsc::Receiver<A2aMessage>, String>;
}

#[derive(Default)]
pub struct InProcessA2aTransport {
    inner: Arc<Mutex<HashMap<String, mpsc::Sender<A2aMessage>>>>,
}

impl InProcessA2aTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl A2aTransport for InProcessA2aTransport {
    async fn send(&self, to: &AgentId, msg: A2aMessage) -> Result<(), String> {
        let map = self.inner.lock().await;
        let tx = map
            .get(&to.0)
            .ok_or_else(|| format!("agent not registered: {}", to.0))?;
        tx.send(msg)
            .await
            .map_err(|_| "receiver dropped".to_string())
    }

    async fn register(&self, id: AgentId) -> Result<mpsc::Receiver<A2aMessage>, String> {
        let (tx, rx) = mpsc::channel(128);
        let mut map = self.inner.lock().await;
        if map.contains_key(&id.0) {
            return Err(format!("agent already registered: {}", id.0));
        }
        map.insert(id.0, tx);
        Ok(rx)
    }
}
