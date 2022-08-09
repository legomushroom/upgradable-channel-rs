use std::pin::Pin;

use connection_utils::Channel;
use tokio_util::codec::Framed;
use cs_utils::futures::GenericCodec;

// TODO: move type to the `connection-utils` crate
pub type FramedChannel<T> = Framed<Pin<Box<dyn Channel>>, GenericCodec<T>>;
