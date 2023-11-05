use chrono::Local;

use crate::database::{Database, Block};

pub struct StopWatch {
    database: Database,
}

impl Default for StopWatch {
    fn default() -> Self {
        Self { database: Database::new().unwrap() }
    }
}

impl StopWatch {
    pub fn start(&mut self) {
        match self.database.stopwatch().start(None) {
            Ok(()) => tracing::info!("Started stopwatch at {:?}", Local::now()),
            Err(e) => tracing::warn!("{:#}", e),
        }
    }

    pub fn stop(&mut self) {
        match self.database.stopwatch().stop() {
            Ok(()) => tracing::info!("Stopped stopwatch at {:?}", Local::now()),
            Err(e) => tracing::warn!("{:#}", e),
        }
    }

    pub fn current(&mut self) -> Option<Block> {
        if let Err(e) = self.database.stopwatch().update() {
            tracing::warn!("{:#}", e);
        }
        match self.database.blocks().current() {
            Ok(block) => block,
            Err(e) => {
                tracing::warn!("{:#}", e);
                None
            }
        }
    }

    pub(crate) fn all_tags(&self) -> Vec<crate::database::Tag> {
        match self.database.all_tags() {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!("{:#}", e);
                Vec::new()
            }
        }
    }

    pub(crate) fn update_tag(&self, block: Block) {
        match self.database.blocks().update_tag(block) {
            Ok(_) => (),
            Err(e) => tracing::warn!("{:#}", e),
        }
    }
}