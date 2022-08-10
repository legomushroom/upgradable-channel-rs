use std::{pin::Pin, sync::Arc};

use tokio::sync::Mutex;
use cs_utils::random_str;
use connection_utils::Channel;

mod channel_message;
pub use channel_message::ChannelMessage;

use crate::{interleaved_channel::divide_channel, types::{TReadHalf, TWriteHalf}};

mod implementations;

pub struct UpgradableChannel {
    sync_id: String,
    channel1: Pin<Box<dyn Channel>>,
    channel_msg: Option<Pin<Box<dyn Channel>>>,
    channel2_reader: Arc<Mutex<Option<TReadHalf>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
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
            channel2_buffer: vec![],
            channel2_reader: Arc::new(Mutex::new(None)),
            channel2_writer: Arc::new(Mutex::new(None)),
        };
    }
}
