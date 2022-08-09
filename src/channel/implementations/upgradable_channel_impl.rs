use std::pin::Pin;

use anyhow::{Result, bail};
use async_trait::async_trait;
use connection_utils::Channel;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use cs_utils::futures::GenericCodec;

use crate::{traits::TUpgradableChannel, channel::{UpgradableChannel, ChannelMessage}};

#[async_trait]
impl TUpgradableChannel for UpgradableChannel {
    fn is_upgrdaded(&self) -> bool {
        return self.is_upgraded_writes && self.is_upgraded_reads;
    }

    async fn upgrade(
        &mut self,
        new_channel: Box<dyn Channel>,
    ) -> Result<()> {
        // save `channel2` instance for later use
        self.channel2.replace(Pin::new(new_channel));

        // create control message channel stream
        let mut channel1_msg = match self.channel1_msg.take() {
            Some(channel) => {
                Framed::new(
                    channel,
                    GenericCodec::<ChannelMessage>::new(),
                )
            },
            None => bail!("Contol message channel not found."),
        };

        // send Sync immediately
        channel1_msg.send(ChannelMessage::Sync(self.sync_id.clone())).await?;

        loop {
            // get next message
            let message = match channel1_msg.next().await {
                Some(result) => result,
                None => bail!("Control channel closed."),
            }?;

            match message {
                // if message is `Sync`, respond with `SyncAck`, that will
                // signify moving an upgrade to the new channel for all `writes` 
                ChannelMessage::Sync(sync_id) => {
                    channel1_msg.send(ChannelMessage::SyncAck(sync_id)).await?;

                    // upgrade for `writes`
                    self.is_upgraded_writes = true;

                    // fully upgraded
                    if self.is_upgrdaded() {
                        return Ok(());
                    }
                },
                // if message is `Sync`, respond with `SyncAck`, that will
                // signify moving an upgrade to the new channel for all `reads` 
                ChannelMessage::SyncAck(sync_id) => {
                    if sync_id != self.sync_id {
                        bail!("SyncAck id mismatch.");
                    }

                    channel1_msg.send(ChannelMessage::SyncAck(sync_id)).await?;

                    // upgrade for `reads`
                    self.is_upgraded_reads = true;

                    // fully upgraded
                    if self.is_upgrdaded() {
                        return Ok(());
                    }
                },
            };
        };
    }
}
