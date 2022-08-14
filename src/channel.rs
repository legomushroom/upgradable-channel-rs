use std::{sync::{Arc, Mutex}, task::Waker};

use connection_utils::Channel;
use tokio::{sync::oneshot::{self, Sender}, io::split};

mod channel_message;
pub use channel_message::ChannelMessage;

use crate::{interleaved_channel::divide_channel, types::{TReadHalf, TWriteHalf}};

use self::implementations::handle_upgrade;

mod implementations;

pub struct UpgradableChannel {
    id: u16,
    test_id: String,
    label: String,
    // main_channel: Pin<Box<dyn Channel>>,
    main_channel_reader: Arc<tokio::sync::Mutex<TReadHalf>>,
    main_channel_writer: Arc<tokio::sync::Mutex<TWriteHalf>>,
    channel2_reader: Arc<Mutex<Option<(TReadHalf, Vec<u8>)>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    last_read_waker: Arc<Mutex<Option<Waker>>>,
}

impl UpgradableChannel {
    pub fn new(
        id: impl AsRef<str> + ToString,
        main_channel: Box<dyn Channel>,
    ) -> (Sender<Box<dyn Channel>>, Box<dyn Channel>) {
        let test_id = id.to_string(); // TODO: take the main channel id instead
        let id = main_channel.id();
        let label = main_channel.label().clone();

        let (main_channel, control_channel) = divide_channel(main_channel);

        let (main_channel_reader, main_channel_writer) = split(main_channel);

        let main_channel_reader = Arc::new(tokio::sync::Mutex::new(Box::pin(main_channel_reader)));
        let main_channel_writer = Arc::new(tokio::sync::Mutex::new(Box::pin(main_channel_writer)));

        let channel2_reader = Arc::new(Mutex::new(None));
        let channel2_writer = Arc::new(Mutex::new(None));

        let (
            new_channel_sender,
            new_channel_receiver,
        ) = oneshot::channel();

        let last_read_waker = Arc::new(Mutex::new(None));

        // let id = random_str(4);

        // TODO: return handle or add `on_error` notification
        let _handle = tokio::spawn(
            handle_upgrade(
                test_id.clone(),
                new_channel_receiver,
                control_channel,
                Arc::clone(&main_channel_reader),
                Arc::clone(&main_channel_writer),
                Arc::clone(&channel2_reader),
                Arc::clone(&channel2_writer),
                Arc::clone(&last_read_waker),
            ),
        );

        return (
            new_channel_sender,
            Box::new(
                UpgradableChannel {
                    id,
                    test_id,
                    label,
                    main_channel_reader,
                    main_channel_writer,
                    // main_channel: Pin::new(main_channel),
                    channel2_reader,
                    channel2_writer,
                    last_read_waker,
                },
            ),
        );
    }
}

static DATA_TRANSFER_CHARACTERS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ@#$%&*+=_";
static DATA_TRANSFER_DELIMITER: &str = "----";

fn add_data_transfer_string(
    start_string: impl AsRef<str> + ToString,
    len: usize,
    sequence_len: usize,
) -> String {
    let mut result = start_string.to_string();

    if result.len() >= len {
        result.truncate(len);
        return result;
    }

    for char in DATA_TRANSFER_CHARACTERS.chars() {
        for _ in 0..sequence_len {
            result = format!("{result}{char}");
        }
        result = format!("{result}{DATA_TRANSFER_DELIMITER}");

        if result.len() >= len {
            return add_data_transfer_string(result, len, sequence_len);
        }
    }

    return add_data_transfer_string(result, len, sequence_len);
}

pub fn data_transfer_string(
    len: usize,
) -> String {
    return add_data_transfer_string("", len, 64);
}


#[cfg(test)]
mod tests {
    use super::data_transfer_string;

    mod binary_data_transfer {
        use std::ops::RangeInclusive;

        use anyhow::anyhow;
        use cs_utils::{random_str_rg, traits::Random, futures::wait_random};
        use rstest::rstest;
        use connection_utils::test::test_async_stream;

        use super::data_transfer_string;
        use crate::{mocks::{ChannelMockOptions, channel_mock_pair}, UpgradableChannel};

        #[rstest]
        #[case(random_str_rg(100..=128))]
        #[case(random_str_rg(220..=256))]
        #[case(random_str_rg(500..=512))]
        #[case(random_str_rg(1_000..=1_024))]
        #[case(random_str_rg(2_000..=2_048))]
        #[case(random_str_rg(4_000..=4_096))]
        #[case(random_str_rg(8_000..=8_192))]
        #[case(random_str_rg(16_000..=16_384))]
        #[case(random_str_rg(30_000..=32_768))]
        #[case(random_str_rg(64_000..=65_536))]
        // #[case(data_transfer_string(8_192))]
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
                    test_async_stream(
                        local_upgradable_channel1,
                        remote_upgradable_channel1,
                        test_data,
                    ).await;
                }),
                Box::pin(async move {
                    wait_random(5..=25).await;

                    on_local_channel1.send(local_channel2)
                        .map_err(|_| { return anyhow!("[local] Cannot send new channel notification."); })
                        .unwrap();
                }),
                Box::pin(async move {
                    wait_random(5..=25).await;

                    on_remote_channel1.send(remote_channel2)
                        .map_err(|_| { return anyhow!("[remote] Cannot send new channel notification."); })
                        .unwrap();
                }),
            );
        }

        #[rstest]
        // #[case(random_str_rg(64_000..=65_536), 0..=0)]
        // #[case(random_str_rg(64_000..=65_536), 5..=25)]
        // #[case(random_str_rg(64_000..=65_536), 10..=50)]
        // #[case(random_str_rg(64_000..=65_536), 20..=100)]
        // #[case(random_str_rg(64_000..=65_536), 40..=200)]
        // #[case(random_str_rg(64_000..=65_536), 0..=0)]
        // #[case(random_str_rg(64_000..=65_536), 5..=25)]
        // #[case(random_str_rg(64_000..=65_536), 10..=75)]
        #[case(random_str_rg(64_000..=65_536), 5..=250)]
        // #[case(data_transfer_string(40_000), 5..=250)]
        #[tokio::test]
        async fn upgrades_to_a_new_channel_channels_latency(
            #[case] test_data: String,
            #[case] latency: RangeInclusive<u64>,
        ) {
            let options1 = ChannelMockOptions::random().throttle(latency.clone());
            let options2 = ChannelMockOptions::random().throttle(latency.clone());

            let (local_channel1, remote_channel1) = channel_mock_pair(options1.clone(), options1.clone());
            let (local_channel2, remote_channel2) = channel_mock_pair(options2.clone(), options2.clone());

            let (on_local_channel1, local_upgradable_channel1) = UpgradableChannel::new("local", local_channel1);
            let (on_remote_channel1, remote_upgradable_channel1) = UpgradableChannel::new("remote", remote_channel1);

            let _res = tokio::join!(
                Box::pin(async move {
                    test_async_stream(
                        local_upgradable_channel1,
                        remote_upgradable_channel1,
                        test_data,
                    ).await;
                }),
                Box::pin(async move {
                    wait_random(5..=25).await;

                    on_local_channel1.send(local_channel2)
                        .map_err(|_| { return anyhow!("[local] Cannot send new channel notification."); })
                        .unwrap();
                }),
                Box::pin(async move {
                    wait_random(5..=25).await;

                    on_remote_channel1.send(remote_channel2)
                        .map_err(|_| { return anyhow!("[remote] Cannot send new channel notification."); })
                        .unwrap();
                }),
            );
        }
    }
}
