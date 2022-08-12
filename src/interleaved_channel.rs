use std::pin::Pin;

use anyhow::{bail, Result};
use connection_utils::Channel;
use cs_utils::futures::GenericCodec;
use serde::{Serialize, Deserialize};
use tokio::io::{duplex, DuplexStream, split, WriteHalf, ReadHalf, AsyncReadExt, AsyncWriteExt};
use futures::{StreamExt, stream::{SplitStream, SplitSink}, future::select_all, Future, select, FutureExt, SinkExt};

mod child_channel;
use child_channel::ChildChannel;
use tokio_util::codec::Framed;

#[derive(Serialize, Deserialize, Debug)]
pub enum LayerMessage {
    Channel1(Vec<u8>),
    Channel2(Vec<u8>),
}

async fn forward_reads(
    mut channel: SplitStream<Framed<Box<dyn Channel>, GenericCodec<LayerMessage>>>,
    mut child1: WriteHalf<DuplexStream>,
    mut child2: WriteHalf<DuplexStream>,
)-> Result<()> {
    let mut child1 = Pin::new(&mut child1);
    let mut child2 = Pin::new(&mut child2);

    loop {
        let item = match channel.next().await {
            Some(item) => item?,
            None => bail!("Stream closed."),
        };

        // TODO: channels should not block each other, use `write()` instead.
        //  Or maybe run the channels under a separate thread.
        //  Or maybe use buffers.
        match item {
            LayerMessage::Channel1(data) => {
                child1.write_all(&data[..]).await?;
            },
            LayerMessage::Channel2(data) => {
                child2.write_all(&data[..]).await?;
            },
        };
    }
}

async fn forward_writes(
    mut channel: SplitSink<Framed<Box<dyn Channel>, GenericCodec<LayerMessage>>, LayerMessage>,
    mut child1: ReadHalf<DuplexStream>,
    mut child2: ReadHalf<DuplexStream>,
) -> Result<()> {
    let mut child1 = Pin::new(&mut child1);
    let mut child2 = Pin::new(&mut child2);

    let mut buf1 = [0; 1024];
    let mut buf2 = [0; 1024];

    loop {
        select! {
            maybe_bytes_read = child1.read(&mut buf1).fuse() => {
                let bytes_read = maybe_bytes_read?;

                channel.send(LayerMessage::Channel1(buf1[..bytes_read].to_vec())).await?;
            },
            maybe_bytes_read = child2.read(&mut buf2).fuse() => {
                let bytes_read = maybe_bytes_read?;

                channel.send(LayerMessage::Channel2(buf2[..bytes_read].to_vec())).await?;
            },
        }
    }
}

fn forward(
    channel: Box<dyn Channel>,
    child1: DuplexStream,
    child2: DuplexStream,
) {
    let (sink, source) = Framed::new(
        channel,
        GenericCodec::<LayerMessage>::new(),
    ).split();

    let (child1_read, child1_write) = split(child1);
    let (child2_read, child2_write) = split(child2);

    let futures: Vec<Pin<Box<dyn Future<Output = _> + Send + 'static>>> = vec![
        Box::pin(forward_reads(source, child1_write, child2_write)),
        Box::pin(forward_writes(sink, child1_read, child2_read)),
    ];

    let _res = tokio::spawn(select_all(futures));
}

