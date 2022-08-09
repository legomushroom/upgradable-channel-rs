use anyhow::Result;
use async_trait::async_trait;
use connection_utils::Channel;

#[async_trait]
pub trait TUpgradableChannel: Channel {
    fn is_upgrdaded(&self) -> bool;
    async fn upgrade(&mut self, new_channel: Box<dyn Channel>) -> Result<()>;
}
