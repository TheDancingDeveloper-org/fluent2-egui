//! Calendar: a Fluent month grid, and the date types the date and time pickers share (WI-1943).
//!
//! Dates are a plain [`CalendarDate`] (proleptic Gregorian, no time zone) with their own
//! arithmetic, so the default build needs no date crate; the `chrono` feature adds conversions
//! to and from `chrono::NaiveDate` and `NaiveTime`. Nothing here reads the clock: the app passes
//! "today" in. Month and weekday names come from [`CalendarStrings`] (English by default).
//!
//! Keyboard: the grid is one Tab stop. Arrows move a day or a week, Page Up and Page Down a
//! month, Home and End to the start and end of the week, Enter or Space picks the focused day.
//! The grid is an AccessKit Grid; the focused day is its active descendant, and each day is a
//! GridCell named with the full date ("Monday, 20 October 2026").

use egui::{
    Align2, CornerRadius, Id, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType,
};
use fluent2_tokens::generated::global::{border_radius, spacing_vertical, stroke_width};

use crate::controls::{Appearance, Button, ButtonSize};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::IconName;
use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{Theme, c};

/// A calendar date: year, month (1–12) and day (1–31), always valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CalendarDate {
    /// The date, or `None` if it does not exist (30 February, month 13).
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        ((MIN_YEAR..=MAX_YEAR).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month))
        .then_some(Self { year, month, day })
    }

    /// The year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// Days since 1970-01-01 (negative before it).
    #[must_use]
    pub fn days_since_epoch(self) -> i64 {
        // Howard Hinnant's days_from_civil.
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01, clamped to [`MIN_YEAR`]–[`MAX_YEAR`].
    #[must_use]
    pub fn from_days_since_epoch(days: i64) -> Self {
        let first = Self {
            year: MIN_YEAR,
            month: 1,
            day: 1,
        }
        .days_since_epoch();
        let last = Self {
            year: MAX_YEAR,
            month: 12,
            day: 31,
        }
        .days_since_epoch();
        let days = days.clamp(first, last);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// The date `days` later (or earlier, when negative).
    #[must_use]
    pub fn add_days(self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch().saturating_add(days))
    }

    /// The same day `months` later, clamped to the end of a shorter month (31 Jan + 1 month is
    /// 28 or 29 Feb).
    #[must_use]
    pub fn add_months(self, months: i32) -> Self {
        // In i64, so no input overflows; the year is then clamped to the supported range.
        let total = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(months);
        let year = total
            .div_euclid(12)
            .clamp(i64::from(MIN_YEAR), i64::from(MAX_YEAR)) as i32;
        let month = (total.rem_euclid(12) + 1) as u8;
        let day = self.day.min(days_in_month(year, month));
        Self { year, month, day }
    }

    /// Day of the week, 0 = Sunday to 6 = Saturday.
    #[must_use]
    pub fn weekday(self) -> u8 {
        (self.days_since_epoch() + 4).rem_euclid(7) as u8
    }

    /// The first day of this date's month.
    #[must_use]
    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// `YYYY-MM-DD`.
    #[must_use]
    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Parse `YYYY-MM-DD` (also with `/` or `.`), or `D/M/YYYY` when `day_first`, else
    /// `M/D/YYYY`. Surrounding whitespace is ignored.
    #[must_use]
    pub fn parse(text: &str, day_first: bool) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split(['-', '/', '.']).collect();
        let [a, b, c] = parts.as_slice() else {
            return None;
        };
        let (a, b, c) = (a.trim(), b.trim(), c.trim());
        if a.len() == 4 {
            return Self::new(a.parse().ok()?, b.parse().ok()?, c.parse().ok()?);
        }
        let (day, month) = if day_first { (a, b) } else { (b, a) };
        Self::new(c.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
    }
}

/// The earliest year dates are clamped to (chrono's lower bound).
pub const MIN_YEAR: i32 = -262_143;
/// The latest year dates are clamped to (chrono's upper bound).
pub const MAX_YEAR: i32 = 262_142;

/// Days in `month` (1–12) of `year`.
#[must_use]
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

