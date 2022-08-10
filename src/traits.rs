use anyhow::Result;
use connection_utils::Channel;

pub trait TUpgradableChannel: Channel {
    // fn is_upgrdaded(&self) -> bool;
    fn upgrade(&mut self, new_channel: Box<dyn Channel>) -> Result<()>;
}
