use std::pin::Pin;

use serde::{Serialize, de::DeserializeOwned};
use tokio_util::codec::Framed;
use cs_utils::futures::GenericCodec;
use connection_utils::{Channel, types::TFramedChannel};

// TODO: move to the `conntetion-utils` crate
pub fn create_framed_stream<T: Serialize + DeserializeOwned>(
    channel: Box<dyn Channel>,
) -> TFramedChannel<T> {
    return Framed::new(
        Pin::new(channel),
        GenericCodec::<T>::new(),
    );
}
