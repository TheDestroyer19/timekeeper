use chrono::{Duration, Local, DateTime};
use eframe::egui::{self, DragValue, RichText};
use egui_datepicker::DatePicker;

use crate::database::Tag;
use crate::history::{DayBlock, GoalState, History};
use crate::{
    app::Settings,
    stopwatch::StopWatch,
    database::Block,
};

#[derive(serde::Deserialize, serde::Serialize, Eq)]
pub enum GuiState {
    Today,
    ThisWeek,
    History(DateTime<Local>),
    Settings,
}
impl PartialEq for GuiState {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl Default for GuiState {
    fn default() -> Self {
        GuiState::Today
    }
}

impl GuiState {
    pub fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(self, GuiState::Today, "Today");
            ui.selectable_value(self, GuiState::ThisWeek, "This Week");
            ui.selectable_value(self, GuiState::History(Local::now()), "History");
            ui.selectable_value(self, GuiState::Settings, "Settings");
        });
    }

    pub fn draw_screen(
        &mut self,
        stopwatch: &mut StopWatch,
        history: &mut History,
        settings: &mut Settings,
        ui: &mut egui::Ui,
    ) {
        let message = match self {
            GuiState::Today => draw_today(stopwatch, history, settings, ui),
            GuiState::ThisWeek => draw_this_week(stopwatch, settings, history, ui),
            GuiState::History(datetime) => draw_history(*datetime, stopwatch, history, settings, ui),
            GuiState::Settings => draw_settings(settings, ui),
        };

        match message {
            GuiMessage::None => (),
            GuiMessage::ChangedBlockTag(block) => stopwatch.update_tag(block),
            GuiMessage::DeletedBlock(block) => history.delete_block(block),
            GuiMessage::SetState(state) => *self = state,
        }
    }
}

#[must_use]
enum GuiMessage {
    None,
    ChangedBlockTag(Block),
    DeletedBlock(Block),
    SetState(GuiState),
}

pub fn draw_goals(
    stopwatch: &mut StopWatch, 
    history: &mut History,
    settings: &Settings, 
    ui: &mut egui::Ui) {
    let daily = history.remaining_daily_goal(settings);
    let weekly = history.remaining_weekly_goal(settings);
    let running = stopwatch.current().is_some();
    
    draw_goal("Daily goal", running, daily, settings.daily_goal, settings, ui);
    draw_goal("Weekly goal", running, weekly, settings.weekly_goal, settings, ui);
}

pub fn draw_goal(label: &str, running: bool, state: GoalState, goal: Duration, settings: &Settings, ui: &mut egui::Ui) {
    match state {
        GoalState::ZeroGoal => (),
        GoalState::StillNeeds(remaining) => {
            let fraction = 1.0 - remaining.num_seconds() as f32 / goal.num_seconds() as f32;
            let progress = egui::ProgressBar::new(fraction);

            if remaining.num_hours() < 10 && running {
                let end_time = Local::now() + remaining;
                ui.add(progress.text(format!("{} finishes at {}", label, end_time.format(&settings.time_format))));
            } else {
                ui.add(progress.text(format!("{} left on {}", fmt_duration(remaining), label)));
            }
        },
        GoalState::Reached => {
            ui.add(egui::ProgressBar::new(1.0).text(format!("Huzzah, {} acheived!", label)));
        },
    }
}

pub fn draw_stopwatch(
    stopwatch: &mut StopWatch, 
    history: &mut History,
    settings: &Settings, 
    ui: &mut egui::Ui
) {
    ui.with_layout(
        egui::Layout::top_down_justified(egui::Align::Center),
        |ui| {
            let current = stopwatch.current();
            
            draw_goals(stopwatch, history, settings, ui);

            if let Some(current) = current {
                let text = format!(
                    "{}\tStop", fmt_duration(current.duration())
                );
                let button = egui::Button::new(RichText::new(text).heading())
                    .fill(egui::color::Color32::DARK_GREEN);
                if ui.add(button).clicked() {
                    stopwatch.stop();
                }
            } else if ui.button(RichText::new("Start").heading()).clicked() {
                stopwatch.start();
            }
        },
    );
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
    stopwatch: &mut StopWatch, 
    history: &mut History,
    settings: &Settings, 
    ui: &mut egui::Ui
) -> GuiMessage {
    let now = Local::now();

    let (total, blocks) = history.blocks_in_day(now);

    ui.horizontal(|ui| {
        ui.label(RichText::new(now.format(&settings.date_format).to_string()).heading());
        ui.label(RichText::new(fmt_duration(total)).heading());
    });

    draw_block_table(blocks, &stopwatch.all_tags(), settings, ui)
}

