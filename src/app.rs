use std::thread;

use chrono::{Duration, Weekday};
use eframe::{egui, epi};

use crate::gui::{draw_stopwatch, GuiState};
use crate::stopwatch::StopWatch;
use crate::{APP_NAME, SETTINGS_KEY, STATE_KEY};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Settings {
    pub week_format: String,
    pub date_format: String,
    pub time_format: String,

    pub start_of_week: Weekday,

    pub daily_target_hours: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            week_format: "Week of %B %e, %Y".into(),
            date_format: "%y-%m-%d".into(),
            time_format: "%H:%M".into(),
            start_of_week: Weekday::Mon,
            daily_target_hours: 8.0,
        }
    }
}

pub struct TimeKeeperApp {
    state: GuiState,
    settings: Settings,
    stopwatch: StopWatch,
}

impl Default for TimeKeeperApp {
    fn default() -> Self {
        Self {
            state: GuiState::default(),
            settings: Settings::default(),
            stopwatch: StopWatch::default(),
        }
    }
}

impl epi::App for TimeKeeperApp {
    fn name(&self) -> &str {
        APP_NAME
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        if let Some(storage) = _storage {
            self.state = epi::get_value(storage, STATE_KEY).unwrap_or_default();
            self.settings = epi::get_value(storage, SETTINGS_KEY).unwrap_or_default();
        }

        //start up bg thread
        let frame = frame.clone();
        thread::spawn(|| bg_timer(frame));
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, STATE_KEY, &self.state);
        epi::set_value(storage, SETTINGS_KEY, &self.settings);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            self.state.draw_tabs(ui);
        });

        egui::TopBottomPanel::bottom("stopwatch").show(ctx, |ui| {
            draw_stopwatch(&mut self.stopwatch, &self.settings, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.state
                .draw_screen(&mut self.stopwatch, &mut self.settings, ui)
        });
    }
}

/// thread to update the gui regularly.
/// This could be improved to only do it while the timer is active and the window is visible
fn bg_timer(frame: epi::Frame) {
    let one_second = Duration::seconds(1)
        .to_std()
        .expect("1 second should be in range");
    loop {
        thread::sleep(one_second);
        frame.request_repaint();
    }
}
