//! FileCard behaviour (WI-1945).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{FileCard, file_card_row};
use fluent2_egui::{Theme, install};

#[derive(Default)]
struct Events {
    open: Vec<&'static str>,
    download: Vec<&'static str>,
    remove: Vec<&'static str>,
}

fn harness() -> Harness<'static, Events> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui_state(
            |ui, events: &mut Events| {
                install(ui.ctx(), &Theme::default());
                file_card_row(ui, |ui| {
                    let out = FileCard::new("report.pdf")
                        .mime("application/pdf")
                        .size(1_250_000)
                        .removable(true)
                        .downloadable(true)
                        .show(ui);
                    if out.open {
                        events.open.push("report.pdf");
                    }
                    if out.download {
                        events.download.push("report.pdf");
                    }
                    if out.remove {
                        events.remove.push("report.pdf");
                    }
                    let out = FileCard::new("photo.jpg")
                        .progress(Some(0.4))
                        .removable(true)
                        .show(ui);
                    if out.remove {
                        events.remove.push("photo.jpg");
                    }
                    FileCard::new("upload.bin")
                        .progress(None)
                        .openable(false)
                        .show(ui);
                });
            },
            Events::default(),
        )
}

#[test]
fn cards_and_actions_are_named() {
    let mut harness = harness();
    harness.run_steps(2);
    let _ = harness.get_by_role_and_label(Role::Button, "report.pdf, 1.2 MB");
    let _ = harness.get_by_role_and_label(Role::Button, "photo.jpg, 40% transferred");
    let _ = harness.get_by_label("upload.bin, transferring");
    let _ = harness.get_by_role_and_label(Role::Button, "Remove report.pdf");
    let _ = harness.get_by_role_and_label(Role::Button, "Download report.pdf");
    let _ = harness.get_by_role_and_label(Role::Button, "Remove photo.jpg");
    assert!(harness.query_by_label("Download photo.jpg").is_none());
}

#[test]
fn each_action_is_reported_on_its_own() {
    let mut harness = harness();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Download report.pdf")
        .click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Remove photo.jpg")
        .click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "report.pdf, 1.2 MB")
        .click();
    harness.run_steps(2);
    let events = harness.state();
    assert_eq!(events.download, ["report.pdf"]);
    assert_eq!(events.remove, ["photo.jpg"]);
    assert_eq!(
        events.open,
        ["report.pdf"],
        "a click on an action does not also open the card"
    );
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
            file_card_row(ui, |ui| {
                FileCard::new("a very long file name that will not fit on the card at all.docx")
                    .size(12)
                    .width(160.0)
                    .show(ui);
                FileCard::new("archive.zip")
                    .size(u64::MAX)
                    .downloadable(true)
                    .show(ui);
            });
        });
        harness.run_steps(2);
    }
}
