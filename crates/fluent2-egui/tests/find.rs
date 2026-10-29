//! FindBar: Ctrl+F opens it, Enter and Shift+Enter move through the matches, the counter, the
//! buttons, Escape, scrolling a virtualised List to the match and the highlight, through kittest
//! (no wgpu).

use egui::{Color32, Key, Modifiers, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::find::{FindBar, FindProvider, FindState};
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::theme::c;
use fluent2_egui::{Theme, install};

struct App {
    find: FindState,
    lines: Vec<String>,
    list: ListState,
    theme: fn() -> Theme,
}

fn lines() -> Vec<String> {
    (0..200)
        .map(|i| match i {
            3 | 150 | 180 => format!("line {i} needle"),
            _ => format!("line {i}"),
        })
        .collect()
}

fn harness(theme: fn() -> Theme) -> Harness<'static, App> {
    Harness::builder()
        .with_size(Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, app: &mut App| {
                install(ui.ctx(), &(app.theme)());
                let providers: [&dyn FindProvider; 1] = [&app.lines];
                FindBar::new("find")
                    .match_case_toggle(true)
                    .show(ui, &mut app.find, &providers);
                let mut list = List::new("lines", app.lines.len())
                    .label("Lines")
                    .row_label(|row| format!("line {row}"));
                if let Some(m) = app.find.take_reveal() {
                    list = list.scroll_to_row(m.item);
                }
                let find = &app.find;
                let lines = &app.lines;
                list.show(ui, &mut app.list, |cell, ui| {
                    let text = lines.get(cell.row).map_or("", String::as_str);
                    ui.label(find.layout_job(ui, 0, cell.row, text));
                });
            },
            App {
                find: FindState::default(),
                lines: lines(),
                list: ListState::default(),
                theme,
            },
        )
}

fn light() -> Theme {
    Theme::light(fluent2_tokens::FLUENT)
}

/// Open the bar with Ctrl+F and type `query`.
fn open_and_type(harness: &mut Harness<'_, App>, query: &str) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    harness.get_by_label("Find").type_text(query);
    harness.run();
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    let themes: [fn() -> Theme; 3] = [
        light,
        || Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast,
    ];
    for theme in themes {
        let mut harness = harness(theme);
        open_and_type(&mut harness, "needle");
        harness.get_by_label("1 of 3");
        harness.get_by_label("Next match");
        harness.get_by_label("Close find");
    }
}

#[test]
fn closed_until_ctrl_f_which_focuses_the_field() {
    let mut harness = harness(light);
    harness.run();
    assert!(harness.query_by_label("Find").is_none());

    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    assert!(harness.state().find.is_open());
    assert!(harness.get_by_label("Find").is_focused());
}

#[test]
fn enter_and_shift_enter_move_through_the_matches() {
    let mut harness = harness(light);
    open_and_type(&mut harness, "needle");
    harness.get_by_label("1 of 3");

    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("2 of 3");
    assert!(
        harness.get_by_label("Find").is_focused(),
        "Enter keeps focus in the field"
    );

    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    harness.get_by_label("3 of 3");
}

#[test]
fn buttons_move_and_close() {
    let mut harness = harness(light);
    open_and_type(&mut harness, "needle");

    harness.get_by_label("Next match").click();
    harness.run();
    harness.get_by_label("2 of 3");
    harness.get_by_label("Previous match").click();
    harness.run();
    harness.get_by_label("1 of 3");

    harness.get_by_label("Close find").click();
    harness.run();
    assert!(!harness.state().find.is_open());
    assert!(harness.query_by_label("Find").is_none());
}

#[test]
fn no_results_and_escape_closes() {
    let mut harness = harness(light);
    open_and_type(&mut harness, "absent");
    harness.get_by_label("No results");

    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().find.is_open());
}

#[test]
fn match_case_toggle_narrows_the_search() {
    let mut harness = harness(light);
    open_and_type(&mut harness, "Needle");
    harness.get_by_label("1 of 3");
    harness.get_by_label("Aa").click();
    harness.run();
    harness.get_by_label("No results");
}

#[test]
fn moving_to_a_match_scrolls_the_list_to_it() {
    let mut harness = harness(light);
    harness.run();
    assert!(
        harness.query_by_label("line 150 needle").is_none(),
        "row 150 starts out of view"
    );

    open_and_type(&mut harness, "needle");
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    harness.get_by_label("2 of 3");
    harness.get_by_label("line 150 needle");
}

#[test]
fn layout_job_highlights_matches_and_the_current_one() {
    let mut harness = harness(light);
    open_and_type(&mut harness, "needle");
    let theme = light();
    let state = &harness.state().find;
    let ctx = harness.ctx.clone();
    let job = state.layout_job_with(&ctx, 0, 3, "line 3 needle", egui::TextFormat::default());
    let backgrounds: Vec<Color32> = job.sections.iter().map(|s| s.format.background).collect();
    assert_eq!(
        backgrounds,
        vec![Color32::TRANSPARENT, c(theme.color.palette_marigold_background_3)],
        "plain text, then the current match"
    );
    let job = state.layout_job_with(&ctx, 0, 150, "line 150 needle", egui::TextFormat::default());
    assert_eq!(
        job.sections.get(1).map(|s| s.format.background),
        Some(c(theme.color.palette_yellow_background_2))
    );
}