/// A time of day, minute precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    /// 00:00.
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0 };
    /// 23:59.
    pub const LAST_MINUTE: Self = Self { hour: 23, minute: 59 };

    /// The time, or `None` past 23:59.
    #[must_use]
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    /// The hour, 0 to 23.
    #[must_use]
    pub fn hour(self) -> u8 {
        self.hour
    }

    /// The minute, 0 to 59.
    #[must_use]
    pub fn minute(self) -> u8 {
        self.minute
    }

    /// Minutes since midnight.
    #[must_use]
    pub fn minutes(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }

    /// `HH:MM` (24-hour) or `h:MM AM` (12-hour).
    #[must_use]
    pub fn format(self, twelve_hour: bool) -> String {
        if twelve_hour {
            let suffix = if self.hour < 12 { "AM" } else { "PM" };
            let hour = match self.hour % 12 {
                0 => 12,
                h => h,
            };
            format!("{hour}:{:02} {suffix}", self.minute)
        } else {
            format!("{:02}:{:02}", self.hour, self.minute)
        }
    }

    /// Parse `14:30`, `1430`, `9`, `2:30 pm`, `2pm`, `12 am`. Case and spaces are ignored.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let lower = text.trim().to_ascii_lowercase().replace(' ', "");
        let (body, meridiem) =
            if let Some(body) = lower.strip_suffix("am").or_else(|| lower.strip_suffix('a')) {
                (body, Some(false))
            } else if let Some(body) = lower.strip_suffix("pm").or_else(|| lower.strip_suffix('p')) {
                (body, Some(true))
            } else {
                (lower.as_str(), None)
            };
        let (hour, minute): (u8, u8) = if let Some((h, m)) = body.split_once([':', '.']) {
            (h.parse().ok()?, m.parse().ok()?)
        } else if body.len() > 2 && body.chars().all(|ch| ch.is_ascii_digit()) {
            let (h, m) = body.split_at(body.len() - 2);
            (h.parse().ok()?, m.parse().ok()?)
        } else {
            (body.parse().ok()?, 0)
        };
        let hour = match meridiem {
            Some(pm) => {
                if !(1..=12).contains(&hour) {
                    return None;
                }
                hour % 12 + if pm { 12 } else { 0 }
            }
            None => hour,
        };
        Self::new(hour, minute)
    }
}

#[cfg(feature = "chrono")]
mod chrono_interop {
    use super::{CalendarDate, TimeOfDay};
    use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};

    impl From<CalendarDate> for NaiveDate {
        fn from(date: CalendarDate) -> Self {
            // Valid by construction; the fallback is unreachable within chrono's range.
            NaiveDate::from_ymd_opt(date.year, u32::from(date.month), u32::from(date.day)).unwrap_or_default()
        }
    }

    impl From<NaiveDate> for CalendarDate {
        fn from(date: NaiveDate) -> Self {
            Self {
                year: date.year(),
                month: date.month() as u8,
                day: date.day() as u8,
            }
        }
    }

    impl From<TimeOfDay> for NaiveTime {
        fn from(time: TimeOfDay) -> Self {
            NaiveTime::from_hms_opt(u32::from(time.hour), u32::from(time.minute), 0).unwrap_or_default()
        }
    }

    impl From<NaiveTime> for TimeOfDay {
        fn from(time: NaiveTime) -> Self {
            Self {
                hour: time.hour() as u8,
                minute: time.minute() as u8,
            }
        }
    }
}

