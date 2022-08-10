use std::{pin::Pin, task::{Context, Poll}, io};

use futures::{ready, FutureExt};
use tokio::io::{AsyncRead, ReadBuf};

use crate::channel::UpgradableChannel;

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut lock = ready!(Box::pin(self.channel2_reader.lock()).poll_unpin(cx));

            if let Some(reader) = lock.as_mut() {
                return reader.as_mut()
                    .poll_read(cx, buf);
            };
        }

        return self.channel1.as_mut()
            .poll_read(cx, buf);
    }
}
