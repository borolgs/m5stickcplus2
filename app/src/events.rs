use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

#[derive(Debug, Clone)]
pub enum Event {
    InitHat(StickHat),
    TabSelected(AppTab),
    Draw,
    StatsUpdated(Stats),
    ButtonDown(Button),
    ButtonUp(Button),
    Remote(Remote),
    JoyC(JoyC),
    LogAdded,
    Controller(Controller),
    Camera(Camera),
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

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    strum::EnumIter,
    strum::EnumCount,
    strum::FromRepr,
    strum::Display,
)]
pub enum AppTab {
    #[strum(to_string = "info")]
    Info,
    #[cfg(feature = "controller")]
    #[strum(to_string = "ctrl")]
    Controller,
    #[cfg(feature = "vehicle")]
    #[strum(to_string = "vehicle")]
    Vehicle,
    #[cfg(feature = "tv")]
    #[strum(to_string = "tv")]
    Remote,
    #[cfg(feature = "debug")]
    #[strum(to_string = "dev")]
    Dev,
}

impl AppTab {
    pub fn next(self) -> Self {
        Self::from_repr((self as usize + 1) % Self::iter().len()).unwrap()
    }

    pub fn prev(self) -> Self {
        let len = Self::iter().len();
        Self::from_repr((self as usize + len - 1) % len).unwrap()
    }

    pub fn titles() -> Vec<String> {
        Self::iter().map(|t| t.to_string()).collect::<Vec<_>>()
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Camera {
    CameraReady { address: [u8; 6] },
    ConnectToCamera { address: [u8; 6] },
    DisconnectFromCamera { address: [u8; 6] },
}

pub type Channel = PubSubChannel<CriticalSectionRawMutex, Event, 4, 4, 5>;
pub type Sender = Publisher<'static, CriticalSectionRawMutex, Event, 4, 4, 5>;
pub type Receiver = Subscriber<'static, CriticalSectionRawMutex, Event, 4, 4, 5>;

pub static EVENTS: Channel = Channel::new();
