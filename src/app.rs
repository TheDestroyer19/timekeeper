use std::thread;

use chrono::Duration;
use eframe::egui;
use tracing::warn;

use crate::database::Database;
use crate::gui::{draw_stopwatch, GuiMessage, GuiState};
use crate::history::History;
use crate::settings::Settings;

const SETTINGS_KEY: &str = "Settings";
const STATE_KEY: &str = "State";

pub struct TimeKeeperApp {
    state: GuiState,
    settings: Settings,
    database: Database,
}

impl eframe::App for TimeKeeperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.database.stopwatch().update().unwrap();
        let current = self.database.blocks().current().unwrap();

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            self.state.draw_tabs(ui);
        });

        let message = egui::TopBottomPanel::bottom("stopwatch")
            .show(ctx, |ui| {
                draw_stopwatch(current, History::new(&self.database), &self.settings, ui)
            })
            .inner;
        self.handle_message(message);

        let message = egui::CentralPanel::default()
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        self.state
                            .draw_screen(&self.database, &mut self.settings, ui)
                    })
                    .inner
            })
            .inner
            .unwrap();
        self.handle_message(message);
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
        let settings: Settings;
        let state: GuiState;

        // load previous state if any
        if let Some(storage) = cc.storage {
            settings = Settings::deserailize(storage.get_string(SETTINGS_KEY));
            if let Some(value) = storage
                .get_string(STATE_KEY)
                .and_then(|s| serde_json::from_str(&s).ok())
            {
                state = value;
            } else {
                warn!("Failed to read gui state");
                state = GuiState::default();
            }
        } else {
            settings = Settings::default();
            state = GuiState::default();
        }

        //start update thread
        let ctx = cc.egui_ctx.clone();
        thread::spawn(|| bg_timer(ctx));

        Self {
            state,
            settings,
            database: Database::new().unwrap(),
        }
    }

    fn handle_message(&mut self, message: GuiMessage) {
        let result: anyhow::Result<()> = (|| {
            match message {
                GuiMessage::None => (),
                GuiMessage::ChangedBlockTag(block) => self.database.blocks().update_tag(block)?,
                GuiMessage::DeletedBlock(block) => History::new(&self.database).delete_block(block),
                GuiMessage::SetState(state) => self.state = state,
                GuiMessage::StartStopwatch(tag) => self.database.stopwatch().start(tag)?,
                GuiMessage::StopStopwatch => self.database.stopwatch().stop()?,
                GuiMessage::CreateTag(name) => self.database.tags().create(&name)?,
                GuiMessage::DeleteTag(tag) => self.database.tags().delete(tag)?,
                GuiMessage::RenameTag(tag) => self.database.tags().rename(tag)?,
            }
            Ok(())
        })();

        if let Err(e) = result {
            warn!("Error updating database: {e:#}");
        }
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
