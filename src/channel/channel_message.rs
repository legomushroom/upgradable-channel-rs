use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ChannelMessage {
    Ping(String),
    Pong(String),
}
