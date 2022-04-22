use chrono::{DateTime, Local, Duration};
use eframe::{egui, epi};

/// A block of time
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct Block {
    //pub tag: String,
    pub start: DateTime<Local>,
    pub length: Duration,
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
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
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

                ui.label(format!("{} - now ({})", block.start.format(&self.time_format), duration));

                if ui.button("Stop").clicked() {
                    let PartialBlock { start } = self.current.take().unwrap();
                    let block = Block {
                        start,
                        length: duration,
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
                    ui.label(format!("{}", (block.start + block.length).format(&self.time_format)));
                    ui.label(format!("{}", block.length));
                    ui.end_row();
                }
            });
        });
    }
}