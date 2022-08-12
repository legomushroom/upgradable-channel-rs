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
            // println!("[{}][writer][write]> getting channel2 lock", self.id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][write]> got channel2 lock", self.id);

            if let Some(writer) = lock.as_mut() {
                println!("[{}][writer][write]> writing to channel2", self.id);

                return writer.as_mut()
                    .poll_write(cx, buf);
            };
        }
        println!("[{}][writer][write]> writing to the main channel", self.id);

        return self.main_channel.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][writer][flush]> getting channel2 lock", self.id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][flush]> got channel2 lock", self.id);

            if let Some(writer) = lock.as_mut() {
                println!("[{}][writer][flush]> flushing channel2", self.id);

                return writer.as_mut()
                    .poll_flush(cx);
            };
        }

        // println!("[{}][flush]> writer unlock", self.id);

        println!("[{}][writer][flush]> flushing main channel", self.id);

        return self.main_channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][writer][shutdown]> getting channel2 lock", self.id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][shutdown]> got channel2 lock", self.id);

            if let Some(writer) = lock.as_mut() {
                println!("[{}][writer][shutdown]> shutdown channel2", self.id);

                return writer.as_mut()
                    .poll_shutdown(cx);
            };
        }

        // println!("[{}][shutdown]> writer unlock", self.id);

        return self.main_channel.as_mut()
            .poll_shutdown(cx);
    }
}
