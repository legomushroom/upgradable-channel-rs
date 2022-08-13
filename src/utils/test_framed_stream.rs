use futures::{StreamExt, SinkExt};
use std::{fmt::Debug, ops::RangeInclusive};
use connection_utils::types::TFramedChannel;
use serde::{Serialize, de::DeserializeOwned, Deserialize};
use cs_utils::{futures::wait_random, random_number, traits::Random, random_str, random_str_rg, test::random_vec};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StreamTestMessage {
    Ping(String),
    Pong(Option<String>, String),
    Data(Vec<u8>),
    Eof,
}

impl Random for StreamTestMessage {
    fn random() -> StreamTestMessage {
        return match random_number(0..=u16::MAX) % 4 {
            0 => StreamTestMessage::Ping(random_str_rg(16..=32)),
            1 => StreamTestMessage::Pong(Some(random_str(32)), random_str_rg(32..=64)),
            2 => StreamTestMessage::Data(random_str_rg(8..=256).as_bytes().to_vec()),
            3 => StreamTestMessage::Eof,
            _ => unreachable!(),
        };
    }
}

pub struct TestOptions {
    items_count: u32,
    throttle_range: RangeInclusive<u64>,
}

impl TestOptions {
    #[allow(unused)] // TODO: remove
    pub fn throttle(
        self,
        throttle_range: RangeInclusive<u64>,
    ) -> TestOptions {
        return TestOptions {
            throttle_range,
            ..self
        }
    }

    pub fn items_count(
        self,
        items_count: u32,
    ) -> TestOptions {
        return TestOptions {
            items_count,
            ..self
        }
    }
}

impl Default for TestOptions {
    fn default() -> TestOptions {
        return TestOptions {
            items_count: 32,
            throttle_range: (0..=0),
        };
    }
}

impl Random for TestOptions {
    fn random() -> TestOptions {
        let min: u64 = random_number(0..5);
        let max: u64 = random_number(5..50);
    
        return TestOptions {
            items_count: random_number(8..=256),
            throttle_range: (min..=max),
        };
    }
}

// TODO: move to the `connection-utils` crate
pub async fn test_framed_stream<T: Serialize + DeserializeOwned + PartialEq + Clone + Debug + Random>(
    stream1: TFramedChannel<T>,
    stream2: TFramedChannel<T>,
    options: TestOptions,
) -> (TFramedChannel<T>, TFramedChannel<T>, TestOptions) {
    // test in backward direction
    let (stream2, stream1, options) = run_framed_stream_test(stream2, stream1, options).await;
    // test in forward direction
    return run_framed_stream_test(stream1, stream2, options).await;
}

async fn run_framed_stream_test<T: Serialize + DeserializeOwned + PartialEq + Clone + Debug + Random>(
    mut stream1: TFramedChannel<T>,
    mut stream2: TFramedChannel<T>,
    options: TestOptions,
) -> (TFramedChannel<T>, TFramedChannel<T>, TestOptions) {
    let data_len = options.items_count;
    let data = random_vec::<T>(data_len);

    let throttle_range1 = options.throttle_range.clone();
    let throttle_range2 = options.throttle_range.clone();

    let ((stream1, data), (stream2, received_data)) = tokio::join!(
        Box::pin(async move {
            for message in &data {
                wait_random(throttle_range1.clone()).await;
    
                stream1.send(message.clone()).await.unwrap();
            }
    
            return (stream1, data);
        }),
        Box::pin(async move {
            let mut received_data = vec![];
    
            loop {
                wait_random(throttle_range2.clone()).await;
    
                let message = stream2.next().await
                    .expect("Stream closed.")
                    .expect("Cannot read stream message.");
    
                received_data.push(message);
    
                if received_data.len() == data_len as usize {
                    break;
                }
            }
    
            return (stream2, received_data);
        }),
    );

    assert_eq!(
        data,
        received_data,
        "Sent and received data must match.",
    );

    return (stream1, stream2, options);
}
