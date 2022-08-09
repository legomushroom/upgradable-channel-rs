pub mod types;

mod traits;
pub use traits::TUpgradableChannel;

mod channel;
pub use channel::UpgradableChannel;

pub mod mocks;