/// Names the calendar shows and reads out. English by default.
#[derive(Clone, Debug)]
pub struct CalendarStrings {
    /// January to December.
    pub months: [&'static str; 12],
    /// Sunday to Saturday.
    pub weekdays: [&'static str; 7],
    /// Two-letter column headers, Sunday to Saturday.
    pub weekdays_short: [&'static str; 7],
    /// The previous-month button.
    pub previous_month: &'static str,
    /// The next-month button.
    pub next_month: &'static str,
    /// Appended to today's accessible name.
    pub today: &'static str,
}

impl Default for CalendarStrings {
    fn default() -> Self {
        Self {
            months: [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ],
            weekdays: [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ],
            weekdays_short: ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
            previous_month: "Previous month",
            next_month: "Next month",
            today: "today",
        }
    }
}

impl CalendarStrings {
    /// "Monday, 20 October 2026".
    #[must_use]
    pub fn long_date(&self, date: CalendarDate) -> String {
        format!(
            "{}, {} {} {}",
            self.weekdays[usize::from(date.weekday())],
            date.day,
            self.months[usize::from(date.month - 1)],
            date.year
        )
    }

    /// "October 2026".
    #[must_use]
    pub fn month_year(&self, date: CalendarDate) -> String {
        format!("{} {}", self.months[usize::from(date.month - 1)], date.year)
    }
}

/// Draws extra content (an event dot, a count) in a day cell, under the day number.
pub type DayContent<'a> = &'a dyn Fn(&Ui, Rect, CalendarDate);

/// What [`Calendar::show`] did this frame.
pub struct CalendarResponse {
    /// The grid.
    pub response: Response,
    /// The day picked this frame (click, Enter or Space).
    pub picked: Option<CalendarDate>,
    /// The month shown, after any navigation this frame (its first day).
    pub month: CalendarDate,
}

/// A month grid. See the module docs.
///
/// ```
/// # use fluent2_egui::controls::{Calendar, CalendarDate};
/// # fn demo(ui: &mut egui::Ui, selected: &mut Option<CalendarDate>) {
/// let today = CalendarDate::new(2026, 10, 20);
/// let out = Calendar::new("due", selected).today(today).show(ui);
/// if let Some(day) = out.picked {
///     let _ = day;
/// }
/// # }
/// ```
pub struct Calendar<'a> {
    id_salt: Id,
    selected: &'a mut Option<CalendarDate>,
    today: Option<CalendarDate>,
    first_weekday: u8,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    strings: CalendarStrings,
    content: Option<DayContent<'a>>,
    cell: f32,
}

#[derive(Clone, Copy)]
struct GridMemory {
    month: CalendarDate,
    focus: CalendarDate,
}

impl<'a> Calendar<'a> {
    /// A calendar bound to `selected`.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        selected: &'a mut Option<CalendarDate>,
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            selected,
            today: None,
            first_weekday: 0,
            min: None,
            max: None,
            strings: CalendarStrings::default(),
            content: None,
            cell: 32.0,
        }
    }

    /// Today's date, which is marked, and shown first when nothing is selected.
    #[must_use]
    pub fn today(mut self, today: Option<CalendarDate>) -> Self {
        self.today = today;
        self
    }

    /// The first column: 0 = Sunday (the default), 1 = Monday, … 6 = Saturday.
    #[must_use]
    pub fn first_weekday(mut self, weekday: u8) -> Self {
        self.first_weekday = weekday % 7;
        self
    }

