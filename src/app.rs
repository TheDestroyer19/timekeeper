use std::thread;

use chrono::{DateTime, Local, Duration};
use eframe::{egui, epi};

/// A block of time
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct Block {
    //pub tag: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

/// A block of time that is still being tracked
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct PartialBlock {
    //pub tag: String,
    pub start: DateTime<Local>,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TimeKeeperApp {
    time_format: String,
    blocks: Vec<Block>,
    current: Option<PartialBlock>,

    //app management stuff
}

impl Default for TimeKeeperApp {
    fn default() -> Self {
        Self {
            time_format: "%m-%d %H:%M".into(),
            blocks: Vec::new(),
            current: None,
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
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }

        //start up bg thread
        let frame = frame.clone();
        thread::spawn(|| bg_timer(frame));
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::TopBottomPanel::bottom("current").show(ctx, |ui| {
            if let Some(block) = &mut self.current {
                let duration = Local::now() - block.start;

                ui.label(format!("{} - now ({})", block.start.format(&self.time_format), fmt_duration(duration)));

                if ui.button("Stop").clicked() {
                    let PartialBlock { start } = self.current.take().unwrap();
                    let block = Block {
                        start,
                        end: Local::now(),
                    };
                    self.blocks.push(block);
                }
            } else {
                if ui.button("Start").clicked() {
                    self.current = Some(PartialBlock {
                        start: Local::now(),
                    })
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("the-grid").num_columns(3).show(ui, |ui| {
                ui.label("Start");
                ui.label("End");
                ui.label("Duration");
                ui.end_row();

                for block in self.blocks.iter() {
                    ui.label(format!("{}", block.start.format(&self.time_format)));
                    ui.label(format!("{}", block.end.format(&self.time_format)));
                    ui.label(format!("{}", fmt_duration(block.end - block.start)));
                    ui.end_row();
                }
            });
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
    let one_second = Duration::seconds(1).to_std().expect("1 second should be in range");
    loop {
        thread::sleep(one_second);
        frame.request_repaint();
    }
}