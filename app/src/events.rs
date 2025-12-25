#[cfg(not(feature = "std"))]
use alloc::string::String;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

#[derive(Debug, Clone)]
pub enum Event {
    Init { version: String },
    ButtonDown(Button),
    ButtonUp(Button),
    Remote(Remote),
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy)]
pub enum Remote {
    OnOff,
}

pub type Channel = PubSubChannel<NoopRawMutex, Event, 4, 4, 4>;
pub type Sender = Publisher<'static, NoopRawMutex, Event, 4, 4, 4>;
pub type Receiver = Subscriber<'static, NoopRawMutex, Event, 4, 4, 4>;

pub fn channel() -> Channel {
    Channel::new()
}
