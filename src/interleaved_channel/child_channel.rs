use std::{pin::Pin, task::{Context, Poll}, io};

use connection_utils::Channel;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

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
