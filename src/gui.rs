use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, TimeZone};
use eframe::egui::{self, DragValue, RichText};
use eframe::epaint::Color32;
use egui_extras::DatePickerButton;
use tracing::info;

use crate::database::{Database, Tag};
// use crate::error::ReportAndContinue;
use crate::history::{DayBlock, GoalState, History};
use crate::{database::Block, settings::Settings};

#[must_use]
pub enum GuiMessage {
    None,
    ChangedBlockTag(Block),
    DeletedBlock(Block),
    SetState(GuiState),
    StartStopwatch(Option<Tag>),
    StopStopwatch,
    CreateTag(String),
    DeleteTag(Tag),
    RenameTag(Tag),
}

#[derive(serde::Deserialize, serde::Serialize, Eq, Default)]
pub enum GuiState {
    #[default]
    Today,
    ThisWeek,
    History(DateTime<Local>),
    Tags(String),
    Settings,
}
impl PartialEq for GuiState {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl GuiState {
    pub fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(self, GuiState::Today, "Today");
            ui.selectable_value(self, GuiState::ThisWeek, "This Week");
            ui.selectable_value(self, GuiState::History(Local::now()), "History");
            ui.selectable_value(self, GuiState::Tags("".to_string()), "Tags");
            ui.selectable_value(self, GuiState::Settings, "Settings");
        });
    }

    pub(crate) fn draw_screen(
        &mut self,
        database: &Database,
        settings: &mut Settings,
        ui: &mut egui::Ui,
    ) -> anyhow::Result<GuiMessage> {
        let mut history = History::new(database);
        let mut tags = database.tags().all()?;

        let message = match self {
            GuiState::Today => draw_today(database, settings, ui)?,
            GuiState::ThisWeek => draw_this_week(settings, &tags, &mut history, ui),
            GuiState::History(datetime) => {
                draw_history(*datetime, &tags, &mut history, settings, ui)
            }
            GuiState::Tags(new_name) => draw_tags(&mut tags, new_name, ui),
            GuiState::Settings => draw_settings(settings, ui),
        };

        Ok(message)
    }
}

pub(crate) fn draw_goals(
    is_running: bool,
    history: &mut History<'_>,
    settings: &Settings,
    ui: &mut egui::Ui,
) {
    let daily = history.remaining_daily_goal(settings);
    let weekly = history.remaining_weekly_goal(settings);

    draw_goal(
        "Daily goal",
        is_running,
        daily,
        settings.daily_goal,
        settings,
        ui,
    );
    draw_goal(
        "Weekly goal",
        is_running,
        weekly,
        settings.weekly_goal,
        settings,
        ui,
    );
}

pub(crate) fn draw_goal(
    label: &str,
    running: bool,
    state: GoalState,
    goal: Duration,
    settings: &Settings,
    ui: &mut egui::Ui,
) {
    match state {
        GoalState::ZeroGoal => (),
        GoalState::StillNeeds(remaining) => {
            let fraction = 1.0 - remaining.num_seconds() as f32 / goal.num_seconds() as f32;
            let progress = egui::ProgressBar::new(fraction);

            if remaining.num_hours() < 10 && running {
                let end_time = Local::now() + remaining;
                ui.add(progress.text(format!(
                    "{} finishes at {}",
                    label,
                    end_time.format(&settings.time_format)
                )));
            } else {
                ui.add(progress.text(format!("{} left on {}", fmt_duration(remaining), label)));
            }
        }
        GoalState::Reached => {
            ui.add(egui::ProgressBar::new(1.0).text(format!("Huzzah, {} acheived!", label)));
        }
    }
}

pub(crate) fn draw_stopwatch(
    current: Option<Block>,
    mut history: History<'_>,
    settings: &Settings,
    ui: &mut egui::Ui,
) -> GuiMessage {
    ui.with_layout(
        egui::Layout::top_down_justified(egui::Align::Center),
        |ui| {
            draw_goals(current.is_some(), &mut history, settings, ui);

            if let Some(current) = current {
                let text = format!("{}\tStop", fmt_duration(current.duration()));
                let button =
                    egui::Button::new(RichText::new(text).heading()).fill(Color32::DARK_GREEN);
                if ui.add(button).clicked() {
                    GuiMessage::StopStopwatch
                } else {
                    GuiMessage::None
                }
            } else if ui.button(RichText::new("Start").heading()).clicked() {
                GuiMessage::StartStopwatch(None)
            } else {
                GuiMessage::None
            }
        },
    )
    .inner
}

