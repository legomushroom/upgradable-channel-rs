use std::{pin::Pin, task::{Context, Poll}, io};

use connection_utils::Channel;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[cfg(test)]
use cs_utils::{random_number, random_str};

pub struct ChildChannel<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> {
    id: u16,
    label: String,
    channel: Pin<Box<TAsyncDuplex>>,
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> ChildChannel<TAsyncDuplex> {
    pub fn new(
        id: u16,
        label: impl AsRef<str> + ToString,
        channel: Box<TAsyncDuplex>,
    ) -> Box<dyn Channel> {
        return Box::new(
            ChildChannel {
                id,
                label: label.to_string(),
                channel: Pin::new(channel),
            },
        );
    }

    #[cfg(test)]
    pub fn new_random(channel: Box<TAsyncDuplex>) -> Box<dyn Channel> {
        return Box::new(
            ChildChannel {
                id: random_number(0..=u16::MAX),
                label: format!("child-channel-{}", random_str(8)),
                channel: Pin::new(channel),
            },
        );
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> Channel for ChildChannel<TAsyncDuplex> {
    fn id(&self) -> u16 {
        return self.id;
    }
    
    fn label(&self) ->  &String {
        return &self.label;
    }
} 

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncRead for ChildChannel<TAsyncDuplex> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        return self.channel.as_mut()
            .poll_read(cx, buf);
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncWrite for ChildChannel<TAsyncDuplex> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        return self.channel.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        return self.channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        return self.channel.as_mut()
            .poll_shutdown(cx);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use connection_utils::test::test_async_stream;
    use cs_utils::{random_str, traits::Random, random_number};

    use super::ChildChannel;
    use crate::{utils::{create_framed_stream, test_framed_stream, TestOptions, StreamTestMessage}, mocks::{channel_mock_pair, ChannelMockOptions}};

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
    async fn transfers_binary_data(
        #[case] test_data_size: usize,
    ) {
        let (channel1, channel2) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        test_async_stream(
            ChildChannel::new_random(Box::new(channel1)),
            ChildChannel::new_random(Box::new(channel2)),
            random_str(test_data_size),
        ).await;
    }

    #[rstest]
    #[case(random_number(6..=8))]
    #[case(random_number(12..=16))]
    #[case(random_number(25..=32))]
    #[case(random_number(53..=64))]
    #[case(random_number(100..=128))]
    #[case(random_number(200..=256))]
    #[tokio::test]
    async fn transfers_stream_data(
        #[case] items_count: u32,
    ) {
        let (channel1, channel2) = channel_mock_pair(ChannelMockOptions::random(), ChannelMockOptions::random());

        let channel1 = ChildChannel::new_random(Box::new(channel1));
        let channel2 = ChildChannel::new_random(Box::new(channel2));

        let channel1 = create_framed_stream::<StreamTestMessage>(channel1);
        let channel2 = create_framed_stream::<StreamTestMessage>(channel2);

        test_framed_stream(
            channel1,
            channel2,
            TestOptions::random().items_count(items_count),
        ).await;
    }
}
