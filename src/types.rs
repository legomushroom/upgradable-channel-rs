use std::pin::Pin;

use connection_utils::Channel;
use tokio::io::{ReadHalf, WriteHalf};

pub type TReadHalf = Pin<Box<ReadHalf<Box<dyn Channel>>>>;
pub type TWriteHalf = Pin<Box<WriteHalf<Box<dyn Channel>>>>;
