use std::{pin::Pin, task::{Context, Poll}, io};

use connection_utils::Channel;
use cs_utils::{random_number, random_str, random_bool, futures::wait, traits::Random};
use futures::{Future, ready};
use tokio::io::{duplex, AsyncRead, AsyncWrite, ReadBuf, DuplexStream};

// TODO: move to `cs-utils` crate
// pub fn wait_sync(timeout_ms: u64) {
//     let handle = thread::spawn(move || {
//         thread::sleep(Duration::from_millis(timeout_ms));
//     });

//     handle.join().unwrap();
// }

// TODO: move to `connection-utils` crate

// fn throttle(
//     waker: &Waker,
//     delay_ms: u64,
// ) {
//     let waker = waker.clone();

//     tokio::spawn(async move {
//         println!("waiting for {delay_ms}ms");

//         wait(delay_ms).await;

//         waker.wake();
//     });
// }

pub struct ChannelMock<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static = DuplexStream> {
    id: u16,
    label: String,
    channel: Pin<Box<TAsyncDuplex>>,
    options: ChannelMockOptions,
    read_delay_future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    write_delay_future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> ChannelMock<TAsyncDuplex> {
    pub fn new(
        channel: Box<TAsyncDuplex>,
        options: ChannelMockOptions,
    ) -> Box<dyn Channel> {
        return Box::new(
            ChannelMock {
                id: options.id,
                label: options.label.clone(),
                channel: Pin::new(channel),
                options,
                read_delay_future: None,
                write_delay_future: None,
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMockOptions {
    id: u16,
    label: String,
    throttle_min_ms: u64,
    throttle_max_ms: u64,
}

impl ChannelMockOptions {
    pub fn id(
        self,
        id: u16,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            id,
            ..self
        };
    }

    pub fn label(
        self,
        label: impl AsRef<str> + ToString,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            label: label.to_string(),
            ..self
        };
    }

    pub fn throttle_ms(
        self,
        throttle_ms: u64,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            throttle_min_ms: throttle_ms,
            throttle_max_ms: throttle_ms,
            ..self
        };
    }

    pub fn throttle_min_ms(
        self,
        throttle_ms: u64,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            throttle_min_ms: throttle_ms,
            ..self
        };
    }

    pub fn throttle_max_ms(
        self,
        throttle_ms: u64,
    ) -> ChannelMockOptions {
        return ChannelMockOptions {
            throttle_max_ms: throttle_ms,
            ..self
        };
    }

    pub fn get_throttle_value(&self) -> u64 {
        let timeout_min = self.throttle_min_ms;
        let timeout_max = self.throttle_max_ms;

        if timeout_max == 0 {
            return 0;
        }

        if timeout_min == timeout_max {
            return timeout_min;
        }

        return random_number(timeout_min..=timeout_max);
    }
}

impl Random for ChannelMockOptions {
    fn random() -> Self {
        return ChannelMockOptions::default()
            .throttle_min_ms(random_number(0..25))
            .throttle_max_ms(random_number(25..150));
    }
}

impl Default for ChannelMockOptions {
    fn default() -> ChannelMockOptions {
        return ChannelMockOptions {
            id: random_number(0..=u16::MAX),
            label: format!("channel-mock-{}", random_str(8)),
            throttle_min_ms: 0,
            throttle_max_ms: 0,
        };
    }
}

pub fn channel_mock_pair(
    options1: ChannelMockOptions,
    options2: ChannelMockOptions,
) -> (Box<dyn Channel>, Box<dyn Channel>) {
    let (channel1, channel2) = duplex(1024);

    return (
        ChannelMock::new(Box::new(channel1), options1),
        ChannelMock::new(Box::new(channel2), options2),
    );
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> Channel for ChannelMock<TAsyncDuplex> {
    fn id(&self) -> u16 {
        return self.id;
    }

    fn label(&self) ->  &String {
        return &self.label;
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncRead for ChannelMock<TAsyncDuplex> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // let id = self.id.clone();
        // if delay future present, wait until it completes
        if let Some(read_delay_future) = self.read_delay_future.as_mut() {
            // println!("[{}]> waiting", id);

            ready!(read_delay_future.as_mut().poll(cx));

            // println!("[{}]> ready", id);

            self.read_delay_future.take();
        }

        // otherwise run the read future to completion
        let result = ready!(self.channel.as_mut().poll_read(cx, buf));

        println!("[{}]> read some data: {:?}", self.id, result);

        // optionally create a throttle delay future
        if self.options.throttle_max_ms > 0 && random_bool() {
            let new_timeout = self.options.get_throttle_value();

            println!("[{}]> create new timeout of {} ms", self.id, new_timeout);
        
            self.read_delay_future = Some(Box::pin(wait(new_timeout)));
        }

        return Poll::Ready(result);
    }
}

impl<TAsyncDuplex: AsyncRead + AsyncWrite + Send + Unpin + 'static> AsyncWrite for ChannelMock<TAsyncDuplex> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // if delay future present, wait until it completes
        if let Some(write_delay_future) = self.write_delay_future.as_mut() {
            // println!("[{}]> waiting", id);

            ready!(write_delay_future.as_mut().poll(cx));

            // println!("[{}]> ready", id);

            self.write_delay_future.take();
        }

        let result = ready!(self.channel.as_mut().poll_write(cx, buf));

        // optionally create a throttle delay future
        if self.options.throttle_max_ms > 0 && random_bool() {
            let new_timeout = self.options.get_throttle_value();

            println!("[{}]> create new timeout of {} ms", self.id, new_timeout);
        
            self.write_delay_future = Some(Box::pin(wait(new_timeout)));
        }

        return Poll::Ready(result);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // if self.options.throttle_ms > 0 && random_bool() && random_bool() {
        //     throttle(cx.waker(), self.options.throttle_ms);

        //     return Poll::Pending;
        // }

        return self.channel.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // if self.options.throttle_ms > 0 && random_bool() && random_bool() {
        //     throttle(cx.waker(), self.options.throttle_ms);

        //     return Poll::Pending;
        // }

        return self.channel.as_mut()
            .poll_flush(cx);
    }
}
