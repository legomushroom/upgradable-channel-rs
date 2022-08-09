use anyhow::{anyhow, bail};
use cs_utils::{random_str, futures::wait};
use connection_utils::Channel;
use std::{pin::Pin, task::{Context, Poll}, io::{self, Error}};
use tokio::io::{AsyncRead, ReadBuf, AsyncWrite};

mod channel_states;
use channel_states::negotiate_channel_update;

mod channel_message;
pub use channel_message::ChannelMessage;

#[derive(Debug, Clone, PartialEq)]
enum ChannelState {
    Init,
    HandshakeStarted,
    HandshakeComplete,
    Upgraded,
    UpgradeFailed,
}

pub struct UpgradableChannel {
    id: String,
    channel1: Pin<Box<dyn Channel>>,
    channel1_msg: Option<Pin<Box<dyn Channel>>>,
    channel2: Option<Pin<Box<dyn Channel>>>,
    state: ChannelState,
}

impl UpgradableChannel {
    pub fn new(
        channel: Box<dyn Channel>,
        channel1_msg: Box<dyn Channel>, // temporary
    ) -> UpgradableChannel {
        return UpgradableChannel {
            id: random_str(32),
            channel1: Pin::new(channel),
            channel1_msg: Some(Pin::new(channel1_msg)),
            channel2: None,
            state: ChannelState::Init,
        };
    }

    pub async fn upgrade(
        &mut self,
        new_channel: Box<dyn Channel>,
    ) -> anyhow::Result<()> {
        let channel1_msg = match self.channel1_msg.take() {
            Some(channel) => channel,
            None => bail!("Contol message channel not found."),
        };

        self.state = ChannelState::HandshakeStarted;

        negotiate_channel_update(
            self.id.clone(),
            channel1_msg,
        ).await?;
        
        self.channel2.replace(Pin::new(new_channel));
        self.state = ChannelState::HandshakeComplete;

        // TODO: channel2 receives all writes
        // TODO: channel2 reads are buffered
        // TODO: read from channel1 only
        
        wait(50).await;

        self.state = ChannelState::Upgraded;

        return Ok(());
    }
}

impl AsyncRead for UpgradableChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.state == ChannelState::Upgraded {
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

impl AsyncWrite for UpgradableChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        return self.channel1.as_mut()
            .poll_write(cx, buf);
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        return self.channel1.as_mut()
            .poll_flush(cx);
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        return self.channel1.as_mut()
            .poll_shutdown(cx);
    }
}
