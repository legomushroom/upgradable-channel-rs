use cs_utils::{futures::wait_random, random_number};
use upgradable_channel::{UpgradableChannel, mocks::{channel_mock_pair, ChannelMockOptions}, TUpgradableChannel};

#[tokio::main]
async fn main() {
    let options1 = ChannelMockOptions::default()
        .throttle_ms(random_number(100..=200));
    let options2 = ChannelMockOptions::default()
        .throttle_ms(random_number(100..=200));

    let (local_channel1, remote_channel1) = channel_mock_pair(options1);
    let (local_channel2, remote_channel2) = channel_mock_pair(options2);

    tokio::try_join!(
        tokio::spawn(async move {
            wait_random(50..=200).await;

            let mut local_channel1 = UpgradableChannel::new(local_channel1);

            println!("> upgrading local channel");

            local_channel1.upgrade(local_channel2)
                .expect("Cannot upgrade local channel.");

            // assert!(local_channel1.is_upgrdaded());

            println!("> local channel upgraded");
        }),
        tokio::spawn(async move {
            wait_random(1..=25).await;

            let mut remote_channel1 = UpgradableChannel::new(remote_channel1);

            println!("> upgrading remote channel");

            remote_channel1.upgrade(remote_channel2)
                .expect("Cannot upgrade remote channel.");

            // assert!(remote_channel1.is_upgrdaded());

            println!("> remote channel upgraded");
        }),
    ).unwrap();

    println!("> all channels upgraded");
}
