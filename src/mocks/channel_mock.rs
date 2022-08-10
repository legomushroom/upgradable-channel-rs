use std::{pin::Pin, task::{Context, Poll}, io, time::Duration, thread};

use connection_utils::Channel;
use cs_utils::{random_number, random_str, random_bool};
use tokio::io::{duplex, AsyncRead, AsyncWrite, ReadBuf, DuplexStream};

// TODO: move to `cs-utils` crate
pub fn wait_sync(timeout_ms: u64) {
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(timeout_ms));
    });

    handle.join().unwrap();
}

// TODO: move to `connection-utils` crate

pub struct ChannelMock<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static = DuplexStream> {
    id: u16,
    label: String,
    channel: Pin<Box<TAsyncDuplex>>,
    options: ChannelMockOptions,
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
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMockOptions {
    id: u16,
    label: String,
    throttle_ms: u64,
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

    pub fn throttle_ms(
        self,
        throttle_ms: u64,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            throttle_ms,
            ..self
        };
    }
}

impl Default for ChannelMockOptions {
    fn default() -> ChannelMockOptions {
        return ChannelMockOptions {
            id: random_number(0..=u16::MAX),
            label: format!("channel-mock-{}", random_str(8)),
            throttle_ms: 1,
        };
    }
}

pub fn channel_mock_pair(
    options: ChannelMockOptions,
) -> (Box<dyn Channel>,  Box<dyn Channel>) {
    let (channel1, channel2) = duplex(1024);

    return (
        ChannelMock::new(Box::new(channel1), options.clone()),
        ChannelMock::new(Box::new(channel2), options.clone()),
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
        if self.options.throttle_ms > 0 && random_bool() {
            // let throttle_ms = self.options.throttle_ms;
            let waker = cx.waker().clone();

            thread::spawn(move || {
                wait_sync(random_number(1..=25));
                waker.wake();
            });

            return Poll::Pending;
        }

        return self.channel.as_mut()
            .poll_read(cx, buf);
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncWrite for ChannelMock<TAsyncDuplex> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.options.throttle_ms > 0 && random_bool() {
            // let throttle_ms = self.options.throttle_ms;
            let waker = cx.waker().clone();

            thread::spawn(move || {
                wait_sync(random_number(5..=15));
                waker.wake();
            });

            return Poll::Pending;
        }

        return self.channel.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.options.throttle_ms > 0 && random_bool() {
            // let throttle_ms = self.options.throttle_ms;
            let waker = cx.waker().clone();

            thread::spawn(move || {
                wait_sync(random_number(5..=15));
                waker.wake();
            });

            return Poll::Pending;
        }

        return self.channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.options.throttle_ms > 0 && random_bool() {
            // let throttle_ms = self.options.throttle_ms;
            let waker = cx.waker().clone();

            thread::spawn(move || {
                wait_sync(random_number(5..=15));
                waker.wake();
            });

            return Poll::Pending;
        }

        return self.channel.as_mut()
            .poll_flush(cx);
    }
}
