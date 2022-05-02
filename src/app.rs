use std::thread;

use chrono::{Datelike, Duration, Local, TimeZone};
use eframe::egui::{DragValue, RichText};
use eframe::{egui, epi};

use crate::stopwatch::StopWatch;
use crate::APP_NAME;

#[derive(PartialEq, Eq)]
enum AppScreen {
    Today,
    Time,
    Settings,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TimeKeeperApp {
    date_format: String,
    time_format: String,

    daily_target_hours: f32,

    stopwatch: StopWatch,

    //app management stuff
    #[serde(skip)]
    screen: AppScreen,
}

impl Default for TimeKeeperApp {
    fn default() -> Self {
        Self {
            date_format: "%y-%m-%d".into(),
            time_format: "%H:%M".into(),
            daily_target_hours: 8.0,
            stopwatch: StopWatch::default(),
            screen: AppScreen::Time,
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
        _ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }

        //open database, and give to stopwatch
        self.stopwatch.init_database();

        //start up bg thread
        let frame = frame.clone();
        thread::spawn(|| bg_timer(frame));
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            self.draw_tabs(ui);
        });

        egui::TopBottomPanel::bottom("stopwatch").show(ctx, |ui| {
            self.draw_stopwatch(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.screen {
            AppScreen::Today => self.draw_today(ui),
            AppScreen::Time => self.draw_times(ui),
            AppScreen::Settings => self.draw_settings(ui),
        });
    }
}

impl TimeKeeperApp {
    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.screen, AppScreen::Today, "Today");
            ui.selectable_value(&mut self.screen, AppScreen::Time, "Time");
            ui.selectable_value(&mut self.screen, AppScreen::Settings, "Settings");
        });
    }

    fn draw_stopwatch(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                if let Some(block) = self.stopwatch.current() {
                    let duration = Local::now() - block.start;

                    ui.label(format!(
                        "{} - now ({})",
                        block.start.format(&self.time_format),
                        fmt_duration(duration)
                    ));

                    if ui.button(RichText::new("Stop").size(20.0)).clicked() {
                        self.stopwatch.stop();
                    }
                } else if ui.button(RichText::new("Start").size(20.0)).clicked() {
                    self.stopwatch.start();
                }
            },
        );
    }

    fn draw_today(&mut self, ui: &mut egui::Ui) {
        let now = Local::now();

        let today = now.date();

        let current = self.stopwatch.current();

        let (mut total, blocks) = self.stopwatch.blocks_in_day(today);

        if let Some(current) = &current {
            total = total + current.duration();
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new(today.format(&self.date_format).to_string()).heading());
            ui.label(RichText::new(fmt_duration(total)).heading());
        });
        egui::Grid::new(today.weekday())
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                for block in blocks {
                    ui.label(block.start.format(&self.time_format).to_string());
                    ui.label("->");
                    if block.start.date() == block.end.date() {
                        ui.label(block.end.format(&self.time_format).to_string());
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(block.end.format(&self.date_format).to_string());
                            ui.label(block.end.format(&self.time_format).to_string());
                        });
                    }
                    ui.label(fmt_duration(block.duration()));
                    ui.end_row();
                }
            });

        ui.separator();

        if self.daily_target_hours > 0.01 && current.is_some() {
            let goal = Duration::minutes((self.daily_target_hours * 60.0) as i64);
            let left = goal - total;
            let target = now + left;
            ui.label(format!(
                "You will reach {} today at {}",
                fmt_duration(goal),
                target.format(&self.time_format)
            ));
        }
    }

    fn draw_times(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("the-grid")
            .num_columns(7)
            .striped(true)
            .show(ui, |ui| {
                let mut to_delete = None;
                let mut prev_date = Local.ymd(2000, 1, 1);

                for block in self.stopwatch.all_blocks() {
                    let date = block.start.date();
                    let end_date = block.end.date();
                    let duration = block.end - block.start;

                    if prev_date != date {
                        ui.label(date.format(&self.date_format).to_string());
                        prev_date = date;
                    } else {
                        ui.label("");
                    }

                    ui.label(block.start.format(&self.time_format).to_string());

                    ui.label("->");

                    if date != end_date {
                        ui.label(end_date.format(&self.date_format).to_string());
                    } else {
                        ui.label("");
                    }

                    ui.label(block.end.format(&self.time_format).to_string());

                    ui.label(fmt_duration(duration));

                    if ui.button("X").clicked() {
                        to_delete = Some(block);
                    }

                    ui.end_row();
                }

                ui.label(RichText::new("Total").heading());
                ui.label("");
                ui.label("");
                ui.label("");
                ui.label("");
                ui.label(fmt_duration(self.stopwatch.total_time()));

                ui.end_row();

                if let Some(index) = to_delete {
                    self.stopwatch.delete_block(index);
                }
            });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("settings-grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Date Format:");
                ui.text_edit_singleline(&mut self.date_format);
                ui.end_row();

                ui.label("Time Format:");
                ui.text_edit_singleline(&mut self.time_format);
                ui.end_row();

                ui.label("Daily Target Hours:");
                ui.add(DragValue::new(&mut self.daily_target_hours).clamp_range(0.0..=24.0));
                ui.end_row();
            });
    }
}

fn fmt_duration(mut duration: Duration) -> String {
    //Assume negative durations are rounding errors, so move to zero
    duration = duration.max(Duration::zero());

    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m {}s", minutes, seconds)
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
