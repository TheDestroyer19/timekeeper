#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod database;
mod gui;
mod history;
mod settings;
pub use app::TimeKeeperApp;

use clap::{Parser, Subcommand};

pub const APP_NAME: &str = "TimeKeeper";

/// A utiltiy to help keep track of time spent working
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the stopwatch immediately
    Start,
}
