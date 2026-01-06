#![no_std]

extern crate alloc;

pub mod battery;
pub mod button;

#[cfg(feature = "ir")]
pub mod ir;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "now")]
pub mod now;

#[cfg(feature = "vehicle")]
pub mod vehicle;

pub mod minijoyc;
