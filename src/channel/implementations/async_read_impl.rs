use std::{pin::Pin, task::{Context, Poll}, io, cmp};

use futures::{ready, FutureExt};
use tokio::io::{AsyncRead, ReadBuf};

use crate::{channel::UpgradableChannel, buf_to_str};

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let filled_before = buf.filled().len();
        // let is_waker = self.last_read_waker.lock().unwrap().is_some();
        // let is_channel2 = self.channel2_reader.lock().unwrap().is_some();

        {
            // println!("[{}][reader][read]> getting channel2 lock", self.test_id);

            let mut lock = self.channel2_reader.lock().unwrap();

            // println!("[{}][reader]> got channel2 lock", self.test_id);

            if let Some((reader, channel2_buffer)) = lock.as_mut() {
                // println!("[{}][reader][read]> reading from channel2 buffer", self.test_id);

                if channel2_buffer.len() > 0 {
                    let bytes_to_read = cmp::min(channel2_buffer.len(), buf.remaining());

                    let bytes = channel2_buffer.drain(0..bytes_to_read).collect::<Vec<_>>();

                    println!("[{}][reader][read]> read from channel2 buffer:\n{:?}", self.test_id, buf_to_str(&bytes));

                    buf.put_slice(&bytes[..]);

                    return Poll::Ready(Ok(()));
                }

                // println!("[{}][reader][read]> reading from channel2", self.test_id);

                let result = ready!(reader.as_mut().poll_read(cx, buf));

                if let Ok(_) = &result {
                    let filled = buf.filled();
                    let bytes = &filled[filled_before..];

                    println!("[{}][reader][read]> read from channel2:\n{:?}", self.test_id, buf_to_str(&bytes));
                }

                return Poll::Ready(result);
            };
        }

        let mut lock = ready!(Box::pin(self.main_channel_reader.lock()).poll_unpin(cx));

        // println!("[{}][reader][read]> reading from the main channel", self.test_id);

        let result = lock.as_mut()
            .poll_read(cx, buf);

        // println!("[{}][reader][read]> read result: {:?}", self.test_id, result);

        // if the main channel returns `Poll::Pending`, we need to save the `Waker` and
        // awake it when the channel is upgraded again. Otherwise the `poll_read` function
        // might never be called again.
        match &result {
            Poll::Pending => {
                self.last_read_waker.lock().unwrap().replace(cx.waker().clone());

                println!("[{}][reader][read]> the main channel pending", self.test_id);
            },
            Poll::Ready(result) => {
                self.last_read_waker.lock().unwrap().take();

                if let Ok(_) = result {
                    let filled = buf.filled();
                    let bytes = &filled[filled_before..];

                    if filled.len() != filled_before {
                        println!("[{}][reader][read]> read from the main channel:\n{:?}", self.test_id, buf_to_str(&bytes));
                    }
                }
            }
        };

        //  println!("[{}][reader][read]> main channel result: {:?}", self.test_id, result);

        return result;
    }
}
