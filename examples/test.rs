use anyhow::anyhow;
use connection_utils::test::test_async_stream;
use cs_utils::{futures::wait_random, random_number, random_str};
use upgradable_channel::{UpgradableChannel, mocks::{channel_mock_pair, ChannelMockOptions}};

#[tokio::main]
async fn main() {
    let options1 = ChannelMockOptions::default()
        .throttle_ms(random_number(1..=3));
    let options2 = ChannelMockOptions::default()
        .throttle_ms(random_number(1..=3));

    let (local_channel1, remote_channel1) = channel_mock_pair(options1.clone(), options1.clone());
    let (local_channel2, remote_channel2) = channel_mock_pair(options2.clone(), options2.clone());

    let (on_local_channel1, local_upgradable_channel1) = UpgradableChannel::new("local", local_channel1);
    let (on_remote_channel1, remote_upgradable_channel1) = UpgradableChannel::new("remote", remote_channel1);

    tokio::try_join!(
        tokio::spawn(async move {
            println!("> starting data transfer");

            test_async_stream(local_upgradable_channel1, remote_upgradable_channel1, random_str(5 * 4096)).await;

            println!("> data transfer complete");
        }),
        tokio::spawn(async move {
            wait_random(1..=50).await;

            on_local_channel1.send(local_channel2)
                .map_err(|_| { return anyhow!("[local] Cannot send new channel notification."); })
                .unwrap();

            println!("> local upgrade sent");
        }),
        tokio::spawn(async move {
            wait_random(1..=50).await;

            on_remote_channel1.send(remote_channel2)
                .map_err(|_| { return anyhow!("[remote] Cannot send new channel notification."); })
                .unwrap();

            println!("> remote upgrade sent");
        }),
    ).unwrap();

    println!("> done");
}
