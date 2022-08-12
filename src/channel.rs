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
    use cs_utils::{traits::Random, random_str_rg, futures::{wait_random, with_thread}};
    use rstest::rstest;
    use tokio::io::{AsyncWriteExt, AsyncReadExt};

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

        let (on_local_channel1, mut local_upgradable_channel1) = UpgradableChannel::new("local", local_channel1);
        let (on_remote_channel1, mut remote_upgradable_channel1) = UpgradableChannel::new("remote", remote_channel1);

        let test_data1 = test_data.clone();
        let test_data2 = test_data.clone();

        let _res = tokio::join!(
            Box::pin((async move {
                println!("> starting data transfer 1");

                let test_data1 = test_data1.as_bytes();

                let mut written = 0;
                while written < test_data1.len() {
                    written += local_upgradable_channel1.write(&test_data1[written..]).await.unwrap();
                }

                println!("> data transfer complete 1");
            })),
            Box::pin((async move {
                println!("> starting data transfer 2");

                let mut received_data = "".to_string();

                let mut buf = [0; 256];
                loop {
                    let bytes_read = remote_upgradable_channel1.read(&mut buf).await.unwrap();

                    let data = &buf[..bytes_read];
                    let received_str = String::from_utf8(data.to_vec()).unwrap();

                    received_data = format!("{received_data}{received_str}");

                    assert!(
                        test_data2.starts_with(&received_data),
                        "Data corruption, sent and received data do not match.",
                    );

                    println!(">> all good so far");

                    if received_data.len() == test_data2.len() {
                        break;
                    }
                }

                println!("> data transfer complete 2");
            })),
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