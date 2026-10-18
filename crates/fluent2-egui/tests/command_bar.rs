//! CommandBar: priority overflow into the More menu, and reporting of overflowed commands.

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::command_bar::{CommandBar, CommandItem};
use fluent2_egui::theme::{self, Theme};
use fluent2_icons::Icon;

const ITEMS: [CommandItem<'static>; 3] = [
    CommandItem {
        key: "open",
        icon: Icon::Open,
        label: "Open",
        priority: 90,
    },
    CommandItem {
        key: "save",
        icon: Icon::Save,
        label: "Save",
        priority: 50,
    },
    CommandItem {
        key: "print",
        icon: Icon::Print,
        label: "Print",
        priority: 10,
    },
];

fn bar_harness(width: f32, activated: Rc<Cell<Option<&'static str>>>) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(width, 200.0))
        .build_ui(move |ui| {
            theme::install(ui.ctx(), &Theme::default());
            let response = CommandBar::new("commands", &ITEMS).show(ui);
            if let Some(key) = response.activated {
                activated.set(Some(key));
            }
        })
}

#[test]
fn shrinking_the_width_moves_the_lowest_priority_into_more() {
    let activated = Rc::new(Cell::new(None));
    let mut harness = bar_harness(800.0, Rc::clone(&activated));
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "More").is_none());
    assert!(harness.query_by_role_and_label(Role::Button, "Print").is_some());

    harness.set_size(egui::vec2(240.0, 200.0));
    harness.run();
    harness.run();
    assert!(
        harness.query_by_role_and_label(Role::Button, "More").is_some(),
        "a narrow bar shows the More button"
    );
    assert!(
        harness.query_by_role_and_label(Role::Button, "Print").is_none(),
        "the lowest-priority item leaves the bar"
    );
    assert!(
        harness.query_by_role_and_label(Role::Button, "Save").is_none(),
        "the middle item leaves the bar too"
    );
    assert!(harness.query_by_role_and_label(Role::Button, "Open").is_some());
}

#[test]
fn clicking_an_overflowed_item_in_the_menu_reports_it() {
    let activated = Rc::new(Cell::new(None));
    let mut harness = bar_harness(240.0, Rc::clone(&activated));
    harness.run();
    harness.get_by_role_and_label(Role::Button, "More").click();
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "Print").click();
    harness.run();
    harness.run();
    assert_eq!(activated.get(), Some("print"));
}
