use chrono::{Duration, Local, DateTime, Date};
use eframe::egui::{self, DragValue, RichText};

use crate::database::Tag;
use crate::stopwatch::DayBlock;
use crate::{
    app::Settings,
    stopwatch::StopWatch,
    database::Block,
};

#[derive(serde::Deserialize, serde::Serialize, Eq)]
pub enum GuiState {
    Today,
    ThisWeek,
    AllTime(DateTime<Local>),
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
            ui.selectable_value(self, GuiState::AllTime(Local::now()), "History");
            ui.selectable_value(self, GuiState::Settings, "Settings");
        });
    }

    pub fn draw_screen(
        &mut self,
        stopwatch: &mut StopWatch,
        settings: &mut Settings,
        ui: &mut egui::Ui,
    ) {
        let message = match self {
            GuiState::Today => draw_today(stopwatch, settings, ui),
            GuiState::ThisWeek => draw_this_week(stopwatch, settings, ui),
            GuiState::AllTime(datetime) => draw_times(datetime.date(), stopwatch, settings, ui),
            GuiState::Settings => draw_settings(settings, ui),
        };

        match message {
            GuiMessage::None => (),
            GuiMessage::ChangedBlockTag(block) => stopwatch.update_tag(block),
            GuiMessage::DeletedBlock(block) => stopwatch.delete_block(block),
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

pub fn draw_todays_goal(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) {
    let goal = Some((settings.daily_target_hours * 60.0) as i64)
        .filter(|n| *n > 0)
        .map(Duration::minutes);
    let goal = match goal {
        Some(goal) => goal,
        None => return,
    };

    let current = stopwatch.current().is_some();
    let now = Local::now();
    let (time_today, _) = stopwatch.blocks_in_day(now.date());
    let time_left = goal - time_today;
    let projected = now + time_left;

    let text = match (current, time_left.cmp(&Duration::zero())) {
        (_, std::cmp::Ordering::Less) => format!(
            "You've reached your goal of {} today. Huzzah!",
            fmt_duration(goal)
        ), //TODO report how much more than the goal today
        (_, std::cmp::Ordering::Equal) => format!(
            "You've reached your goal of {} today. Huzzah!",
            fmt_duration(goal)
        ),
        (true, std::cmp::Ordering::Greater) => format!(
            "You will reach {} today at {}",
            fmt_duration(goal),
            projected.format(&settings.time_format)
        ),
        (false, std::cmp::Ordering::Greater) => format!(
            "If you start right now, you can reach {} today at {}",
            fmt_duration(goal),
            projected.format(&settings.time_format)
        ),
    };

    ui.label(text);
}

pub fn draw_stopwatch(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) {
    ui.with_layout(
        egui::Layout::top_down_justified(egui::Align::Center),
        |ui| {
            let current = stopwatch.current();
            if let Some(block) = &current {
                let duration = block.duration();

                ui.label(format!(
                    "{} - now ({})",
                    block.start.format(&settings.time_format),
                    fmt_duration(duration)
                ));
            };
            
            draw_todays_goal(stopwatch, settings, ui);

            if current.is_some() {
                if ui.button(RichText::new("Stop").size(20.0)).clicked() {
                    stopwatch.stop();
                }
            } else if ui.button(RichText::new("Start").size(20.0)).clicked() {
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

fn draw_today(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) -> GuiMessage {
    let now = Local::now();
    let today = now.date();

    let (total, blocks) = stopwatch.blocks_in_day(today);

    ui.horizontal(|ui| {
        ui.label(RichText::new(today.format(&settings.date_format).to_string()).heading());
        ui.label(RichText::new(fmt_duration(total)).heading());
    });

    let message = draw_block_table(blocks, &stopwatch.all_tags(), settings, ui);

    message
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
                    if block.start.date() != block.end.date() {
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

fn draw_this_week(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) -> GuiMessage {
    let today = Local::now();
    draw_week(today.date(), settings, stopwatch, ui)
}

fn draw_week(day: chrono::Date<Local>, settings: &Settings, stopwatch: &mut StopWatch, ui: &mut egui::Ui) -> GuiMessage {
    let mut message = GuiMessage::None;
    let tags = &stopwatch.all_tags();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let (total, blocks) = stopwatch.blocks_in_week(day, settings);


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
    });

    message
}

fn draw_times(date: Date<Local>, stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) -> GuiMessage {
    let mut message = GuiMessage::None;
    let date = StopWatch::start_of_week(date, settings);

    ui.horizontal(|ui| {
        if ui.button("<<<").clicked() {
            message = GuiMessage::SetState(GuiState::AllTime((date - Duration::days(7)).and_hms(11, 0, 0)))
        }
        ui.heading(format!("Week of {}", date.format(&settings.date_format)));
        if ui.button(">>>").clicked() {
            message = GuiMessage::SetState(GuiState::AllTime((date + Duration::days(7)).and_hms(11, 0, 0)))
        }
    });

    ui.separator();

    let message2 = draw_week(date, settings, stopwatch, ui);

    match message2 {
        GuiMessage::None => message,
        msg => msg,
    }
}

fn draw_settings(settings: &mut Settings, ui: &mut egui::Ui) -> GuiMessage {
    ui.heading("Date And Time");
    egui::Grid::new("settings-grid-formats")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Start of week:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Sun, "Sun");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Mon, "Mon");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Tue, "Tue");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Wed, "Wed");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Thu, "Thu");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Fri, "Fri");
                ui.selectable_value(&mut settings.start_of_week, chrono::Weekday::Sat, "Sat");
            });
            ui.end_row();

            ui.label("Date Format:");
            ui.text_edit_singleline(&mut settings.date_format);
            ui.end_row();

            ui.label("Time Format:");
            ui.text_edit_singleline(&mut settings.time_format);
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
                let mut hours = settings.daily_target_hours.floor();
                let mut minutes = (settings.daily_target_hours - hours) * 60.0;

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

                settings.daily_target_hours = hours + minutes / 60.0;
            });
            ui.end_row();
        });

    GuiMessage::None
}
