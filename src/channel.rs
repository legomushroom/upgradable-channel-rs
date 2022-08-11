use std::{pin::Pin, sync::{Arc, Mutex}};

use cs_utils::{futures::with_thread, random_str};
use tokio::sync::{oneshot::{self, Sender}, mpsc::{self, Receiver}};
use connection_utils::Channel;

mod channel_message;
pub use channel_message::ChannelMessage;

use crate::{interleaved_channel::divide_channel, types::{TReadHalf, TWriteHalf}};

use self::implementations::handle_upgrade;

mod implementations;

pub struct UpgradableChannel {
    id: String,
    main_channel: Pin<Box<dyn Channel>>,
    channel2_reader: Arc<Mutex<Option<TReadHalf>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    // channel2_buffer: Receiver<Vec<u8>>,
}

impl UpgradableChannel {
    pub fn new(
        id: impl AsRef<str> + ToString,
        main_channel: Box<dyn Channel>,
    ) -> (Sender<Box<dyn Channel>>, Box<dyn Channel>) {
        let id = id.to_string();
        let (main_channel, control_channel) = divide_channel(main_channel);

        let channel2_reader = Arc::new(Mutex::new(None));
        let channel2_writer = Arc::new(Mutex::new(None));

        let (
            new_channel_sender,
            new_channel_receiver,
        ) = oneshot::channel();

        let (
            channel2_buffer_sender,
            channel2_buffer,
        ) = mpsc::channel(100);

        // let id = random_str(4);

        // TODO: return handle or add `on_error` notification
        let _handle = tokio::spawn(
            with_thread(
                handle_upgrade(
                    id.clone(),
                    new_channel_receiver,
                    control_channel,
                    Arc::clone(&channel2_reader),
                    Arc::clone(&channel2_writer),
                    channel2_buffer_sender,
                ),
            ),
        );

        return (
            new_channel_sender,
            Box::new(
                UpgradableChannel {
                    id,
                    main_channel: Pin::new(main_channel),
                    channel2_reader,
                    channel2_writer,
                    // channel2_buffer,
                },
            ),
        );
    }
}
