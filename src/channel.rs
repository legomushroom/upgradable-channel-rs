use std::pin::Pin;

use connection_utils::Channel;
use cs_utils::random_str;

mod channel_message;
pub use channel_message::ChannelMessage;

mod implementations;

pub struct UpgradableChannel {
    sync_id: String,
    channel1: Pin<Box<dyn Channel>>,
    channel1_msg: Option<Pin<Box<dyn Channel>>>,
    channel2: Option<Pin<Box<dyn Channel>>>,
    is_upgraded_reads: bool,
    is_upgraded_writes: bool,
    channel2_buffer: Vec<u8>,
}

// TODO: create `channel1_msg` out of `channel1`

impl UpgradableChannel {
    pub fn new(
        channel: Box<dyn Channel>,
        channel1_msg: Box<dyn Channel>, // TODO: temporary
    ) -> UpgradableChannel {
        return UpgradableChannel {
            sync_id: random_str(32),
            channel1: Pin::new(channel),
            channel1_msg: Some(Pin::new(channel1_msg)),
            channel2: None,
            is_upgraded_reads: false,
            is_upgraded_writes: false,
            channel2_buffer: vec![],
        };
    }
}
