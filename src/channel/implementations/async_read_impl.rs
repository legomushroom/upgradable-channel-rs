use std::{pin::Pin, task::{Context, Poll}, io, cmp};

use tokio::io::{AsyncRead, ReadBuf};

use crate::channel::UpgradableChannel;

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.is_upgraded_reads {
            let channel2_buffer_len = self.channel2_buffer.len();

            let channel2 = match self.channel2.as_mut() {
                Some(channel) => channel,
                None => {
                    #[cfg(debug_assertions)]
                    panic!("[poll_read] Channel2 not found, but `is_upgraded_reads` is set.");

                    #[cfg(not(debug_assertions))]
                    return self.channel1.as_mut()
                        .poll_read(cx, buf);
                },
            };

            // if there are some data in the channel2 buffer and the read buffer has
            // some space left, copy data from the channel2 buffer to the read buffer
            if channel2_buffer_len > 0 && buf.remaining() > 0 {
                let buf_remaining = cmp::min(buf.remaining(), channel2_buffer_len);

                let buffered_data = self.channel2_buffer
                    .drain(..buf_remaining)
                    .collect::<Vec<_>>();

                buf.put_slice(&buffered_data[..]);

                // TODO: read from the channel2 and buffer read data to
                // make sure data flows while we empty the buffer?

                return Poll::Ready(Ok(()));
            }

            return channel2.as_mut()
                .poll_read(cx, buf);
        }

        return self.channel1.as_mut()
            .poll_read(cx, buf);
    }
}
