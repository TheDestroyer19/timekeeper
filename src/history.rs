use chrono::{Local, Duration, Datelike, DateTime, Timelike, Days};

use crate::{
    database::{Database, Block}, 
    app::Settings
};

pub enum GoalState {
    ZeroGoal,
    StillNeeds(Duration),
    Reached,
}

/// wrapper for details about one day
pub struct DayBlock {
    pub day: DateTime<Local>,
    pub blocks: Vec<Block>,
    pub total: Duration,
}
impl Default for DayBlock {
    fn default() -> Self {
        Self { day: Local::now(), blocks: Default::default(), total: Duration::zero() }
    }
}

pub struct History {
    database: Database,
}

impl Default for History {
    fn default() -> Self {
        Self { database: Database::new().unwrap() }
    }
}

impl History {
    pub fn delete_block(&mut self, block: Block) {
        if let Err(e) = self.database.blocks().delete(block) {
            tracing::warn!("{:#}", e);
        }
    }

    pub fn blocks_in_day(&self, day: DateTime<Local>) -> (Duration, Vec<Block>) {
        let before = day - Duration::seconds(day.num_seconds_from_midnight() as i64);
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

    pub fn total_time(&self, day: DateTime<Local>) -> Duration {
        let before = day - Duration::seconds(day.num_seconds_from_midnight() as i64);
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

    pub fn start_of_week(date: DateTime<Local>, settings: &Settings) -> DateTime<Local> {
        let offset = match settings.start_of_week {
            chrono::Weekday::Mon => date.weekday().num_days_from_monday(),
            chrono::Weekday::Sun => date.weekday().num_days_from_sunday(),
            _ => panic!("Unsupported start of week"),
        };

        let date = date - Days::new(offset as u64);
        assert_eq!(date.weekday(), settings.start_of_week);
        date
    }

    pub fn blocks_in_week(&self, day: DateTime<Local>, settings: &Settings) -> (Duration, [DayBlock; 7]) {
        let mut days = <[DayBlock; 7]>::default();
        let mut day = History::start_of_week(day, settings);
        let mut grand_total = Duration::zero();
        
        for dayblock in &mut days {
            let (total, blocks) = self.blocks_in_day(day);

            dayblock.blocks = blocks;
            grand_total = grand_total + total;
            dayblock.total = total;
            dayblock.day = day;
            day =  day + Days::new(1);
        }

        (grand_total, days)
    }

    pub(crate) fn remaining_daily_goal(&self, settings: &Settings) -> GoalState {
        let goal  = settings.daily_goal;
        if goal <= Duration::zero() {
            return GoalState::ZeroGoal;
        }

        let time_today = self.total_time(Local::now());

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

        let time_this_week = self.blocks_in_week(Local::now(), settings).0;

        let remaining = goal - time_this_week;

        if remaining <= Duration::zero() {
            GoalState::Reached
        } else {
            GoalState::StillNeeds(remaining)
        }
    }
}