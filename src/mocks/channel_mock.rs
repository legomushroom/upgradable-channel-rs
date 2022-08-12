use std::{pin::Pin, task::{Context, Poll}, io, ops::RangeInclusive};

use futures::{Future, ready};
use connection_utils::Channel;
use tokio::io::{duplex, AsyncRead, AsyncWrite, ReadBuf, DuplexStream};
use cs_utils::{random_number, random_str, random_bool, futures::wait_random, traits::Random};

// TODO: move to `cs-utils` crate
// pub fn wait_sync(timeout_ms: u64) {
//     let handle = thread::spawn(move || {
//         thread::sleep(Duration::from_millis(timeout_ms));
//     });

//     handle.join().unwrap();
// }

pub struct ChannelMock<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static = DuplexStream> {
    id: u16,
    label: String,
    channel: Pin<Box<TAsyncDuplex>>,
    options: ChannelMockOptions,
    read_delay_future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    write_delay_future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> ChannelMock<TAsyncDuplex> {
    pub fn new(
        channel: Box<TAsyncDuplex>,
        options: ChannelMockOptions,
    ) -> Box<dyn Channel> {
        return Box::new(
            ChannelMock {
                id: options.id,
                label: options.label.clone(),
                channel: Pin::new(channel),
                options,
                read_delay_future: None,
                write_delay_future: None,
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMockOptions {
    id: u16,
    label: String,
    throttle_range: RangeInclusive<u64>,
}

impl ChannelMockOptions {
    pub fn id(
        self,
        id: u16,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            id,
            ..self
        };
    }

    pub fn label(
        self,
        label: impl AsRef<str> + ToString,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            label: label.to_string(),
            ..self
        };
    }

    pub fn throttle(
        self,
        throttle_range: RangeInclusive<u64>,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            throttle_range,
            ..self
        };
    }
}

impl Random for ChannelMockOptions {
    fn random() -> Self {
        let min = random_number(0..5);
        let max = random_number(5..=50);

        return ChannelMockOptions::default()
            .throttle(min..=max);
    }
}

impl Default for ChannelMockOptions {
    fn default() -> ChannelMockOptions {
        return ChannelMockOptions {
            id: random_number(0..=u16::MAX),
            label: format!("channel-mock-{}", random_str(8)),
            throttle_range: (0..=0),
        };
    }
}

pub fn channel_mock_pair(
    options1: ChannelMockOptions,
    options2: ChannelMockOptions,
) -> (Box<dyn Channel>, Box<dyn Channel>) {
    let (channel1, channel2) = duplex(1024);

    return (
        ChannelMock::new(Box::new(channel1), options1),
        ChannelMock::new(Box::new(channel2), options2),
    );
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> Channel for ChannelMock<TAsyncDuplex> {
    fn id(&self) -> u16 {
        return self.id;
    }

    fn label(&self) ->  &String {
        return &self.label;
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncRead for ChannelMock<TAsyncDuplex> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // if delay future present, wait until it completes
        if let Some(read_delay_future) = self.read_delay_future.as_mut() {
            ready!(read_delay_future.as_mut().poll(cx));

            self.read_delay_future.take();
        }

        // otherwise run the read future to completion
        let result = ready!(self.channel.as_mut().poll_read(cx, buf));

        // println!("[{}]> read some data: {:?}", self.id, result);

        // optionally create a throttle delay future
        if random_bool() {
            // println!("[{}]> create new timeout", self.id);
        
            self.read_delay_future = Some(Box::pin(wait_random(self.options.throttle_range.clone())));
        }

        return Poll::Ready(result);
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncWrite for ChannelMock<TAsyncDuplex> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // if delay future present, wait until it completes
        if let Some(write_delay_future) = self.write_delay_future.as_mut() {
            ready!(write_delay_future.as_mut().poll(cx));

            self.write_delay_future.take();
        }

        let result = ready!(self.channel.as_mut().poll_write(cx, buf));

        // optionally create a throttle delay future
        if random_bool() {
            // println!("[{}]> create new timeout", self.id);
        
            self.write_delay_future = Some(Box::pin(wait_random(self.options.throttle_range.clone())));
        }

        return Poll::Ready(result);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        return self.channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        return self.channel.as_mut()
            .poll_flush(cx);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use connection_utils::test::test_async_stream;
    use cs_utils::{random_str, traits::Random, random_number};

    use crate::utils::{create_framed_stream, test_framed_stream, TestOptions, StreamTestMessage};

    use super::channel_mock_pair;

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
        let (channel1, channel2) = channel_mock_pair(Random::random(), Random::random());

        test_async_stream(
            channel1,
            channel2,
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
        let (channel1, channel2) = channel_mock_pair(Random::random(), Random::random());

        let channel1 = create_framed_stream::<StreamTestMessage>(channel1);
        let channel2 = create_framed_stream::<StreamTestMessage>(channel2);

        test_framed_stream(
            channel1,
            channel2,
            TestOptions::random().items_count(items_count),
        ).await;
    }
}
