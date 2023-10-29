use std::thread;

use chrono::{Duration, Weekday};
use eframe::egui;
use serde::{Serialize, Deserialize};
use tracing::warn;

use crate::gui::{draw_stopwatch, GuiState};
use crate::history::History;
use crate::stopwatch::StopWatch;
use crate::{SETTINGS_KEY, STATE_KEY};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Duration")]
struct DurationDef {
    #[serde(getter="Duration::num_seconds")]
    secs: i64
}
impl From<DurationDef> for Duration {
    fn from(d: DurationDef) -> Self {
        Duration::seconds(d.secs)
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Settings {
    pub week_format: String,
    pub date_format: String,
    pub time_format: String,

    pub start_of_week: Weekday,

    #[serde(with = "DurationDef")]
    pub daily_goal: Duration,
    #[serde(with = "DurationDef")]
    pub weekly_goal: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            week_format: "Week of %B %e, %Y".into(),
            date_format: "%y-%m-%d".into(),
            time_format: "%H:%M".into(),
            start_of_week: Weekday::Mon,
            daily_goal: Duration::hours(8),
            weekly_goal: Duration::hours(40),
        }
    }
}

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
                self.state
                    .draw_screen(&mut self.stopwatch, &mut self.history, &mut self.settings, ui)
            })
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(STATE_KEY, serde_json::to_string(&self.state).unwrap());
        storage.set_string(SETTINGS_KEY, serde_json::to_string(&self.settings).unwrap());
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
            if let Some(settings) = storage.get_string(SETTINGS_KEY) {
                match serde_json::from_str(&settings) {
                    Ok(value) => r.settings = value,
                    Err(e) => warn!("Failed to read settings: {:?}", e),
                };
            }
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
