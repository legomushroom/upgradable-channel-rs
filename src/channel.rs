use std::pin::Pin;

use cs_utils::random_str;
use connection_utils::Channel;

mod channel_message;
pub use channel_message::ChannelMessage;

use crate::interleaved_channel::divide_channel;

mod implementations;

pub struct UpgradableChannel {
    sync_id: String,
    channel1: Pin<Box<dyn Channel>>,
    channel_msg: Option<Pin<Box<dyn Channel>>>,
    channel2: Option<Pin<Box<dyn Channel>>>,
    is_upgraded_reads: bool,
    is_upgraded_writes: bool,
    channel2_buffer: Vec<u8>,
}

impl UpgradableChannel {
    pub fn new(
        channel: Box<dyn Channel>,
    ) -> UpgradableChannel {
        let (channel, channel_msg) = divide_channel(channel);

        return UpgradableChannel {
            sync_id: random_str(32),
            channel1: Pin::new(channel),
            channel_msg: Some(Pin::new(channel_msg)),
            channel2: None,
            is_upgraded_reads: false,
            is_upgraded_writes: false,
            channel2_buffer: vec![],
        };
    }
}
