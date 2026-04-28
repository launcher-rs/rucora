use async_trait::async_trait;
use futures_util::stream::BoxStream;

use crate::{channel::types::ChannelEvent, error::ChannelError};

/// Channel（通信渠道）接口。
///
/// - `send`：向外部发送一个事件
/// - `stream`：订阅外部输入事件（例如用户消息）
#[async_trait]
pub trait Channel: Send + Sync {
    /// 发送事件到渠道。
    async fn send(&self, event: ChannelEvent) -> Result<(), ChannelError>;

    /// 订阅事件流。
    ///
    /// 约定：该流可以是”来自外部的输入”，也可以是”系统内部事件总线”。
    ///
    /// # Errors
    ///
    /// 当无法创建事件流时返回 [`ChannelError`]。
    fn stream(
        &self,
    ) -> Result<BoxStream<'static, Result<ChannelEvent, ChannelError>>, ChannelError>;
}
