use std::{pin::Pin, task::{Context, Poll}, io};

use tokio::io::AsyncWrite;

use crate::channel::UpgradableChannel;

impl AsyncWrite for UpgradableChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.is_upgraded_writes {
            let channel2 = match self.channel2.as_mut() {
                Some(channel) => channel,
                None => return Poll::Ready(todo!()),
            };

            return channel2.as_mut()
                .poll_write(cx, buf);
        }

        return self.channel1.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.is_upgraded_writes {
            let channel2 = match self.channel2.as_mut() {
                Some(channel) => channel,
                None => return Poll::Ready(todo!()),
            };

            return channel2.as_mut()
                .poll_flush(cx);
        }

        return self.channel1.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.is_upgraded_writes {
            let channel2 = match self.channel2.as_mut() {
                Some(channel) => channel,
                None => return Poll::Ready(todo!()),
            };

            return channel2.as_mut()
                .poll_shutdown(cx);
        }

        return self.channel1.as_mut()
            .poll_shutdown(cx);
    }
}
