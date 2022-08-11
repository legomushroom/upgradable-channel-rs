use std::fmt::Debug;
use connection_utils::types::TFramedChannel;
use cs_utils::{random_bool, futures::wait_random};
use futures::{StreamExt, SinkExt};

use serde::{Serialize, de::DeserializeOwned};

// TODO: move to the `connection-utils` crate
pub async fn test_framed_stream<T: Serialize + DeserializeOwned + PartialEq + Clone + Debug>(
    mut stream1: TFramedChannel<T>,
    mut stream2: TFramedChannel<T>,
    data: Vec<T>,
) {
    let data_len = data.len();

    let future1 = Box::pin(async move {
        for message in &data {
            if random_bool() {
                wait_random(5..=150).await;
            }

            stream1.send(message.clone()).await.unwrap();
        }

        return data;
    });

    let future2 = Box::pin(async move {
        let mut received_data = vec![];

        loop {
            if random_bool() {
                wait_random(5..=150).await;
            }

            let message = stream2.next().await
                .expect("Stream closed.")
                .expect("Cannot read stream message.");

            received_data.push(message);

            if received_data.len() == data_len {
                break;
            }
        }

        return received_data;
    });

    let (result1, result2) = tokio::join!(
        future1,
        future2,
    );

    assert_eq!(
        result1,
        result2,
        "Sent and received data must match.",
    );
}
