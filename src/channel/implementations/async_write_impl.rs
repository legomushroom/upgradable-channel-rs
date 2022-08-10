use std::{pin::Pin, task::{Context, Poll}, io};

use futures::{ready, FutureExt};
use tokio::io::AsyncWrite;

use crate::channel::UpgradableChannel;

impl AsyncWrite for UpgradableChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        {
            let mut lock = ready!(Box::pin(self.channel2_writer.lock()).poll_unpin(cx));

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_write(cx, buf);
            };
        }

        return self.channel1.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut lock = ready!(Box::pin(self.channel2_writer.lock()).poll_unpin(cx));

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_flush(cx);
            };
        }

        return self.channel1.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut lock = ready!(Box::pin(self.channel2_writer.lock()).poll_unpin(cx));

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_shutdown(cx);
            };
        }

        return self.channel1.as_mut()
            .poll_shutdown(cx);
    }
}
