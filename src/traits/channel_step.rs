use anyhow::Result;

pub trait ChannelStep {
    fn run(&mut self) -> Result<Box<dyn Channel>>;
}