    /// Days before `min` or after `max` cannot be picked.
    #[must_use]
    pub fn range(mut self, min: Option<CalendarDate>, max: Option<CalendarDate>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Month and weekday names.
    #[must_use]
    pub fn strings(mut self, strings: CalendarStrings) -> Self {
        self.strings = strings;
        self
    }

    /// Draw extra content in each day cell (for example a dot on days with events).
    #[must_use]
    pub fn day_content(mut self, content: DayContent<'a>) -> Self {
        self.content = Some(content);
        self
    }

    /// Side of a day cell, in points. The default is 32.
    #[must_use]
    pub fn cell_size(mut self, side: f32) -> Self {
        if side.is_finite() {
            self.cell = side.clamp(20.0, 64.0);
        }
        self
    }

    fn allowed(&self, date: CalendarDate) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    /// Show the calendar.
    pub fn show(self, ui: &mut Ui) -> CalendarResponse {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let color = &theme.color;
        let id = ui.make_persistent_id(self.id_salt);
        let memory_id = id.with("memory");
        let start = self
            .selected
            .or(self.today)
            .or_else(|| CalendarDate::new(2000, 1, 1))
            .unwrap_or(CalendarDate {
                year: 2000,
                month: 1,
                day: 1,
            });
        let mut memory = ui
            .ctx()
            .data(|data| data.get_temp::<GridMemory>(memory_id))
            .unwrap_or(GridMemory {
                month: start.first_of_month(),
                focus: start,
            });
        let mut picked = None;
        let cell = self.cell;
        let width = cell * 7.0;

        // Header: month and year, with previous and next buttons.
        ui.allocate_ui(Vec2::new(width, cell), |ui| {
            ui.horizontal(|ui| {
                ui.set_width(width);
                ui.label(
                    crate::foundation::text::rich(
                        self.strings.month_year(memory.month),
                        TypeRamp::Body1Strong,
                    )
                    .color(c(color.neutral_foreground_1)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let next = ui.add(
                        Button::icon_only(IconName::ChevronRight, self.strings.next_month)
                            .appearance(Appearance::Subtle)
                            .size(ButtonSize::Small)
                            .id_salt(id.with("next")),
                    );
                    let previous = ui.add(
                        Button::icon_only(IconName::ChevronLeft, self.strings.previous_month)
                            .appearance(Appearance::Subtle)
                            .size(ButtonSize::Small)
                            .id_salt(id.with("previous")),
                    );
                    if next.clicked() {
                        memory.month = memory.month.add_months(1);
                        memory.focus = memory.focus.add_months(1);
                    }
                    if previous.clicked() {
                        memory.month = memory.month.add_months(-1);
                        memory.focus = memory.focus.add_months(-1);
                    }
                });
            });
        });
        ui.add_space(spacing_vertical::XS);

        // Weekday headers.
        let (header, _) = ui.allocate_exact_size(Vec2::new(width, cell * 0.75), Sense::hover());
        for column in 0..7u8 {
            let weekday = (self.first_weekday + column) % 7;
            let center = egui::pos2(
                header.left() + cell * (f32::from(column) + 0.5),
                header.center().y,
            );
            ui.painter().text(
                center,
                Align2::CENTER_CENTER,
                self.strings.weekdays_short[usize::from(weekday)],
                safe_font(ui, TypeRamp::Caption1),
                c(color.neutral_foreground_3),
            );
        }

        // The grid: one Tab stop.
        let (grid, mut response) = ui.allocate_exact_size(Vec2::new(width, cell * 6.0), Sense::click());

        // Keyboard.
        if response.has_focus() {
            let input = ui.input(|i| {
                [
                    i.key_pressed(Key::ArrowLeft),
                    i.key_pressed(Key::ArrowRight),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::PageUp),
                    i.key_pressed(Key::PageDown),
                    i.key_pressed(Key::Home),
                    i.key_pressed(Key::End),
                    i.key_pressed(Key::Enter) || i.key_pressed(Key::Space),
                ]
            });
            let mut focus = memory.focus;
            let column = i64::from((focus.weekday() + 7 - self.first_weekday) % 7);
            if input[0] {
                focus = focus.add_days(-1);
            }
            if input[1] {
                focus = focus.add_days(1);
            }
            if input[2] {
                focus = focus.add_days(-7);
            }
            if input[3] {
                focus = focus.add_days(7);
            }
            if input[4] {
                focus = focus.add_months(-1);
            }
            if input[5] {
                focus = focus.add_months(1);
            }
            if input[6] {
                focus = focus.add_days(-column);
            }
            if input[7] {
                focus = focus.add_days(6 - column);
            }
            if self.allowed(focus) {
                memory.focus = focus;
            }
            if input[8] && self.allowed(memory.focus) {
                picked = Some(memory.focus);
            }
        }
        // Keep the focused day's month on screen.
        if memory.focus.first_of_month() != memory.month {
            memory.month = memory.focus.first_of_month();
        }
        let lead = (memory.month.weekday() + 7 - self.first_weekday) % 7;
        let first_cell = memory.month.add_days(-i64::from(lead));

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut active_node = None;
        // The day cells hang under the grid node: each is registered by a hover-only
        // interaction in a child Ui whose accessibility parent is the grid.
        let cells_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(grid)
                .accessibility_parent(response.id),
        );
        for index in 0..42 {
            let date = first_cell.add_days(index);
            let row = (index / 7) as f32;
            let col = (index % 7) as f32;
            let rect = Rect::from_min_size(grid.min + Vec2::new(col * cell, row * cell), Vec2::splat(cell))
                .shrink(1.0);
            let in_month = date.month == memory.month.month;
            let allowed = self.allowed(date);
            let selected = *self.selected == Some(date);
            let today = self.today == Some(date);
            let hovered = pointer.is_some_and(|p| rect.contains(p)) && response.hovered() && allowed;
            let radius = CornerRadius::same(border_radius::MEDIUM as u8);
            let (fill, ink) = if selected {
                (Some(color.brand_background), color.neutral_foreground_on_brand)
            } else if hovered {
                (
                    Some(color.subtle_background_hover),
                    color.neutral_foreground_1_hover,
                )
            } else if !allowed {
                (None, color.neutral_foreground_disabled)
            } else if in_month {
                (None, color.neutral_foreground_1)
            } else {
                (None, color.neutral_foreground_3)
            };
            if let Some(fill) = fill {
                ui.painter().rect_filled(rect, radius, c(fill));
            }
            if today && !selected {
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    Stroke::new(stroke_width::THICK, c(color.brand_stroke_1)),
                    StrokeKind::Inside,
                );
            }
            let ramp = if today {
                TypeRamp::Body1Strong
            } else {
                TypeRamp::Body1
            };
            ui.painter().text(
                egui::pos2(
                    rect.center().x,
                    rect.center().y - if self.content.is_some() { 3.0 } else { 0.0 },
                ),
                Align2::CENTER_CENTER,
                date.day.to_string(),
                safe_font(ui, ramp),
                c(ink),
            );
            if let Some(content) = self.content {
                content(ui, rect, date);
            }
            if response.has_focus() && date == memory.focus && focus_visible(ui, &response) {
                paint_focus_ring(ui, rect, radius);
            }
            if response.clicked() && pointer.is_some_and(|p| rect.contains(p)) && allowed {
                picked = Some(date);
                memory.focus = date;
            }
            // AccessKit cell.
            let cell_id = cells_ui
                .interact(rect, id.with(("day", index)), Sense::hover())
                .id;
            let mut name = self.strings.long_date(date);
            if today {
                name = format!("{name}, {}", self.strings.today);
            }
            let bounds = rect;
            ui.ctx().accesskit_node_builder(cell_id, |node| {
                node.set_role(egui::accesskit::Role::GridCell);
                node.set_label(name);
                node.set_selected(selected);
                node.set_row_index((index / 7) as usize);
                node.set_column_index((index % 7) as usize);
                node.set_bounds(egui::accesskit::Rect {
                    x0: f64::from(bounds.min.x),
                    y0: f64::from(bounds.min.y),
                    x1: f64::from(bounds.max.x),
                    y1: f64::from(bounds.max.y),
                });
                if !allowed {
                    node.set_disabled();
                }
            });
            if date == memory.focus {
                active_node = Some(cell_id);
            }
        }
        if let Some(day) = picked {
            *self.selected = Some(day);
            response.mark_changed();
        }
        let label = self.strings.month_year(memory.month);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Unknown, true, label.clone()));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Grid);
            node.set_row_count(6);
            node.set_column_count(7);
            if let Some(active) = active_node {
                node.set_active_descendant(active.accesskit_id());
            }
        });
        ui.ctx().data_mut(|data| data.insert_temp(memory_id, memory));
        CalendarResponse {
            response,
            picked,
            month: memory.month,
        }
    }
}

