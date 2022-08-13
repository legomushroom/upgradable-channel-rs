#[cfg(test)] // TODO: remove?
mod create_framed_stream;
#[cfg(test)] // TODO: remove?
pub use create_framed_stream::create_framed_stream;

#[cfg(test)] // TODO: remove?
mod test_framed_stream;
#[cfg(test)] // TODO: remove?
pub use test_framed_stream::{test_framed_stream, TestOptions, StreamTestMessage};
