#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod gui;
mod stopwatch;
mod history;
mod database;
pub use app::TimeKeeperApp;

pub const APP_NAME: &str = "TimeKeeper";
const SETTINGS_KEY: &str = "Settings";
const STATE_KEY: &str = "State";