/// The ramp step's font, in the proportional family while the semibold family is not loaded
/// (painting with an unbound family panics in egui).
fn safe_font(ui: &Ui, step: TypeRamp) -> egui::FontId {
    let mut font = font_id(step);
    if let egui::FontFamily::Name(_) = &font.family {
        let bound = ui.fonts(|f| f.definitions().families.contains_key(&font.family));
        if !bound {
            font.family = egui::FontFamily::Proportional;
        }
    }
    font
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u8, day: u8) -> CalendarDate {
        CalendarDate::new(y, m, day).unwrap_or(CalendarDate {
            year: 0,
            month: 1,
            day: 1,
        })
    }

    #[test]
    fn epoch_round_trip_and_weekdays() {
        assert_eq!(d(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(d(1970, 1, 1).weekday(), 4, "Thursday");
        assert_eq!(d(2026, 10, 20).weekday(), 2, "Tuesday");
        assert_eq!(d(2000, 2, 29).weekday(), 2);
        for days in [-1_000_000, -1, 0, 59, 11_016, 20_746, 3_000_000] {
            assert_eq!(CalendarDate::from_days_since_epoch(days).days_since_epoch(), days);
        }
    }

    #[test]
    fn arithmetic_never_overflows() {
        // WI-1943 review: large month counts used to overflow i32 and panic.
        let date = d(2026, 10, 20);
        assert_eq!(date.add_months(i32::MAX).year(), MAX_YEAR);
        assert_eq!(date.add_months(i32::MIN).year(), MIN_YEAR);
        assert_eq!(date.add_days(i64::MAX).year(), MAX_YEAR);
        assert_eq!(date.add_days(i64::MIN).year(), MIN_YEAR);
        let last = d(MAX_YEAR, 12, 31);
        assert_eq!(last.add_days(1), last, "the last date clamps");
        assert_eq!(d(MIN_YEAR, 1, 1).add_months(-1).year(), MIN_YEAR);
        assert!(CalendarDate::new(MAX_YEAR + 1, 1, 1).is_none());
        assert_eq!(CalendarDate::from_days_since_epoch(last.days_since_epoch()), last);
    }

    #[test]
    fn month_arithmetic_clamps() {
        assert_eq!(d(2026, 1, 31).add_months(1), d(2026, 2, 28));
        assert_eq!(d(2024, 1, 31).add_months(1), d(2024, 2, 29));
        assert_eq!(d(2026, 12, 15).add_months(1), d(2027, 1, 15));
        assert_eq!(d(2026, 1, 15).add_months(-1), d(2025, 12, 15));
        assert_eq!(d(2026, 3, 1).add_days(-1), d(2026, 2, 28));
        assert!(CalendarDate::new(2026, 2, 29).is_none());
        assert!(CalendarDate::new(2026, 13, 1).is_none());
    }

    #[test]
    fn date_parsing() {
        assert_eq!(CalendarDate::parse("2026-10-20", false), Some(d(2026, 10, 20)));
        assert_eq!(CalendarDate::parse(" 20/10/2026 ", true), Some(d(2026, 10, 20)));
        assert_eq!(CalendarDate::parse("10/20/2026", false), Some(d(2026, 10, 20)));
        assert_eq!(CalendarDate::parse("2026-02-30", false), None);
        assert_eq!(CalendarDate::parse("yesterday", false), None);
        assert_eq!(d(2026, 1, 5).to_iso(), "2026-01-05");
    }

    #[test]
    fn time_parsing_and_format() {
        let t = |h, m| TimeOfDay::new(h, m);
        assert_eq!(TimeOfDay::parse("14:30"), t(14, 30));
        assert_eq!(TimeOfDay::parse("1430"), t(14, 30));
        assert_eq!(TimeOfDay::parse("9"), t(9, 0));
        assert_eq!(TimeOfDay::parse("2:30 pm"), t(14, 30));
        assert_eq!(TimeOfDay::parse("2PM"), t(14, 0));
        assert_eq!(TimeOfDay::parse("12 am"), t(0, 0));
        assert_eq!(TimeOfDay::parse("12pm"), t(12, 0));
        assert_eq!(TimeOfDay::parse("25:00"), None);
        assert_eq!(TimeOfDay::parse("13pm"), None);
        assert_eq!(t(0, 5).map(|t| t.format(true)).as_deref(), Some("12:05 AM"));
        assert_eq!(t(14, 30).map(|t| t.format(false)).as_deref(), Some("14:30"));
    }

    #[test]
    fn long_date() {
        assert_eq!(
            CalendarStrings::default().long_date(d(2026, 10, 20)),
            "Tuesday, 20 October 2026"
        );
    }
}
