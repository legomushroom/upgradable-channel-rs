use std::{pin::Pin, task::{Context, Poll}, io};

use futures::{ready, FutureExt};
use tokio::io::AsyncWrite;

use crate::{channel::UpgradableChannel, buf_to_str};

impl AsyncWrite for UpgradableChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        {
            // println!("[{}][writer][write]> getting channel2 lock", self.test_id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][write]> got channel2 lock", self.test_id);

            if let Some(writer) = lock.as_mut() {
                // println!("[{}][writer][write]> writing to channel2:\n{:?}", self.test_id, buf_to_str(buf));

                let result = ready!(writer.as_mut().poll_write(cx, buf));

                if let Ok(bytes_written) = &result {
                    let data_str = buf_to_str(&buf[..*bytes_written]);
        
                    println!("[{}][writer][write]> wrote to channel2:\n{:?}", self.test_id,data_str);
                }
        
                return Poll::Ready(result);
            };
        }

        let mut lock = ready!(Box::pin(self.main_channel_writer.lock()).poll_unpin(cx));

        let result = ready!(lock.as_mut().poll_write(cx, buf));

        if let Ok(bytes_written) = &result {
            let data_str = buf_to_str(&buf[..*bytes_written]);

            println!("[{}][writer][write]> wrote to the main channel:\n{:?}", self.test_id,data_str);
        }

        return Poll::Ready(result);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][writer][flush]> getting channel2 lock", self.test_id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][flush]> got channel2 lock", self.test_id);

            if let Some(writer) = lock.as_mut() {
                println!("[{}][writer][flush]> flushing channel2", self.test_id);

                return writer.as_mut()
                    .poll_flush(cx);
            };
        }

        // println!("[{}][flush]> writer unlock", self.test_id);

        println!("[{}][writer][flush]> flushing main channel", self.test_id);

        let mut lock = ready!(Box::pin(self.main_channel_writer.lock()).poll_unpin(cx));

        return lock.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        {
            // println!("[{}][writer][shutdown]> getting channel2 lock", self.test_id);

            let mut lock = self.channel2_writer.lock().unwrap();

            // println!("[{}][writer][shutdown]> got channel2 lock", self.test_id);

            if let Some(writer) = lock.as_mut() {
                println!("[{}][writer][shutdown]> shutdown channel2", self.test_id);

                return writer.as_mut()
                    .poll_shutdown(cx);
            };
        }

        println!("[{}][shutdown]> writer unlock", self.test_id);

        let mut lock = ready!(Box::pin(self.main_channel_writer.lock()).poll_unpin(cx));

        return lock.as_mut()
            .poll_shutdown(cx);
    }
}
