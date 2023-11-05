use std::thread;

use chrono::Duration;
use eframe::egui;
use tracing::warn;

use crate::gui::{draw_stopwatch, GuiState};
use crate::history::History;
use crate::settings::Settings;
use crate::stopwatch::StopWatch;

const SETTINGS_KEY: &str = "Settings";
const STATE_KEY: &str = "State";

#[derive(Default)]
pub struct TimeKeeperApp {
    state: GuiState,
    settings: Settings,
    stopwatch: StopWatch,
    history: History,
}

impl eframe::App for TimeKeeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            self.state.draw_tabs(ui);
        });

        egui::TopBottomPanel::bottom("stopwatch").show(ctx, |ui| {
            draw_stopwatch(&mut self.stopwatch, &mut self.history, &self.settings, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.state.draw_screen(
                    &mut self.stopwatch,
                    &mut self.history,
                    &mut self.settings,
                    ui,
                )
            })
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(STATE_KEY, serde_json::to_string(&self.state).unwrap());
        storage.set_string(SETTINGS_KEY, self.settings.serialize());
    }
}

impl TimeKeeperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut r = Self::default();

        // load previous state if any
        if let Some(storage) = cc.storage {
            r.settings = Settings::deserailize(storage.get_string(SETTINGS_KEY));
            if let Some(state) = storage.get_string(STATE_KEY) {
                match serde_json::from_str(&state) {
                    Ok(value) => r.state = value,
                    Err(e) => warn!("Failed to read state: {:?}", e),
                };
            }
        }

        //start update thread
        let ctx = cc.egui_ctx.clone();
        thread::spawn(|| bg_timer(ctx));

        r
    }
}

/// thread to update the gui regularly.
/// This could be improved to only do it while the timer is active and the window is visible
fn bg_timer(frame: egui::Context) {
    let one_second = Duration::seconds(1)
        .to_std()
        .expect("1 second should be in range");
    loop {
        thread::sleep(one_second);
        frame.request_repaint();
    }
}
