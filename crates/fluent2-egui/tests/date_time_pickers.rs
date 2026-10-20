//! Calendar, DatePicker and TimePicker behaviour (WI-1943).

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Calendar, CalendarDate, DatePicker, TimeOfDay, TimePicker};
use fluent2_egui::{Theme, install};

fn date(y: i32, m: u8, d: u8) -> Option<CalendarDate> {
    CalendarDate::new(y, m, d)
}

fn calendar(selected: Option<CalendarDate>) -> Harness<'static, Option<CalendarDate>> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            |ui, selected: &mut Option<CalendarDate>| {
                install(ui.ctx(), &Theme::default());
                Calendar::new("cal", selected).today(date(2026, 10, 20)).show(ui);
            },
            selected,
        )
}

fn active_label(harness: &Harness<'_, Option<CalendarDate>>) -> String {
    let grid = harness.get_by_role(Role::Grid);
    let active = grid.accesskit_node().data().active_descendant();
    harness
        .get_all_by_role(Role::GridCell)
        .find(|cell| Some(cell.accesskit_node().locate().0) == active)
        .and_then(|cell| cell.accesskit_node().label())
        .unwrap_or_default()
}

#[test]
fn grid_and_cells_are_named() {
    let mut harness = calendar(None);
    harness.run_steps(2);
    let _ = harness.get_by_role_and_label(Role::Grid, "October 2026");
    let _ = harness.get_by_role_and_label(Role::GridCell, "Tuesday, 20 October 2026, today");
    assert_eq!(harness.get_all_by_role(Role::GridCell).count(), 42);
    assert_eq!(active_label(&harness), "Tuesday, 20 October 2026, today");
}

#[test]
fn keyboard_moves_the_focused_day_and_picks() {
    let mut harness = calendar(None);
    harness.run_steps(2);
    harness.get_by_role(Role::Grid).focus();
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(active_label(&harness), "Wednesday, 21 October 2026");
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    assert_eq!(active_label(&harness), "Wednesday, 28 October 2026");
    harness.key_press(Key::PageDown);
    harness.run_steps(2);
    let _ = harness.get_by_role_and_label(Role::Grid, "November 2026");
    assert_eq!(active_label(&harness), "Saturday, 28 November 2026");
    harness.key_press(Key::Home);
    harness.run_steps(2);
    assert_eq!(active_label(&harness), "Sunday, 22 November 2026");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(*harness.state(), date(2026, 11, 22));
}

#[test]
fn a_click_on_a_day_picks_it() {
    let mut harness = calendar(None);
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::GridCell, "Friday, 30 October 2026")
        .click();
    harness.run_steps(2);
    assert_eq!(*harness.state(), date(2026, 10, 30));
    let cell = harness.get_by_role_and_label(Role::GridCell, "Friday, 30 October 2026");
    assert!(cell.accesskit_node().is_selected().unwrap_or(false));
}

#[test]
fn day_content_is_drawn_for_every_cell() {
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let mut selected = None;
        let dot = |ui: &egui::Ui, rect: egui::Rect, day: CalendarDate| {
            seen.set(seen.get() + 1);
            if day.day().is_multiple_of(5) {
                ui.painter().circle_filled(
                    rect.center_bottom() - egui::vec2(0.0, 5.0),
                    2.0,
                    egui::Color32::RED,
                );
            }
        };
        Calendar::new("events", &mut selected)
            .today(date(2026, 10, 20))
            .day_content(&dot)
            .show(ui);
    });
    harness.run();
    assert!(calls.get() >= 42);
}

#[derive(Default)]
struct Picked {
    date: Option<CalendarDate>,
    invalid: bool,
    time: Option<TimeOfDay>,
}

fn pickers() -> Harness<'static, Picked> {
    Harness::builder()
        .with_size(egui::vec2(500.0, 500.0))
        .build_ui_state(
            |ui, state: &mut Picked| {
                install(ui.ctx(), &Theme::default());
                let out = DatePicker::new("due", &mut state.date)
                    .label("Due date")
                    .today(date(2026, 10, 20))
                    .show(ui);
                state.invalid = out.invalid;
                TimePicker::new("at", &mut state.time).step_minutes(15).show(ui);
            },
            Picked::default(),
        )
}

