use std::thread;

use chrono::{DateTime, Duration, Local};
use eframe::egui::RichText;
use eframe::{egui, epi};

/// A block of time
#[derive(serde::Deserialize, serde::Serialize)]
struct Block {
    //pub tag: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

/// A block of time that is still being tracked
#[derive(serde::Deserialize, serde::Serialize)]
struct PartialBlock {
    //pub tag: String,
    pub start: DateTime<Local>,
}

#[derive(PartialEq, Eq)]
enum AppScreen {
    Time,
    Settings,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TimeKeeperApp {
    date_format: String,
    time_format: String,
    datetime_format: String,
    blocks: Vec<Block>,
    current: Option<PartialBlock>,

    //app management stuff
    #[serde(skip)]
    screen: AppScreen,
}

impl Default for TimeKeeperApp {
    fn default() -> Self {
        Self {
            date_format: "%y-%m-%d".into(),
            time_format: "%H:%M".into(),
            datetime_format: "%m-%d %H:%M".into(),
            blocks: Vec::new(),
            current: None,
            screen: AppScreen::Time,
        }
    }
}

impl epi::App for TimeKeeperApp {
    fn name(&self) -> &str {
        "TimeKeeper"
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
            AppScreen::Time => self.draw_times(ui),
            AppScreen::Settings => self.draw_settings(ui),
        });
    }
}

impl TimeKeeperApp {
    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.screen, AppScreen::Time, "Time");
            ui.selectable_value(&mut self.screen, AppScreen::Settings, "Settings");
        });
    }

    fn draw_stopwatch(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                
                if let Some(block) = &mut self.current {
                    let duration = Local::now() - block.start;

                    ui.label(format!(
                        "{} - now ({})",
                        block.start.format(&self.time_format),
                        fmt_duration(duration)
                    ));

                    if ui.button(RichText::new("Stop").size(20.0)).clicked() {
                        let PartialBlock { start } = self.current.take().unwrap();
                        let block = Block {
                            start,
                            end: Local::now(),
                        };
                        self.blocks.push(block);
                    }
                } else if ui.button(RichText::new("Start").size(20.0)).clicked() {
                    self.current = Some(PartialBlock {
                        start: Local::now(),
                    })
                }
            },
        );
    }

    fn draw_times(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("the-grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Start");
                ui.label("End");
                ui.label("Duration");
                ui.end_row();

                let mut to_delete = None;

                for (index, block) in self.blocks.iter().enumerate() {
                    ui.label(block.start.format(&self.time_format).to_string());
                    ui.label(block.end.format(&self.time_format).to_string());
                    ui.label(fmt_duration(block.end - block.start));

                    if ui.button("X").clicked() {
                        to_delete = Some(index);
                    }

                    ui.end_row();
                }

                if let Some(index) = to_delete {
                    self.blocks.remove(index);
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

                ui.label("Date/Time Format:");
                ui.text_edit_singleline(&mut self.datetime_format);
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
