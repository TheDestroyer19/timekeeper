use chrono::{Date, Duration, Local, NaiveDate, TimeZone, Datelike};

use crate::app::Settings;
use crate::database::{Database, Block};

/// wrapper for details about one day
pub struct DayBlock {
    pub day: Date<Local>,
    pub blocks: Vec<Block>,
    pub total: Duration,
}
impl Default for DayBlock {
    fn default() -> Self {
        Self { day: Local::now().date(), blocks: Default::default(), total: Duration::zero() }
    }
}

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

    pub fn blocks_in_week(&mut self, day: Date<Local>, settings: &Settings) -> (Duration, [DayBlock; 7]) {
        let mut days = <[DayBlock; 7]>::default();
        let year = day.year();
        let week = day.iso_week();
        let mut weekday = settings.start_of_week.clone();
        let mut grand_total = Duration::zero();
        
        for dayblock in &mut days {
            let day = NaiveDate::from_isoywd(year, week.week(), weekday);
            let day = Local.from_local_date(&day).unwrap();
    
            let (total, blocks) = self.blocks_in_day(day);

            dayblock.blocks = blocks;
            grand_total = grand_total + total;
            dayblock.total = total;
            dayblock.day = day;
            weekday = weekday.succ();
        }

        (grand_total, days)
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