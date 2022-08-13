pub mod types;

mod traits;
pub use traits::TUpgradableChannel;

mod channel;
pub use channel::UpgradableChannel;

pub mod mocks;

mod interleaved_channel;

mod utils;

pub fn buf_to_str(buf: &[u8]) -> String {
    return String::from_utf8(buf.to_vec()).unwrap();
}
