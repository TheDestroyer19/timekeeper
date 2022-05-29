use chrono::{Date, Duration, Local};

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
            match self.database.blocks().update(block) {
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
                if let Err(e) = self.database.blocks().update(block.clone()) {
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

    pub fn delete_block(&mut self, block: Block) {
        if let Err(e) = self.database.blocks().delete(block) {
            tracing::warn!("{:#}", e);
        }
    }

    pub fn all_blocks(&self) -> Vec<Block> {
        match self.database.blocks().all() {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("{:#}", e);
                Vec::new()
            }
        }
    }

    pub fn blocks_in_day(&mut self, day: Date<Local>) -> (Duration, Vec<Block>) {
        let before = day.and_hms(0, 0, 0);
        let after = before + Duration::days(1);

        match self.database.blocks().all_in_range(before, after) {
            Err(e) => {
                tracing::warn!("{:#}", e);
                (Duration::zero(), Vec::new())
            },
            Ok(blocks) => {
                let total = blocks.iter()
                    .fold(Duration::zero(), |a, b| a + b.duration());
                (total, blocks)
            }
        }
    }

    pub fn total_time(&self) -> Duration {
        match self.database.blocks().total_time() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("{:#}", e);
                Duration::zero()
            }
        }
    }
}