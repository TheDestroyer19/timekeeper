use chrono::{Duration, Local, TimeZone};
use eframe::egui::{self, DragValue, RichText};

use crate::stopwatch::DayBlock;
use crate::{
    app::Settings,
    stopwatch::StopWatch,
    database::Block,
};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum GuiState {
    Today,
    ThisWeek,
    AllTime,
    Settings,
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
            ui.selectable_value(self, GuiState::AllTime, "History");
            ui.selectable_value(self, GuiState::Settings, "Settings");
        });
    }

    pub fn draw_screen(
        &self,
        stopwatch: &mut StopWatch,
        settings: &mut Settings,
        ui: &mut egui::Ui,
    ) {
        match self {
            GuiState::Today => draw_today(stopwatch, settings, ui),
            GuiState::ThisWeek => draw_this_week(stopwatch, settings, ui),
            GuiState::AllTime => draw_times(stopwatch, settings, ui),
            GuiState::Settings => draw_settings(settings, ui),
        }
    }
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

fn draw_today(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) {
    let now = Local::now();
    let today = now.date();

    let (total, blocks) = stopwatch.blocks_in_day(today);

    ui.horizontal(|ui| {
        ui.label(RichText::new(today.format(&settings.date_format).to_string()).heading());
        ui.label(RichText::new(fmt_duration(total)).heading());
    });

    draw_block_table(&blocks, settings, ui);

    ui.separator();
}

fn draw_block_table(blocks: &[Block], settings: &Settings, ui: &mut egui::Ui) {
    if blocks.is_empty() {
        return;
    }

    egui::Grid::new(blocks[0].id())
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            for block in blocks {
                ui.label(block.start.format(&settings.time_format).to_string());
                ui.label("->");
                if block.start.date() == block.end.date() {
                    ui.label(block.end.format(&settings.time_format).to_string());
                } else {
                    ui.horizontal(|ui| {
                        ui.label(block.end.format(&settings.date_format).to_string());
                        ui.label(block.end.format(&settings.time_format).to_string());
                    });
                }
                ui.label(fmt_duration(block.duration()));
                ui.end_row();
            }
        });
}

fn draw_this_week(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) {
    let today = Local::now();
    draw_week(today, settings, stopwatch, ui);
}

fn draw_week(day: chrono::DateTime<Local>, settings: &Settings, stopwatch: &mut StopWatch, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let (total, blocks) = stopwatch.blocks_in_week(day.date(), settings);

        for DayBlock { day, blocks, total } in blocks {
            let header = format!(
                "{} - {}",
                day.format(&settings.date_format).to_string(),
                fmt_duration(total)
            );
            egui::CollapsingHeader::new(RichText::new(header).heading())
                .id_source(day)
                .show(ui, |ui| {
                draw_block_table(&blocks, settings, ui);
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Grand Total:").heading());
            ui.label(RichText::new(fmt_duration(total)).heading());
        });
    });
}

fn draw_times(stopwatch: &mut StopWatch, settings: &Settings, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("the-grid")
            .num_columns(7)
            .striped(true)
            .show(ui, |ui| {
                let mut to_delete = None;
                let mut prev_date = Local.ymd(2000, 1, 1);

                for block in stopwatch.all_blocks() {
                    let date = block.start.date();
                    let end_date = block.end.date();
                    let duration = block.end - block.start;

                    if prev_date != date {
                        ui.label(date.format(&settings.date_format).to_string());
                        prev_date = date;
                    } else {
                        ui.label("");
                    }

                    ui.label(block.start.format(&settings.time_format).to_string());

                    ui.label("->");

                    if date != end_date {
                        ui.label(end_date.format(&settings.date_format).to_string());
                    } else {
                        ui.label("");
                    }

                    ui.label(block.end.format(&settings.time_format).to_string());

                    ui.label(fmt_duration(duration));

                    if ui.button("X").clicked() {
                        to_delete = Some(block);
                    }

                    ui.end_row();
                }

                if let Some(index) = to_delete {
                    stopwatch.delete_block(index);
                }
            });
        
        ui.horizontal(|ui| {
            ui.label(RichText::new("Total").heading());
            ui.label(fmt_duration(stopwatch.total_time()));
        })
    });
}

fn draw_settings(settings: &mut Settings, ui: &mut egui::Ui) {
    egui::Grid::new("settings-grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Date Format:");
            ui.text_edit_singleline(&mut settings.date_format);
            ui.end_row();

            ui.label("Time Format:");
            ui.text_edit_singleline(&mut settings.time_format);
            ui.end_row();

            ui.label("Daily Target Hours:");
            ui.add(DragValue::new(&mut settings.daily_target_hours).clamp_range(0.0..=24.0));
            ui.end_row();
        });
}
