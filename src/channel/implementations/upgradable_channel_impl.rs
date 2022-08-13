use std::{sync::{Arc, Mutex}, pin::Pin, task::Waker};

use anyhow::{Result, bail};
use tokio_util::codec::Framed;
use cs_utils::{futures::{GenericCodec, wait}, random_str};
use futures::{SinkExt, StreamExt, select, FutureExt};
use connection_utils::{Channel, types::TFramedChannel};
use tokio::{io::{split, AsyncReadExt, AsyncWriteExt}, sync::oneshot::Receiver};

use crate::{channel::ChannelMessage, types::{TReadHalf, TWriteHalf}};

async fn handle_control_message(
    id: String,
    on_new_channel: Receiver<Box<dyn Channel>>,
    mut control_channel: TFramedChannel<ChannelMessage>,
    mut main_channel_reader: Arc<tokio::sync::Mutex<TReadHalf>>,
    mut main_channel_writer: Arc<tokio::sync::Mutex<TWriteHalf>>,
    channel2_reader: Arc<Mutex<Option<(TReadHalf, Vec<u8>)>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    last_read_waker: Arc<Mutex<Option<Waker>>>,
) -> Result<()> {
    let our_sync_id = random_str(32);

    let mut channel2_buffer = vec![];

    let mut reader = None;
    let mut writer = None;

    let mut on_new_channel_fused = on_new_channel.fuse();

    let mut is_syn_sent = false;

    async fn read_channel2(
        reader: &mut Option<TReadHalf>,
    ) -> Result<Option<Vec<u8>>> {
        let reader = match reader {
            Some(r) => r,
            None => return Ok(None),
        };

        let mut buf = [0; 1024];
        let bytes_read = reader.read(&mut buf).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let data = &buf[..bytes_read];

        return Ok(
            Some(data.to_vec()),
        );
    }

    loop {
        if writer.is_some() && !is_syn_sent {
            println!("[{}]> sending sync", id);
            control_channel.send(ChannelMessage::Sync(our_sync_id.clone())).await?;
            println!("[{}]> sync sent", id);

            is_syn_sent = true;
        }

        select! {
            maybe_channel2_data = read_channel2(&mut reader).fuse() => {
                let maybe_channel2_data = maybe_channel2_data?;

                // println!("[{}]> channel2 data?: {:?}", id, maybe_channel2_data.is_some());

                match maybe_channel2_data {
                    Some(channel2_data) => {
                        // println!("[{}]> got channel2 data:\n{:?}", id, buf_to_str(&channel2_data));
                        channel2_buffer.extend_from_slice(&channel2_data[..]);
                    }
                    None => wait(1).await,
                };
            },
            maybe_new_channel = on_new_channel_fused => {
                let new_channel = maybe_new_channel?;
                let (rx, tx) = split(new_channel);

                // println!("[{}]> got new channel", id);

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

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            {
                                println!("[{}][upgrade][sync]> shutting down", id);
    
                                main_channel_writer.lock().await
                                    .shutdown().await?;
    
                                println!("[{}][upgrade][sync]> shut down", id);
                            }

                            {
                                println!("[{}][upgrade][sync]> upgrade for writes", id);
                                channel2_writer.lock().unwrap()
                                    .replace(w);
                                println!("[{}][upgrade][sync]> upgraded for writes", id);
                            }
                        }

                        control_channel.send(sync_ack).await?;
                    },
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `reads` 
                    ChannelMessage::SyncAck(our_sync_id, their_sync_id) => {
                        if our_sync_id != our_sync_id {
                            bail!("SyncAck id mismatch.");
                        }

                        // upgrade for `writes`
                        if let Some(w) = writer.take() {
                            {
                                println!("[{}][upgrade][sync-ack]> shutting down", id);
    
                                main_channel_writer.lock().await
                                    .shutdown().await?;
    
                                println!("[{}][upgrade][sync-ack]> shut down", id);
                            }

                            {
                                println!("[{}][upgrade][sync-ack]> upgrade for writes", id);
                                channel2_writer.lock().unwrap()
                                    .replace(w);
                                println!("[{}][upgrade][sync-ack]> upgraded for writes", id);
                            }
                        }

                        wait(1).await;
                    
                        control_channel.send(ChannelMessage::Ack(their_sync_id, our_sync_id)).await?;

                        wait(1).await;

                        // wait for `T1` period before fully upgrading to `channel2`
                        // wait(250).await;

                        // // upgrade for `reads`
                        // if let Some(r) = reader.take() {
                        //     println!("[{}][upgrade][sync-ack]> upgrade for reads", id);
                        //     channel2_reader.lock().unwrap()
                        //         .replace((r, channel2_buffer));
                        //     println!("[{}][upgrade][sync-ack]> upgraded for reads", id);
                        // }

                        // fully upgraded
                        // return Ok(());
                    },
                    // if message is `Sync`, respond with `SyncAck`, that will
                    // signify moving an upgrade to the new channel for all `reads` 
                    ChannelMessage::Ack(sync_id, their_sync_id) => {
                        if sync_id != our_sync_id {
                            bail!("Ack id mismatch.");
                        }

                        wait(1).await;

                        control_channel.send(ChannelMessage::Ack(their_sync_id, our_sync_id)).await?;

                        wait(1).await;

                        if let Some(waker) = last_read_waker.lock().unwrap().take() {
                            println!("[{}][upgrade][ack]> waking up before upgrade", id);

                            waker.wake();
                        }

                        wait(1).await;

                        // // wait for `T1` period before fully upgrading to `channel2`
                        // wait(250).await;

                        // upgrade for `reads`
                        if let Some(r) = reader.take() {
                            {
                                println!("[{}][upgrade][ack]> reading all", id);
                                
                                let mut data = vec![];
                                main_channel_reader.lock().await
                                    .read_to_end(&mut data).await?;

                                println!("[{}][upgrade][ack]> read {} bytes", id, data.len());

                                // prepend data to the `channel2_buffer`
                                data.extend_from_slice(&channel2_buffer[..]);
                                channel2_buffer = data;

                                println!("[{}][upgrade][ack]> read all", id);
                            }

                            {
                                println!("[{}][upgrade][ack]> upgrade for reads", id);

                                channel2_reader.lock().unwrap()
                                    .replace((r, channel2_buffer));

                                println!("[{}][upgrade][ack]> upgraded for reads", id);
                            }
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
    main_channel_reader: Arc<tokio::sync::Mutex<TReadHalf>>,
    main_channel_writer: Arc<tokio::sync::Mutex<TWriteHalf>>,
    channel2_reader: Arc<Mutex<Option<(TReadHalf, Vec<u8>)>>>,
    channel2_writer: Arc<Mutex<Option<TWriteHalf>>>,
    last_read_waker: Arc<Mutex<Option<Waker>>>,
) -> Result<()> {
    // create control message channel stream
    let control_channel = Framed::new(
        Pin::new(control_channel),
        GenericCodec::<ChannelMessage>::new(),
    );

    let _res = handle_control_message(
        id.clone(),
        on_new_channel,
        control_channel,
        main_channel_reader,
        main_channel_writer,
        channel2_reader,
        channel2_writer,
        Arc::clone(&last_read_waker),
    ).await;

    println!("[{}]> handle_control_message returned: {:?}", id, _res);

    wait(1).await;
    
    let mut lock = last_read_waker.lock().unwrap();
    if let Some(waker) = lock.take() {
        println!("[{}]> waking up", id);
        waker.wake();
    }

    return Ok(());
}
