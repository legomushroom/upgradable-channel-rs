use std::{pin::Pin, sync::{Arc, Mutex}, task::Waker};

use cs_utils::futures::with_thread;
use tokio::sync::{oneshot::{self, Sender}, mpsc};
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
    last_read_waker: Arc<Mutex<Option<Waker>>>,
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

        // let (
        //     channel2_buffer_sender,
        //     channel2_buffer,
        // ) = mpsc::channel(100);

        let last_read_waker = Arc::new(Mutex::new(None));

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
                    Arc::clone(&last_read_waker),
                    // channel2_buffer_sender,
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
                    last_read_waker,
                    // channel2_buffer,
                },
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use cs_utils::{traits::Random, random_str_rg, futures::wait_random};
    use rstest::rstest;
    use connection_utils::test::test_async_stream;

    use crate::{mocks::{ChannelMockOptions, channel_mock_pair}, UpgradableChannel};

    #[rstest]
    #[case(random_str_rg(64000..=64096))]
    #[tokio::test]
    async fn upgrades_to_a_new_channel(
        #[case] test_data: String,
    ) {
        let options1 = ChannelMockOptions::random();
        let options2 = ChannelMockOptions::random();

        let (local_channel1, remote_channel1) = channel_mock_pair(options1.clone(), options1.clone());
        let (local_channel2, remote_channel2) = channel_mock_pair(options2.clone(), options2.clone());

        let (on_local_channel1, local_upgradable_channel1) = UpgradableChannel::new("local", local_channel1);
        let (on_remote_channel1, remote_upgradable_channel1) = UpgradableChannel::new("remote", remote_channel1);

        let _res = tokio::join!(
            Box::pin(async move {
                println!("> starting data transfer");

                test_async_stream(
                    local_upgradable_channel1,
                    remote_upgradable_channel1,
                    test_data,
                ).await;

                println!("> data transfer complete");
            }),
            Box::pin(async move {
                wait_random(1..=25).await;

                on_local_channel1.send(local_channel2)
                    .map_err(|_| { return anyhow!("[local] Cannot send new channel notification."); })
                    .unwrap();

                println!("> local upgrade sent");
            }),
            Box::pin(async move {
                wait_random(1..=25).await;

                on_remote_channel1.send(remote_channel2)
                    .map_err(|_| { return anyhow!("[remote] Cannot send new channel notification."); })
                    .unwrap();

                println!("> remote upgrade sent");
            }),
        );

        println!("> done");
    }
}