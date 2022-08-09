use std::{pin::Pin, task::{Context, Poll}, io::{self, Error}};

use tokio::io::{AsyncRead, ReadBuf};

use crate::channel::UpgradableChannel;

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.is_upgraded_reads {
            let channel2 = match self.channel2.as_mut() {
                Some(channel) => channel,
                None => {
                    // TODO: get the real error
                    let error = Error::last_os_error();
                    return Poll::Ready(Err(error));
                },
            };

            // TODO: reply with the buffered data first
            
            return channel2.as_mut()
                .poll_read(cx, buf);
        }

        return self.channel1.as_mut()
            .poll_read(cx, buf);
    }
}