fn type_and_enter(harness: &mut Harness<'static, Picked>, role: Role, label: &str, text: &str) {
    harness.get_by_role_and_label(role, label).focus();
    harness.run_steps(2);
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
}

#[test]
fn date_picker_parses_typed_dates_and_flags_bad_ones() {
    let mut harness = pickers();
    harness.run_steps(2);
    type_and_enter(&mut harness, Role::TextInput, "Due date", "25/12/2026");
    assert_eq!(harness.state().date, date(2026, 12, 25));
    assert!(!harness.state().invalid);
    type_and_enter(&mut harness, Role::TextInput, "Due date", "nonsense");
    assert_eq!(
        harness.state().date,
        date(2026, 12, 25),
        "bad text leaves the date as it was"
    );
    assert!(harness.state().invalid);
}

#[test]
fn date_picker_opens_a_calendar_and_takes_a_pick() {
    let mut harness = pickers();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Choose due date")
        .click();
    harness.run_steps(3);
    let _ = harness.get_by_role_and_label(Role::Grid, "October 2026");
    harness
        .get_by_role_and_label(Role::GridCell, "Monday, 26 October 2026")
        .click();
    harness.run_steps(3);
    assert_eq!(harness.state().date, date(2026, 10, 26));
    assert!(
        harness.query_by_role(Role::Grid).is_none(),
        "a pick closes the calendar"
    );

    // Alt+Down from the field opens it again.
    harness.get_by_role_and_label(Role::TextInput, "Due date").focus();
    harness.run_steps(2);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(3);
    let _ = harness.get_by_role(Role::Grid);
}

#[test]
fn time_picker_parses_typed_times() {
    let mut harness = pickers();
    harness.run_steps(2);
    type_and_enter(&mut harness, Role::ComboBox, "HH:MM", "2:30 pm");
    assert_eq!(harness.state().time, TimeOfDay::new(14, 30));
}

#[test]
fn time_picker_lists_times_at_its_step() {
    let mut value = None;
    let picker = TimePicker::new("t", &mut value).step_minutes(15);
    let options = picker.options();
    assert_eq!(options.len(), 96);
    assert_eq!(options.get(1).copied(), TimeOfDay::new(0, 15));
}

#[test]
fn renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui.ctx(), &theme);
            let mut selected = date(2026, 10, 14);
            Calendar::new("c", &mut selected)
                .today(date(2026, 10, 20))
                .first_weekday(1)
                .range(date(2026, 10, 5), date(2026, 10, 25))
                .show(ui);
            let mut due = date(2026, 10, 1);
            DatePicker::new("d", &mut due).show(ui);
        });
        harness.run_steps(2);
    }
}

#[test]
fn time_picker_keeps_typed_text_until_it_is_committed() {
    // WI-1943 review: the field used to be reparsed and reformatted on every keystroke, so
    // "14:30" could not be typed.
    let mut harness = pickers();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::ComboBox, "HH:MM").focus();
    harness.run_steps(2);
    for ch in ["1", "4"] {
        harness.event(Event::Text(ch.to_owned()));
        harness.run_steps(1);
    }
    let field = harness.get_by_role_and_label(Role::ComboBox, "HH:MM");
    assert_eq!(
        field.accesskit_node().value().as_deref(),
        Some("14"),
        "no reformat while typing"
    );
    assert_eq!(harness.state().time, None, "nothing is committed while typing");
    for ch in [":", "3", "0"] {
        harness.event(Event::Text(ch.to_owned()));
        harness.run_steps(1);
    }
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().time, TimeOfDay::new(14, 30));

    // A 12-hour entry is kept as typed until Enter too.
    harness.get_by_role_and_label(Role::ComboBox, "HH:MM").focus();
    harness.run_steps(2);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run_steps(1);
    harness.event(Event::Text("9pm".to_owned()));
    harness.run_steps(1);
    assert_eq!(harness.state().time, TimeOfDay::new(14, 30));
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().time, TimeOfDay::new(21, 0));
}
