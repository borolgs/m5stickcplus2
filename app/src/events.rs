#[cfg(not(feature = "std"))]
use alloc::string::String;

use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

#[derive(Debug, Clone)]
pub enum Event {
    Init { version: String },
    BatteryLevelUpdated { level: u8 },
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

pub type Channel = PubSubChannel<CriticalSectionRawMutex, Event, 4, 4, 4>;
pub type Sender = Publisher<'static, CriticalSectionRawMutex, Event, 4, 4, 4>;
pub type Receiver = Subscriber<'static, CriticalSectionRawMutex, Event, 4, 4, 4>;

pub static EVENTS: Channel = Channel::new();
