use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Event {
    InitHat(StickHat),
    Draw,
    StatsUpdated(Stats),
    ButtonDown(Button),
    ButtonUp(Button),
    Remote(Remote),
    JoyC(JoyC),
    LogAdded,
    Controller(Controller),
    Vehicle(Vehicle),
}

#[derive(Debug, Clone, Copy)]
pub enum StickHat {
    MiniJoyC,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub battery_level: u8,
    pub heap_used: usize,
    pub heap_free: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoyC {
    Button,
    Arrow(JoycDirection),
    Pos((i8, i8)),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Controller {
    Move(i8, i8),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Vehicle {
    Move(i8, i8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoycDirection {
    Up,
    Right,
    Down,
    Left,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::EnumCount, strum::FromRepr)]
pub enum Remote {
    OnOff,
    Home,
    Back,
    Ok,
    Up,
    Right,
    Down,
    Left,
    Mute,
    VolumeUp,
    VolumeDown,
}

pub type Channel = PubSubChannel<CriticalSectionRawMutex, Event, 4, 4, 4>;
pub type Sender = Publisher<'static, CriticalSectionRawMutex, Event, 4, 4, 4>;
pub type Receiver = Subscriber<'static, CriticalSectionRawMutex, Event, 4, 4, 4>;

pub static EVENTS: Channel = Channel::new();
