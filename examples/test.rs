use cs_utils::futures::wait;
use upgradable_channel::{UpgradableChannel, mocks::channel_mock_pair, TUpgradableChannel};

#[tokio::main]
async fn main() {
    let (local_channel1, remote_channel1) = channel_mock_pair(1, "channel1".to_string());
    let (local_channel1_msg, remote_channel1_msg) = channel_mock_pair(2, "channel2".to_string());
    let (local_channel2, remote_channel2) = channel_mock_pair(3, "channel3".to_string());

    tokio::try_join!(
        tokio::spawn(async move {
            let mut local_channel1 = UpgradableChannel::new(
                local_channel1,
                local_channel1_msg,
            );

            println!("> upgrading local channel");

            local_channel1.upgrade(local_channel2).await
                .expect("Cannot upgrade local channel.");

            println!("> local channel upgraded");

            wait(50).await;
        }),
        tokio::spawn(async move {
            let mut remote_channel1 = UpgradableChannel::new(
                remote_channel1,
                remote_channel1_msg,
            );

            println!("> upgrading remote channel");

            remote_channel1.upgrade(remote_channel2).await
                .expect("Cannot upgrade remote channel.");

            println!("> remote channel upgraded");

            wait(50).await;
        }),
    ).unwrap();

    println!("> all channels upgraded");
}
