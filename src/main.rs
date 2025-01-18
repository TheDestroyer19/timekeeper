#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use timekeeper::APP_NAME;
    use tracing::{info, warn};

    let subscriber = tracing_subscriber::fmt().finish();

    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|_e| eprintln!("Unable to set default subscriber"));

    tracing::info!("Starting up");
    let native_options = eframe::NativeOptions::default();
    let finish = eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(timekeeper::TimeKeeperApp::new(cc)))),
    );

    if let Err(e) = finish {
        warn!("App exited with error: {:?}", e);
    } else {
        info!("Shut down gracefully");
    }
}
