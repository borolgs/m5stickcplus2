#[cfg(not(feature = "std"))]
use alloc::{collections::vec_deque::VecDeque, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{collections::VecDeque, string::String, vec::Vec};

use crate::events::{EVENTS, Event};
use core::cell::RefCell;
use critical_section::Mutex;
use log::Level;

static LOGGER: Logger = Logger;
const LOGGER_CAPACITY: usize = 20;
static LOGS: Mutex<RefCell<Option<VecDeque<(Level, String)>>>> = Mutex::new(RefCell::new(None));

pub struct Logger;

pub fn init() {
    _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug));
    critical_section::with(|cs| {
        LOGS.borrow_ref_mut(cs)
            .replace(VecDeque::with_capacity(LOGGER_CAPACITY));
    });
}

pub fn latest_log_lines(limit: usize) -> Vec<(Level, String)> {
    critical_section::with(|cs| {
        LOGS.borrow_ref(cs)
            .as_ref()
            .map(|logs| {
                logs.iter()
                    .rev()
                    .take(limit)
                    .rev()
                    .map(|(level, msg)| (level.clone(), msg.clone()))
                    .collect()
            })
            .unwrap_or_default()
    })
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());
            #[allow(unused)]
            let (color, reset) = match record.level() {
                Level::Error => ("\x1b[31m", "\x1b[0m"), // red
                Level::Warn => ("\x1b[33m", "\x1b[0m"),  // yellow
                Level::Info => ("\x1b[32m", "\x1b[0m"),  // green
                Level::Debug => ("\x1b[34m", "\x1b[0m"), // blue
                Level::Trace => ("\x1b[35m", "\x1b[0m"), // magenta
            };

            #[cfg(feature = "alloc")]
            esp_println::println!("{}[{}]{} {}", color, record.level(), reset, &msg);
            #[cfg(feature = "std")]
            println!("{}[{}]{} {}", color, record.level(), reset, &msg);

            critical_section::with(|cs| {
                if let Some(logs) = LOGS.borrow_ref_mut(cs).as_mut() {
                    if logs.len() == LOGGER_CAPACITY {
                        logs.pop_front();
                    }
                    logs.push_back((record.level(), msg));
                }
            });
            _ = EVENTS.immediate_publisher().try_publish(Event::LogAdded);
        }
    }

    fn flush(&self) {}
}
