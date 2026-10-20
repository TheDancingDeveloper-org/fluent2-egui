//! Date and time: a Calendar with event dots, a DatePicker and a TimePicker (WI-1943).

use fluent2_egui::controls::{Calendar, CalendarDate, DatePicker, Divider, Field, TimeOfDay, TimePicker};

use super::Page;

/// The gallery's fixed "today", so screenshots do not change from day to day.
fn today() -> Option<CalendarDate> {
    CalendarDate::new(2026, 10, 20)
}

pub struct DateTimePage {
    selected: Option<CalendarDate>,
    due: Option<CalendarDate>,
    start: Option<TimeOfDay>,
    end: Option<TimeOfDay>,
}

impl Default for DateTimePage {
    fn default() -> Self {
        Self {
            selected: CalendarDate::new(2026, 10, 23),
            due: CalendarDate::new(2026, 11, 2),
            start: TimeOfDay::new(9, 30),
            end: None,
        }
    }
}

impl Page for DateTimePage {
    fn name(&self) -> &'static str {
        "Date and time"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Date and time");
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(224.0);
                ui.add(Divider::horizontal().text("Calendar"));
                let theme = fluent2_egui::Theme::get(ui.ctx());
                let dot = fluent2_egui::theme::c(theme.color.brand_foreground_1);
                // Event dots on a few days, drawn by the app.
                let events = move |ui: &egui::Ui, rect: egui::Rect, day: CalendarDate| {
                    if day.month() == 10 && [6, 13, 20, 23, 29].contains(&day.day()) {
                        ui.painter()
                            .circle_filled(rect.center_bottom() - egui::vec2(0.0, 5.0), 2.0, dot);
                    }
                };
                Calendar::new("gallery_calendar", &mut self.selected)
                    .today(today())
                    .first_weekday(1)
                    .day_content(&events)
                    .show(ui);
            });
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.set_width(260.0);
                ui.add(Divider::horizontal().text("Pickers"));
                Field::new("Due date")
                    .hint("Type a date or pick one")
                    .show(ui, |ui| {
                        DatePicker::new("gallery_due", &mut self.due)
                            .today(today())
                            .first_weekday(1)
                            .show(ui);
                    });
                Field::new("Starts").show(ui, |ui| {
                    TimePicker::new("gallery_start", &mut self.start)
                        .step_minutes(15)
                        .show(ui);
                });
                Field::new("Ends").show(ui, |ui| {
                    TimePicker::new("gallery_end", &mut self.end)
                        .twelve_hour(true)
                        .show(ui);
                });
            });
        });
    }
}
