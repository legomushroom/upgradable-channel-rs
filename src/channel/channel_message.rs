use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ChannelMessage {
    Sync(String),
    SyncAck(String),
}
