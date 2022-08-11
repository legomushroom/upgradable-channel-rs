use std::{pin::Pin, task::{Context, Poll}, io};

use tokio::io::AsyncWrite;

use crate::channel::UpgradableChannel;

impl AsyncWrite for UpgradableChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        {
            // println!("[{}][write]> writer lock", self.id);

            let mut lock = self.channel2_writer.lock().unwrap();

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_write(cx, buf);
            };
        }
            // println!("[{}][write]> writer unlock", self.id);

        return self.main_channel.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][flush]> writer lock", self.id);
            let mut lock = self.channel2_writer.lock().unwrap();

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_flush(cx);
            };
        }

        // println!("[{}][flush]> writer unlock", self.id);

        return self.main_channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][shutdown]> writer lock", self.id);
            let mut lock = self.channel2_writer.lock().unwrap();

            if let Some(writer) = lock.as_mut() {
                return writer.as_mut()
                    .poll_shutdown(cx);
            };
        }

        // println!("[{}][shutdown]> writer unlock", self.id);

        return self.main_channel.as_mut()
            .poll_shutdown(cx);
    }
}
