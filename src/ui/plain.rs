// Conserve backup system.
// Copyright 2015, 2016 Martin Pool.

//! Display log messages to stdout with no color or cursor movement,
//! perhaps for a log file.


use log;

use super::Counts;

/// Log in plain text to stdout.
pub struct TextLogger;

pub struct PlainUI;


impl TextLogger {
    pub fn new() -> Option<TextLogger> {
        Some(TextLogger)
    }
}

impl log::Log for TextLogger {
    fn enabled(&self, _metadata: &log::LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &log::LogRecord) {
        if ! self.enabled(record.metadata()) {
            return;
        }
        println!("{}", record.args());
    }
}


/// A plain text UI that prints log messages to stdout and does nothing about progress
/// counters.
impl PlainUI {
    /// Make a PlainUI.
    pub fn new() -> PlainUI {
        PlainUI {}
    }
}


impl super::UI for PlainUI {
    fn show_progress(&mut self, _counts: &Counts) {}

    /// Show a log message.
    fn log(&mut self, record: &log::LogRecord) {
        println!("{}", record.args());
    }
}
