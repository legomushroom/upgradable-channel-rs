use std::{pin::Pin, task::{Context, Poll}, io};

use tokio::io::{AsyncRead, ReadBuf};

use crate::channel::UpgradableChannel;

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut lock = self.channel2_reader.lock().unwrap();

            if let Some(reader) = lock.as_mut() {
                println!("[{}][read]> reading from channel2", self.id);
                return reader.as_mut()
                    .poll_read(cx, buf);
            };
        }

        println!("[{}][read]> reading from the main channel", self.id);

        return self.main_channel.as_mut()
            .poll_read(cx, buf);
    }
}
