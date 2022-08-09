use std::{pin::Pin, task::{Context, Poll}, io};

use connection_utils::Channel;
use tokio::io::{duplex, AsyncRead, AsyncWrite, ReadBuf, DuplexStream};

// TODO: move to `connection-utils` crate

pub struct ChannelMock<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static = DuplexStream> {
    id: u16,
    label: String,
    channel: Pin<Box<TAsyncDuplex>>,
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> ChannelMock<TAsyncDuplex> {
    pub fn new(
        id: u16,
        label: String,
        channel: Box<TAsyncDuplex>,
    ) -> Box<dyn Channel> {
        return Box::new(
            ChannelMock {
                id,
                label,
                channel: Pin::new(channel),
            },
        );
    }
}

pub fn channel_mock_pair(
    id: u16,
    label: String,
) -> (Box<dyn Channel>,  Box<dyn Channel>) {
    let (channel1, channel2) = duplex(1024);

    return (
        ChannelMock::new(id, label.clone(), Box::new(channel1)),
        ChannelMock::new(id, label.clone(), Box::new(channel2)),
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
        return self.channel.as_mut()
            .poll_write(cx, buf);
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
