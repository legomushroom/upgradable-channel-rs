use connection_utils::test::test_async_stream;
use cs_utils::{random_str, traits::Random};
use upgradable_channel::mocks::{ChannelMockOptions, channel_mock_pair};

#[tokio::main]
async fn main() {
    let (channel1, channel2) = channel_mock_pair(
        ChannelMockOptions::random(),
        ChannelMockOptions::random(),
    );

    let data = random_str(5 * 4096);

    println!("> starting data transfer");
    
    test_async_stream(channel1, channel2, data).await;

    println!("> data transfer complete");

}
