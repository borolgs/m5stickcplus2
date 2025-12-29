#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

pub mod app;
pub mod events;
pub mod layout;
pub mod remote;

pub use app::App;
pub use events::*;
