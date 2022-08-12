use std::{sync::{Arc, Mutex}, pin::Pin, task::Waker};

use anyhow::{Result, bail};
use tokio_util::codec::Framed;
use connection_utils::{Channel, types::TFramedChannel};
use cs_utils::{futures::GenericCodec, random_str};
use futures::{SinkExt, StreamExt, select, FutureExt};
use tokio::{io::split, sync::{oneshot::Receiver, mpsc::Sender}};

use crate::{channel::ChannelMessage, types::{TReadHalf, TWriteHalf}};

async fn handle_control_message(
    id: String,
    on_new_channel: Receiver<Box<dyn Channel>>,
    mut control_channel: TFramedChannel<ChannelMessage>,
    channel2_reader: Arc<Mutex<Option<TReadHalf>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    // channel2_buffer_sender: Sender<Vec<u8>>,
) -> Result<()> {
    let our_sync_id = random_str(32);

    let mut reader = None;
    let mut writer = None;

    let mut on_new_channel_fused = on_new_channel.fuse();

    let mut is_syn_sent = false;

    // channel2 read buffer
    // let mut buf = [0; 1024];
    loop {
        if writer.is_some() && !is_syn_sent {
            println!("[{}]> sending sync", id);
            control_channel.send(ChannelMessage::Sync(our_sync_id.clone())).await?;
            println!("[{}]> sync sent", id);

            is_syn_sent = true;
        }

        select! {
            maybe_new_channel = on_new_channel_fused => {
                let new_channel = maybe_new_channel?;
                let (rx, tx) = split(new_channel);

                println!("[{}]> got new channel", id);

                reader.replace(Box::pin(rx));
                writer.replace(Box::pin(tx));
            },
            message = control_channel.next().fuse() => {
                // get next message
                let message = match message {
                    Some(result) => result,
                    None => bail!("Control channel closed."),
                }?;

                println!("[{}]> got new message: {:?}", id, message);

                match message {
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `writes` 
                    ChannelMessage::Sync(sync_id) => {
                        let sync_ack = ChannelMessage::SyncAck(sync_id, our_sync_id.clone());

                        if !writer.is_some() {
                            continue;
                        }

                        control_channel.send(sync_ack).await?;

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            println!("[{}][upgrade][sync]> upgrade for writes", id);
                            channel2_writer.lock().unwrap()
                                .replace(w);
                            println!("[{}][upgrade][sync]> upgraded for writes", id);

                        }
                        // println!("[{}][upgrade][sync]> writer unlock", id);
                    },
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `reads` 
                    ChannelMessage::SyncAck(our_sync_id, their_sync_id) => {
                        if our_sync_id != our_sync_id {
                            bail!("SyncAck id mismatch.");
                        }

                        // upgrade for `reads`
                        if let Some(r) = reader.take() {
                            println!("[{}][upgrade][sync-ack]> upgrade for reads", id);
                            channel2_reader.lock().unwrap()
                                .replace(r);
                            println!("[{}][upgrade][sync-ack]> upgraded for reads", id);
                        }
                        // println!("[{}][upgrade][sync-ack]> reader unlock", id);
                    
                        control_channel.send(ChannelMessage::Ack(their_sync_id)).await?;

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            println!("[{}][upgrade][sync-ack]> upgrade for writes", id);
                            channel2_writer.lock().unwrap()
                                .replace(w);
                            println!("[{}][upgrade][sync-ack]> upgraded for writes", id);
                        }
                        // println!("[{}][upgrade][sync-ack]> writer unlock", id);

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
                            println!("[{}][upgrade][ack]> upgrade for reads", id);

                            channel2_reader.lock().unwrap()
                                .replace(r);

                            println!("[{}][upgrade][ack]> upgraded for reads", id);
                        }
                        // println!("[{}][upgrade][ack]> reader unlock", id);

                        return Ok(());
                    },
                };
            }
        }
    }
}

pub async fn handle_upgrade(
    id: String,
    on_new_channel: Receiver<Box<dyn Channel>>,
    control_channel: Box<dyn Channel>,
    channel2_reader: Arc<Mutex<Option<TReadHalf>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    // channel2_buffer_sender: Sender<Vec<u8>>,
    last_read_waker: Arc<Mutex<Option<Waker>>>,
) -> Result<()> {
    // create control message channel stream
    let control_channel = Framed::new(
        Pin::new(control_channel),
        GenericCodec::<ChannelMessage>::new(),
    );

    let _res = handle_control_message(
        id,
        on_new_channel,
        control_channel,
        channel2_reader,
        channel2_writer,
        // channel2_buffer_sender,
    ).await;

    let mut lock = last_read_waker.lock().unwrap();
    if let Some(waker) = lock.take() {
        waker.wake();
    }

    println!("handle_control_message returned: {:?}", _res);

    return Ok(());
}
