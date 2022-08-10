use std::sync::Arc;

use anyhow::{Result, bail};
use tokio::{io::split, sync::Mutex};
use tokio_util::codec::Framed;
use connection_utils::Channel;
use cs_utils::futures::GenericCodec;
use futures::{SinkExt, StreamExt, select, FutureExt};

use crate::{traits::TUpgradableChannel, channel::{UpgradableChannel, ChannelMessage}, types::{TFramedChannel, TReadHalf, TWriteHalf}};

async fn handle_control_message(
    our_sync_id: impl AsRef<str> + ToString,
    mut control_channel: TFramedChannel<ChannelMessage>,
    reader: TReadHalf,
    writer: TWriteHalf,
    channel2_reader: Arc<Mutex<Option<TReadHalf>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
) -> Result<()> {
    let our_sync_id = our_sync_id.to_string();

    let mut reader = Some(reader);
    let mut writer = Some(writer);

    // send Sync immediately
    control_channel.send(ChannelMessage::Sync(our_sync_id.clone())).await?;

    // channel2 read buffer
    // let mut buf = [0; 1024];
    loop {
        select! {
            // read and buffer data from the channel2 until the upgrade
            // to the news complete
            // maybe_bytes_read = reader.read(&mut buf).fuse() => {
            //     let bytes_read = maybe_bytes_read?;
                
            //     channel2_buffer.extend_from_slice(&buf[..bytes_read]);

            //     continue;
            // },
            // negotiate upgrade to the new channel
            message = control_channel.next().fuse() => {
                // get next message
                let message = match message {
                    Some(result) => result,
                    None => bail!("Control channel closed."),
                }?;

                match message {
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `writes` 
                    ChannelMessage::Sync(sync_id) => {
                        let sync_ack = ChannelMessage::SyncAck(sync_id, our_sync_id.clone());
                        control_channel.send(sync_ack).await?;

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            channel2_writer.lock().await
                                .replace(w);
                        }
                    },
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `reads` 
                    ChannelMessage::SyncAck(our_sync_id, their_sync_id) => {
                        if our_sync_id != our_sync_id {
                            bail!("SyncAck id mismatch.");
                        }

                        // upgrade for `reads`
                        if let Some(r) = reader.take() {
                            channel2_reader.lock().await
                                .replace(r);
                        }
                       
                        control_channel.send(ChannelMessage::Ack(their_sync_id)).await?;

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            channel2_writer.lock().await
                                .replace(w);
                        }

                        // fully upgraded
                        return Ok(());
                    },
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `reads` 
                    ChannelMessage::Ack(sync_id) => {
                        if sync_id != our_sync_id {
                            bail!("Ack id mismatch.");
                        }

                        // upgrade for `reads`
                        if let Some(r) = reader.take() {
                            channel2_reader.lock().await
                                .replace(r);
                        }

                        return Ok(());
                    },
                };
            },
        }
    }
}

impl TUpgradableChannel for UpgradableChannel {
    // fn is_upgrdaded(&self) -> bool {
    //     return self.is_upgraded_writes && self.is_upgraded_reads;
    // }

    fn upgrade(
        &mut self,
        new_channel: Box<dyn Channel>,
    ) -> Result<()> {
        // create control message channel stream
        let control_channel = match self.channel_msg.take() {
            Some(channel) => {
                Framed::new(
                    channel,
                    GenericCodec::<ChannelMessage>::new(),
                )
            },
            None => bail!("Contol message channel not found."),
        };

        let (reader, writer) = split(new_channel);

        tokio::spawn(handle_control_message(
            self.sync_id.clone(),
            control_channel,
            Box::pin(reader),
            Box::pin(writer),
            Arc::clone(&self.channel2_reader),
            Arc::clone(&self.channel2_writer),

        ));

        return Ok(());
    }
}
