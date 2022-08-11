use connection_utils::Channel;

use crate::channel::UpgradableChannel;

impl Channel for UpgradableChannel {
    fn id(&self) -> u16 {
        // TODO: should be dependend on channel2 too?
        return self.main_channel.id();
    }

    fn label(&self) ->  &String {
        // TODO: should be dependend on channel2 too?
        return self.main_channel.label();
    }
}
