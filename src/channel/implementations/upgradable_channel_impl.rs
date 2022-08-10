use std::pin::Pin;

use anyhow::{Result, bail};
use tokio::io::AsyncReadExt;
use async_trait::async_trait;
use tokio_util::codec::Framed;
use connection_utils::Channel;
use cs_utils::futures::GenericCodec;
use futures::{SinkExt, StreamExt, select, FutureExt};

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
        let mut channel2 = Pin::new(new_channel);

        // create control message channel stream
        let mut channel1_msg = match self.channel_msg.take() {
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

        // channel2 read buffer
        let mut buf = [0; 1024];
        loop {
            select! {
                // read and buffer data from the channel2 until the upgrade
                // to the new channel is complete
                maybe_bytes_read = channel2.read(&mut buf).fuse() => {
                    let bytes_read = maybe_bytes_read?;
                    
                    self.channel2_buffer.extend_from_slice(&buf[..bytes_read]);

                    continue;
                },
                // negotiate upgrade to the new channel
                message = channel1_msg.next().fuse() => {
                    // get next message
                    let message = match message {
                        Some(result) => result,
                        None => bail!("Control channel closed."),
                    }?;

                    match message {
                        // if message is `Sync`, respond with `SyncAck`, that will
                        // signify moving an upgrade to the new channel for all `writes` 
                        ChannelMessage::Sync(sync_id) => {
                            let sync_ack = ChannelMessage::SyncAck(sync_id, self.sync_id.clone());
                            channel1_msg.send(sync_ack).await?;

                            // upgrade for `writes`
                            self.is_upgraded_writes = true;

                            // fully upgraded
                            if self.is_upgrdaded() {
                                self.channel2.replace(channel2);

                                return Ok(());
                            }
                        },
                        // if message is `Sync`, respond with `SyncAck`, that will
                        // signify moving an upgrade to the new channel for all `reads` 
                        ChannelMessage::SyncAck(our_sync_id, their_sync_id) => {
                            if our_sync_id != self.sync_id {
                                bail!("SyncAck id mismatch.");
                            }

                            // upgrade for `reads`
                            self.is_upgraded_reads = true;

                            channel1_msg.send(ChannelMessage::Ack(their_sync_id)).await?;

                            // upgrade for `writes`
                            self.is_upgraded_writes = true;

                            // fully upgraded
                            if self.is_upgrdaded() {
                                self.channel2.replace(channel2);

                                return Ok(());
                            }
                        },
                        // if message is `Sync`, respond with `SyncAck`, that will
                        // signify moving an upgrade to the new channel for all `reads` 
                        ChannelMessage::Ack(sync_id) => {
                            if sync_id != self.sync_id {
                                bail!("Ack id mismatch.");
                            }

                            // upgrade for `reads`
                            self.is_upgraded_reads = true;

                            // fully upgraded
                            if self.is_upgrdaded() {
                                self.channel2.replace(channel2);

                                return Ok(());
                            }
                        },
                    };
                },
            }
        }
    }
}