pub fn fmt_duration(mut duration: Duration) -> String {
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

fn draw_today(
    database: &Database,
    settings: &Settings,
    ui: &mut egui::Ui,
) -> anyhow::Result<GuiMessage> {
    let now = Local::now();
    let history = History::new(database);

    let (total, blocks) = history.blocks_in_day(now);

    ui.horizontal(|ui| {
        ui.label(RichText::new(now.format(&settings.date_format).to_string()).heading());
        ui.label(RichText::new(fmt_duration(total)).heading());
    });

    Ok(draw_block_table(
        blocks,
        &database.tags().all()?,
        settings,
        ui,
    ))
}

fn draw_block_table(
    blocks: Vec<Block>,
    tags: &[Tag],
    settings: &Settings,
    ui: &mut egui::Ui,
) -> GuiMessage {
    if blocks.is_empty() {
        return GuiMessage::None;
    }

    let mut message = GuiMessage::None;

    egui::Grid::new(blocks[0].id())
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            for mut block in blocks {
                ui.horizontal(|ui| {
                    ui.label(block.start.format(&settings.time_format).to_string());
                    ui.label("->");
                    if block.start.date_naive() != block.end.date_naive() {
                        //TODO don't add the date -- waiting on auto splitting blocks that cover multiple days
                        ui.label(block.end.format(&settings.date_format).to_string());
                    }
                    ui.label(block.end.format(&settings.time_format).to_string());
                });
                ui.label(fmt_duration(block.duration()));

                let old_tag = block.tag.clone();
                let mut to_delete = false;

                ui.horizontal(|ui| {
                    let tag_text = if let Some(tag) = &block.tag {
                        &tag.name
                    } else {
                        ""
                    };
                    egui::ComboBox::from_id_salt(block.id())
                        .selected_text(tag_text)
                        .show_ui(ui, |ui| {
                            for tag in tags {
                                ui.selectable_value(&mut block.tag, Some(tag.clone()), &tag.name);
                            }
                            if old_tag.is_some() {
                                ui.separator();
                                ui.selectable_value(&mut block.tag, None, "Remove tag");
                            }
                        });

                    to_delete = ui.button("X").clicked();
                });

                if to_delete {
                    message = GuiMessage::DeletedBlock(block);
                } else if old_tag != block.tag {
                    message = GuiMessage::ChangedBlockTag(block);
                }

                ui.end_row();
            }
        });

    message
}

fn draw_this_week(
    settings: &Settings,
    tags: &[Tag],
    history: &mut History<'_>,
    ui: &mut egui::Ui,
) -> GuiMessage {
    let today = Local::now();
    draw_week(today, tags, settings, history, ui)
}

fn draw_week(
    day: chrono::DateTime<Local>,
    tags: &[Tag],
    settings: &Settings,
    history: &mut History<'_>,
    ui: &mut egui::Ui,
) -> GuiMessage {
    let mut message = GuiMessage::None;

    let (total, blocks) = history.blocks_in_week(day, settings);

    for DayBlock { day, blocks, total } in blocks {
        if total.is_zero() {
            continue;
        }
        let header = format!(
            "{} - {}",
            day.format(&settings.date_format),
            fmt_duration(total)
        );
        egui::CollapsingHeader::new(RichText::new(header).heading())
            .id_salt(day.naive_local().date())
            .show(ui, |ui| {
                message = draw_block_table(blocks, tags, settings, ui);
            });
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Total:").heading());
        ui.label(RichText::new(fmt_duration(total)).heading());
    });

    message
}