fn draw_block_table(blocks: Vec<Block>, tags: &[Tag], settings: &Settings, ui: &mut egui::Ui) -> GuiMessage {
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
                    egui::ComboBox::from_id_source(block.id())
                    .selected_text(tag_text)
                    .show_ui(ui, |ui| {
                        for tag in tags {
                            ui.selectable_value(&mut block.tag, Some(tag.clone()), &tag.name);
                        }
                        ui.separator();
                        ui.selectable_value(&mut block.tag, None, "Remove tag");
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

fn draw_this_week(stopwatch: &mut StopWatch, settings: &Settings,
    history: &mut History, ui: &mut egui::Ui) -> GuiMessage {
    let today = Local::now();
    draw_week(today, settings, stopwatch, history,  ui)
}

fn draw_week(
    day: chrono::DateTime<Local>, 
    settings: &Settings, 
    stopwatch: &mut StopWatch, 
    history: &mut History,
    ui: &mut egui::Ui
) -> GuiMessage {
    let mut message = GuiMessage::None;
    let tags = &stopwatch.all_tags();

    let (total, blocks) = history.blocks_in_week(day, settings);

    for DayBlock { day, blocks, total } in blocks {
        if total.is_zero() { continue; }
        let header = format!(
            "{} - {}",
            day.format(&settings.date_format).to_string(),
            fmt_duration(total)
        );
        egui::CollapsingHeader::new(RichText::new(header).heading())
            .id_source(day)
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

#[allow(deprecated)]
fn draw_history(date: DateTime<Local>, stopwatch: &mut StopWatch, history: &mut History, settings: &Settings, ui: &mut egui::Ui) -> GuiMessage {
    let mut message = GuiMessage::None;
    let start_of_week = History::start_of_week(date, settings);

    ui.horizontal(|ui| {
        if ui.button("<<<").clicked() {
            message = GuiMessage::SetState(GuiState::History(start_of_week - Duration::days(7)))
        }
        ui.add(DatePicker::new("history-datepicker", &mut start_of_week.date())
            .date_format(&settings.week_format)
            .highlight_weekend(true)
            .movable(true)
            .sunday_first(settings.start_of_week == chrono::Weekday::Sun));
        if start_of_week != date {
            message = GuiMessage::SetState(GuiState::History(start_of_week));
        }
        if ui.button(">>>").clicked() {
            message = GuiMessage::SetState(GuiState::History(start_of_week + Duration::days(7)))
        }
    });

    ui.separator();

    let message2 = draw_week(start_of_week, settings, stopwatch, history, ui);

    match message2 {
        GuiMessage::None => message,
        msg => msg,
    }
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

            ui.label("Week Format:");
            ui.text_edit_singleline(&mut settings.week_format);
            ui.end_row();
            ui.label("");
            ui.label(now.format(&settings.week_format).to_string());
            ui.end_row();

            ui.label("Time Format:");
            ui.text_edit_singleline(&mut settings.time_format);
            ui.end_row();
            ui.label("");
            ui.label(now.format(&settings.time_format).to_string());
            ui.end_row();

            ui.label("");
            ui.hyperlink_to("Formatter reference", "https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html");
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

                ui.add(DragValue::new(&mut hours)
                    .clamp_range(0.0..=24.0)
                    .speed(0.2)
                    .fixed_decimals(0)
                    .suffix(" hours"));
                ui.add(DragValue::new(&mut minutes)
                    .clamp_range(0.0..=60.0)
                    .speed(0.2)
                    .fixed_decimals(0)
                    .suffix(" minutes"));

                settings.daily_goal = Duration::minutes(hours * 60 + minutes);
            });
            ui.end_row();

            ui.label("Weekly Target:");
            ui.horizontal(|ui| {
                let mut hours = settings.weekly_goal.num_hours();
                let mut minutes = settings.weekly_goal.num_minutes() % 60;

                ui.add(DragValue::new(&mut hours)
                    .clamp_range(0.0..=168.0)
                    .speed(0.2)
                    .fixed_decimals(0)
                    .suffix(" hours"));
                ui.add(DragValue::new(&mut minutes)
                    .clamp_range(0.0..=60.0)
                    .speed(0.2)
                    .fixed_decimals(0)
                    .suffix(" minutes"));

                settings.weekly_goal = Duration::minutes(hours * 60 + minutes);
            })
        });

    GuiMessage::None
}
