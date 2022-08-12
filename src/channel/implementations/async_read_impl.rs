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
            // println!("[{}][reader][read]> getting channel2 lock", self.id);

            let mut lock = self.channel2_reader.lock().unwrap();

            // println!("[{}][reader]> got channel2 lock", self.id);

            if let Some(reader) = lock.as_mut() {
                println!("[{}][reader][read]> reading from channel2", self.id);
                return reader.as_mut()
                    .poll_read(cx, buf);
            };
        }

        println!("[{}][reader][read]> reading from the main channel", self.id);

        let result =  self.main_channel.as_mut()
            .poll_read(cx, buf);

        // if the main channel returns `Poll::Pending`, we need to save the `Waker` and
        // awake it when the channel is upgraded again. Otherwise the `poll_read` function
        // might never be called again.
        if let Poll::Pending = result {
            self.last_read_waker.lock().unwrap().replace(cx.waker().clone());
        }

        //  println!("[{}][reader][read]> main channel result: {:?}", self.id, result);

        return result;
    }
}
