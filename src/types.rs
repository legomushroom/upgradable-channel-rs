use std::pin::Pin;

use connection_utils::Channel;
use tokio::io::{ReadHalf, WriteHalf};
use tokio_util::codec::Framed;
use cs_utils::futures::GenericCodec;

// TODO: move type to the `connection-utils` crate
pub type TFramedChannel<T> = Framed<Pin<Box<dyn Channel>>, GenericCodec<T>>;
pub type TReadHalf = Pin<Box<ReadHalf<Box<dyn Channel>>>>;
pub type TWriteHalf = Pin<Box<WriteHalf<Box<dyn Channel>>>>;