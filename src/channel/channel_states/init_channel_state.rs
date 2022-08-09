use std::pin::Pin;

use anyhow::bail;
use connection_utils::Channel;
use cs_utils::futures::GenericCodec;
use futures::{StreamExt, SinkExt};
use tokio_util::codec::Framed;

use crate::channel::ChannelMessage;

pub async fn negotiate_channel_update(
    id: String,
    channel1_msg: Pin<Box<dyn Channel>>,
) -> anyhow::Result<()> {
    let mut channel1_msg = Framed::new(
        channel1_msg,
        GenericCodec::<ChannelMessage>::new(),
    );

    loop {
        let message = match channel1_msg.next().await {
            Some(result) => result,
            None => bail!("Control channel closed."),
        }?;

        match message {
            ChannelMessage::Ping(ping_id) => {
                channel1_msg.send(ChannelMessage::Pong(ping_id)).await?;

                return Ok(());
            },
            ChannelMessage::Pong(pong_id) => {
                if id != pong_id {
                    bail!("Unknown PONG ID {:?}.", &pong_id);
                }

                return Ok(());
            },
        }
    }
}
