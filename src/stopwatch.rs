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
        if let Err(e) = self.database.blocks().insert(|block| block.running = true) {
            tracing::warn!("{:#}", e);
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut block) = self.current() {
            block.running = false;
            match self.database.blocks().update_running(block) {
                Ok(()) => (),
                Err(e) => tracing::warn!("{:#}", e),
            }
        } else {
            tracing::warn!("Tried to stop the stopwatch when it wasn't running")
        }
    }

    pub fn current(&mut self) -> Option<Block> {
        match self.database.blocks().current() {
            Ok(Some(mut block)) => {
                block.end = Local::now();
                if let Err(e) = self.database.blocks().update_running(block.clone()) {
                    tracing::warn!("{:#}", e);
                };
                Some(block)
            },
            Ok(None) => None,
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