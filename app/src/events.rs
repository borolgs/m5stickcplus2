use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

pub type Channel = PubSubChannel<NoopRawMutex, Event, 4, 4, 4>;
pub type Sender = Publisher<'static, NoopRawMutex, Event, 4, 4, 4>;
pub type Receiver = Subscriber<'static, NoopRawMutex, Event, 4, 4, 4>;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    ButtonDown(Button),
    ButtonUp(Button),
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    A,
    B,
    C,
}

pub fn channel() -> Channel {
    Channel::new()
}
