use chrono::{Date, Duration, Local, NaiveDate, TimeZone, Datelike};

use crate::app::Settings;
use crate::database::{Database, Block};

pub enum GoalState {
    ZeroGoal,
    StillNeeds(Duration),
    Reached,
}

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

    pub fn blocks_in_day(&self, day: Date<Local>) -> (Duration, Vec<Block>) {
        let before = day.and_hms(0, 0, 0);
        let after = before + Duration::days(1);

        match self.database.blocks().in_range(before, after) {
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

    pub fn total_time(&self, day: Date<Local>) -> Duration {
        let before = day.and_hms(0, 0, 0);
        let after = before + Duration::days(1);

        match self.database.blocks().in_range(before, after) {
            Err(e) => {
                tracing::warn!("{:#}", e);
                Duration::zero()
            },
            Ok(blocks) => {
                blocks.iter()
                    .fold(Duration::zero(), |a, b| a + b.duration())
            }
        }
    }

    pub fn start_of_week(date: Date<Local>, settings: &Settings) -> Date<Local> {
        let year = date.year();
        let week = date.iso_week().week();
        let weekday = settings.start_of_week.clone();
        let day = NaiveDate::from_isoywd(year, week, weekday);
        Local.from_local_date(&day).unwrap()
    }

    pub fn blocks_in_week(&self, day: Date<Local>, settings: &Settings) -> (Duration, [DayBlock; 7]) {
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

    pub(crate) fn remaining_daily_goal(&self, settings: &Settings) -> GoalState {
        let goal  = settings.daily_goal;
        if goal <= Duration::zero() {
            return GoalState::ZeroGoal;
        }

        let time_today = self.total_time(Local::now().date());

        let remaining = goal - time_today;

        if remaining <= Duration::zero() {
            GoalState::Reached
        } else {
            GoalState::StillNeeds(remaining)
        }
    }

    pub(crate) fn remaining_weekly_goal(&self, settings: &Settings) -> GoalState {
        let goal  = settings.weekly_goal;
        if goal <= Duration::zero() {
            return GoalState::ZeroGoal;
        }

        let time_this_week = self.blocks_in_week(Local::now().date(), settings).0;

        let remaining = goal - time_this_week;

        if remaining <= Duration::zero() {
            GoalState::Reached
        } else {
            GoalState::StillNeeds(remaining)
        }
    }
}