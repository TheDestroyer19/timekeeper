use chrono::{Duration, Weekday};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Duration")]
struct DurationDef {
    #[serde(getter = "Duration::num_seconds")]
    secs: i64,
}
impl From<DurationDef> for Duration {
    fn from(d: DurationDef) -> Self {
        Duration::seconds(d.secs)
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub(crate) struct Settings {
    pub date_format: String,
    pub time_format: String,

    pub start_of_week: Weekday,

    #[serde(with = "DurationDef")]
    pub daily_goal: Duration,
    #[serde(with = "DurationDef")]
    pub weekly_goal: Duration,
}

impl Settings {
    pub(crate) fn deserailize(serialized: Option<String>) -> Settings {
        let Some(serialized) = serialized else {
            return Self::default();
        };

        match serde_json::from_str(&serialized) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to read settings: {}", e);
                Self::default()
            }
        }
    }

    pub(crate) fn serialize(&self) -> String {
        match serde_json::to_string(&self) {
            Ok(serialized) => serialized,
            Err(e) => {
                error!("Failed to serialize settings. {}", e);
                "{}".to_string()
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            date_format: "%y-%m-%d".into(),
            time_format: "%H:%M".into(),
            start_of_week: Weekday::Mon,
            daily_goal: Duration::hours(8),
            weekly_goal: Duration::hours(40),
        }
    }
}
