#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let subscriber = tracing_subscriber::fmt()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|_e| eprintln!("Unable to set default subscriber"));
    
    tracing::info!("Hello world!");
    let app = timekeeper::TimeKeeperApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