fn draw_history(
    date: DateTime<Local>,
    tags: &[Tag],
    history: &mut History<'_>,
    settings: &Settings,
    ui: &mut egui::Ui,
) -> GuiMessage {
    let start_of_week = History::start_of_week(date, settings);

    let r = ui.horizontal(|ui| {
        if ui.button("<<<").clicked() {
            return GuiMessage::SetState(GuiState::History(start_of_week - Duration::days(7)));
        }
        let mut naive_date = start_of_week.date_naive();
        ui.add(DatePickerButton::new(&mut naive_date));
        // ui.add(DatePicker::new("history-datepicker", &mut start_of_week.date())
        //     .date_format(&settings.week_format)
        //     .highlight_weekend(true)
        //     .movable(true)
        //     .sunday_first(settings.start_of_week == chrono::Weekday::Sun));
        if let Some(time) = Local
            .from_local_datetime(&NaiveDateTime::new(
                naive_date,
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            ))
            .earliest()
        {
            if date.date_naive() != time.date_naive() {
                info!("Changed date using datepicker");
                return GuiMessage::SetState(GuiState::History(time));
            }
        }
        if ui.button(">>>").clicked() {
            return GuiMessage::SetState(GuiState::History(start_of_week + Duration::days(7)));
        }
        GuiMessage::None
    });

    match r.inner {
        GuiMessage::None => (),
        msg => return msg,
    }

    ui.separator();

    draw_week(start_of_week, tags, settings, history, ui)
}

fn draw_tags(tags: &mut [Tag], new_name: &mut String, ui: &mut egui::Ui) -> GuiMessage {
    let mut message = GuiMessage::None;
    egui::Grid::new("tags")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for tag in tags {
                ui.label(&tag.name);
                if ui.button("X").clicked() {
                    message = GuiMessage::DeleteTag(tag.clone())
                }
                ui.end_row();
            }
        });

    ui.separator();

    ui.horizontal(|ui| {
        ui.text_edit_singleline(new_name);
        if ui.button("Create").clicked() {
            message = GuiMessage::CreateTag(new_name.clone());
        }
    });

    message
}

fn draw_settings(settings: &mut Settings, ui: &mut egui::Ui) -> GuiMessage {
    let now = Local::now();
    ui.heading("Date And Time");
    egui::Grid::new("settings-grid-formats")
        .num_columns(3)
        .show(ui, |ui| {
            ui.label("Start of week:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Sun, "Sunday");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Mon, "Monday");
            });
            ui.end_row();

            ui.label("Date Format:");
            ui.text_edit_singleline(&mut settings.date_format);
            ui.end_row();
            ui.label("");
            ui.label(now.format(&settings.date_format).to_string());
            ui.end_row();

            ui.label("Time Format:");
            ui.text_edit_singleline(&mut settings.time_format);
            ui.end_row();
            ui.label("");
            ui.label(now.format(&settings.time_format).to_string());
            ui.end_row();

            ui.label("");
            ui.hyperlink_to(
                "Formatter reference",
                "https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html",
            );
            ui.end_row();
        });

    ui.separator();

    ui.heading("Goals");
    egui::Grid::new("settings-grid-datetime-logic")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Daily Target:");
            ui.horizontal(|ui| {
                let mut hours = settings.daily_goal.num_hours();
                let mut minutes = settings.daily_goal.num_minutes() % 60;

                ui.add(
                    DragValue::new(&mut hours)
                        .range(0.0..=24.0)
                        .speed(0.2)
                        .fixed_decimals(0)
                        .suffix(" hours"),
                );
                ui.add(
                    DragValue::new(&mut minutes)
                        .range(0.0..=60.0)
                        .speed(0.2)
                        .fixed_decimals(0)
                        .suffix(" minutes"),
                );

                settings.daily_goal = Duration::minutes(hours * 60 + minutes);
            });
            ui.end_row();

            ui.label("Weekly Target:");
            ui.horizontal(|ui| {
                let mut hours = settings.weekly_goal.num_hours();
                let mut minutes = settings.weekly_goal.num_minutes() % 60;

                ui.add(
                    DragValue::new(&mut hours)
                        .range(0.0..=168.0)
                        .speed(0.2)
                        .fixed_decimals(0)
                        .suffix(" hours"),
                );
                ui.add(
                    DragValue::new(&mut minutes)
                        .range(0.0..=60.0)
                        .speed(0.2)
                        .fixed_decimals(0)
                        .suffix(" minutes"),
                );

                settings.weekly_goal = Duration::minutes(hours * 60 + minutes);
            })
        });

    GuiMessage::None
}