pub fn divide_channel(
    channel: Box<dyn Channel>,
) -> (Box<dyn Channel>, Box<dyn Channel>) {
    // TODO: add `buffer_size` attribute to the Channel trait
    // let buffer_size = channel.buffer_size();
    
    let (child1_sink, child1_source) = duplex(1024);
    let (child2_sink, child2_source) = duplex(1024);

    let child_channel1 = ChildChannel::new(
        channel.id(),
        channel.label(),
        Box::new(child1_sink),
    );

    let child_channel2 = ChildChannel::new(
        channel.id(),
        channel.label(),
        Box::new(child2_sink),
    );

    forward(
        channel,
        child1_source,
        child2_source,
    );

    return (child_channel1, child_channel2);
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use connection_utils::test::test_async_stream;
    use cs_utils::{random_number, random_str, futures::wait_random, traits::Random};
    
    use crate::utils::{test_framed_stream, TestOptions, StreamTestMessage};
    use crate::utils::create_framed_stream;
    use crate::mocks::{channel_mock_pair, ChannelMockOptions};

    use super::divide_channel;

    #[rstest]
    #[case(128)]
    #[case(256)]
    #[case(512)]
    #[case(1_024)]
    #[case(2_048)]
    #[case(4_096)]
    #[case(8_192)]
    #[case(16_384)]
    #[case(32_768)]
    #[case(65_536)]
    #[tokio::test]
    async fn divides_channel(
        #[case] test_data_size: usize,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        tokio::try_join!(
            tokio::spawn(async move {
                test_async_stream(
                    local_channel1,
                    remote_channel1,
                    random_str(test_data_size),
                ).await;
            }),
            tokio::spawn(async move {
                test_async_stream(
                    local_channel2,
                    remote_channel2,
                    random_str(test_data_size),
                ).await;
            }),
        ).unwrap();
    }

    #[rstest]
    #[case(128)]
    #[case(256)]
    #[case(512)]
    #[case(1_024)]
    #[case(2_048)]
    #[case(4_096)]
    #[case(8_192)]
    #[case(16_384)]
    #[case(32_768)]
    #[case(65_536)]
    #[tokio::test]
    async fn works_if_second_channel_is_not_used(
        #[case] test_data_size: usize,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, _local_channel2) = divide_channel(local_channel);
        let (remote_channel1, _remote_channel2) = divide_channel(remote_channel);

        test_async_stream(
            local_channel1,
            remote_channel1,
            random_str(test_data_size),
        ).await;
    }

    #[rstest]
    #[case(128)]
    #[case(256)]
    #[case(512)]
    #[case(1_024)]
    #[case(2_048)]
    #[case(4_096)]
    #[case(8_192)]
    #[case(16_384)]
    #[case(32_768)]
    #[case(65_536)]
    #[tokio::test]
    async fn works_if_first_channel_is_not_used(
        #[case] test_data_size: usize,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (_local_channel1, local_channel2) = divide_channel(local_channel);
        let (_remote_channel1, remote_channel2) = divide_channel(remote_channel);

        test_async_stream(
            local_channel2,
            remote_channel2,
            random_str(test_data_size),
        ).await;
    }

    #[rstest]
    #[case(128)]
    #[case(256)]
    #[case(512)]
    #[case(1_024)]
    #[case(2_048)]
    #[case(4_096)]
    #[case(8_192)]
    #[case(16_384)]
    #[case(32_768)]
    #[case(65_536)]
    #[tokio::test]
    async fn works_if_second_channel_dropped(
        #[case] test_data_size: usize,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        drop(local_channel2);
        drop(remote_channel2);

        test_async_stream(
            local_channel1,
            remote_channel1,
            random_str(test_data_size),
        ).await;
    }

    #[rstest]
    #[case(128)]
    #[case(256)]
    #[case(512)]
    #[case(1_024)]
    #[case(2_048)]
    #[case(4_096)]
    #[case(8_192)]
    #[case(16_384)]
    #[case(32_768)]
    #[case(65_536)]
    #[tokio::test]
    async fn works_if_first_channel_dropped(
        #[case] test_data_size: usize,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        drop(local_channel1);
        drop(remote_channel1);

        test_async_stream(
            local_channel2,
            remote_channel2,
            random_str(test_data_size),
        ).await;
    }

    #[rstest]
    #[case(128, 8)]
    #[case(256, 16)]
    #[case(512, 32)]
    #[case(1_024, 64)]
    #[case(2_048, 128)]
    #[case(4_096, 256)]
    #[tokio::test]
    async fn works_if_second_channel_is_a_stream(
        #[case] test_data_size: usize,
        #[case] items_count: u32,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        let local_channel2 = create_framed_stream::<StreamTestMessage>(local_channel2);
        let remote_channel2 = create_framed_stream::<StreamTestMessage>(remote_channel2);

        tokio::try_join!(
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_async_stream(
                    local_channel1,
                    remote_channel1,
                    random_str(test_data_size),
                ).await;
            }),
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_framed_stream(
                    local_channel2,
                    remote_channel2,
                    TestOptions::random().items_count(items_count),
                ).await;
            }),
        ).unwrap();
    }

    #[rstest]
    #[case(128, 8)]
    #[case(256, 16)]
    #[case(512, 32)]
    #[case(1_024, 64)]
    #[case(2_048, 128)]
    #[case(4_096, 256)]
    #[tokio::test]
    async fn works_if_first_channel_is_a_stream(
        #[case] test_data_size: usize,
        #[case] items_count: u32,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        let local_channel1 = create_framed_stream::<StreamTestMessage>(local_channel1);
        let remote_channel1 = create_framed_stream::<StreamTestMessage>(remote_channel1);

        tokio::try_join!(
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_framed_stream(
                    local_channel1,
                    remote_channel1,
                    TestOptions::random().items_count(items_count),
                ).await;
            }),
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_async_stream(
                    local_channel2,
                    remote_channel2,
                    random_str(test_data_size),
                ).await;
            }),
        ).unwrap();
    }

    #[rstest]
    #[case(random_number(6..=8), random_number(6..=8))]
    #[case(random_number(12..=16), random_number(12..=16))]
    #[case(random_number(25..=32), random_number(25..=32))]
    #[case(random_number(53..=64), random_number(53..=64))]
    #[case(random_number(100..=128), random_number(100..=128))]
    #[case(random_number(200..=256), random_number(200..=256))]
    #[tokio::test]
    async fn works_if_all_channels_are_streams(
        #[case] items_count1: u32,
        #[case] items_count2: u32,
    ) {
        let (local_channel, remote_channel) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let (local_channel1, local_channel2) = divide_channel(local_channel);
        let (remote_channel1, remote_channel2) = divide_channel(remote_channel);

        let local_channel1 = create_framed_stream::<StreamTestMessage>(local_channel1);
        let remote_channel1 = create_framed_stream::<StreamTestMessage>(remote_channel1);

        let local_channel2 = create_framed_stream::<StreamTestMessage>(local_channel2);
        let remote_channel2 = create_framed_stream::<StreamTestMessage>(remote_channel2);

        tokio::try_join!(
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_framed_stream(
                    local_channel1,
                    remote_channel1,
                    TestOptions::random().items_count(items_count1),
                ).await;
            }),
            tokio::spawn(async move {
                wait_random(1..=25).await;

                test_framed_stream(
                    local_channel2,
                    remote_channel2,
                    TestOptions::random().items_count(items_count2),
                ).await;
            }),
        ).unwrap();
    }
}
