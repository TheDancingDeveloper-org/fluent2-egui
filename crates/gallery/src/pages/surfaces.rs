//! Surfaces: dialogs, menus, popovers, tooltips, toasts, message bars, cards and tab lists
//! (WI-1930).

use fluent2_egui::controls::{Button, FileCard, Input, file_card_row};
use fluent2_egui::surfaces::{
    Card, CardAppearance, CardHeader, Dialog, DialogKind, DialogResult, Intent, MenuItem, MessageBar,
    Popover, TabAppearance, TabList, Toast, ToastPosition, Toaster, TooltipExt, context_menu,
};
use fluent2_icons::Icon;

use super::Page;

const INTENTS: [(&str, Intent); 4] = [
    ("Info", Intent::Info),
    ("Success", Intent::Success),
    ("Warning", Intent::Warning),
    ("Error", Intent::Error),
];

pub struct SurfacesPage {
    modal: bool,
    non_modal: bool,
    alert: bool,
    tall: bool,
    popover: bool,
    selected_card: bool,
    tab: usize,
    subtle_tab: usize,
    fields: Vec<String>,
    status: String,
    attachments: Vec<(&'static str, &'static str, u64)>,
}

impl Default for SurfacesPage {
    fn default() -> Self {
        Self {
            modal: false,
            non_modal: false,
            alert: false,
            tall: false,
            popover: false,
            selected_card: true,
            tab: 0,
            subtle_tab: 1,
            fields: vec![String::new(); 24],
            status: "Nothing yet".to_owned(),
            attachments: vec![
                ("Q3 report.pdf", "application/pdf", 1_250_000),
                ("Team photo.jpg", "image/jpeg", 3_400_000),
                ("Budget 2027.xlsx", "", 48_000),
                ("logs-2026-10-19.zip", "application/zip", 82_000_000),
            ],
        }
    }
}

impl Page for SurfacesPage {
    fn name(&self) -> &'static str {
        "Surfaces"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Surfaces");
        ui.label(format!("Last result: {}", self.status));
        ui.add_space(8.0);

        ui.strong("Dialogs");
        ui.horizontal_wrapped(|ui| {
            self.modal |= ui.add(Button::new("Modal dialog")).clicked();
            self.non_modal |= ui.add(Button::new("Non-modal dialog")).clicked();
            self.alert |= ui.add(Button::new("Alert dialog")).clicked();
            self.tall |= ui.add(Button::new("Tall dialog")).clicked();
        });
        self.dialogs(ui.ctx());

        ui.add_space(8.0);
        ui.strong("Menus, popovers and tooltips");
        ui.horizontal_wrapped(|ui| {
            let anchor = ui.add(Button::new("Popover").icon(Icon::Info));
            if anchor.clicked() {
                self.popover = !self.popover;
            }
            Popover::new("surfaces_popover")
                .with_arrow()
                .label("About popovers")
                .show(&anchor, &mut self.popover, |ui| {
                    ui.label("A popover holds rich, non-modal content.");
                });
            let target = ui
                .add(Button::new("Right-click me"))
                .fluent_tooltip("Opens a context menu on right-click");
            context_menu(&target, |ui| {
                if ui
                    .add(MenuItem::new("Copy").icon(Icon::Copy).shortcut("Ctrl+C"))
                    .clicked()
                {
                    self.status = "Copy".to_owned();
                }
                if ui
                    .add(
                        MenuItem::new("Paste")
                            .icon(Icon::ClipboardPaste)
                            .shortcut("Ctrl+V"),
                    )
                    .clicked()
                {
                    self.status = "Paste".to_owned();
                }
            });
            ui.add(Button::icon_only(Icon::QuestionCircle, "Help"))
                .fluent_tooltip("Help and support");
        });

        ui.add_space(8.0);
        ui.strong("Toasts");
        ui.horizontal_wrapped(|ui| {
            for (label, intent) in INTENTS {
                if ui.add(Button::new(format!("{label} toast"))).clicked() {
                    Toaster::push(
                        ui.ctx(),
                        Toast {
                            intent,
                            title: format!("{label} toast"),
                            body: Some("Toasts stack in the corner and time out.".to_owned()),
                            timeout: None,
                            action: Some("Undo".to_owned()),
                        },
                    );
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            // Presses coalesce into one toast while it is showing (push_keyed).
            if ui.add(Button::new("Pause 12 torrents")).clicked() {
                Toaster::push_keyed(
                    ui.ctx(),
                    "pause-torrents",
                    Toast {
                        intent: Intent::Info,
                        title: "Paused 12 torrents".to_owned(),
                        body: None,
                        timeout: None,
                        action: None,
                    },
                );
            }
            // Fifty pushes show the newest three and a "+47 more" row.
            if ui.add(Button::new("Push 50 toasts")).clicked() {
                for n in 1..=50 {
                    Toaster::push(
                        ui.ctx(),
                        Toast {
                            intent: Intent::Success,
                            title: format!("Item {n}"),
                            body: None,
                            timeout: None,
                            action: None,
                        },
                    );
                }
            }
        });
        let history = Toaster::history(ui.ctx());
        egui::CollapsingHeader::new(format!("Toast history ({})", history.len())).show(ui, |ui| {
            for toast in history.iter().rev() {
                ui.label(&toast.title);
            }
        });
        Toaster::show(ui.ctx(), ToastPosition::BottomEnd);

        ui.add_space(8.0);
        ui.strong("Attachments");
        let mut removed = None;
        file_card_row(ui, |ui| {
            for (index, (name, mime, size)) in self.attachments.iter().enumerate() {
                let mut card = FileCard::new(*name)
                    .size(*size)
                    .removable(true)
                    .downloadable(true);
                if !mime.is_empty() {
                    card = card.mime(*mime);
                }
                let out = card.show(ui);
                if out.remove {
                    removed = Some(index);
                }
                if out.open {
                    self.status = format!("Open {name}");
                }
                if out.download {
                    self.status = format!("Download {name}");
                }
            }
            FileCard::new("Uploading draft.docx")
                .progress(Some(0.62))
                .openable(false)
                .show(ui);
        });
        if let Some(index) = removed {
            let (name, _, _) = self.attachments.remove(index);
            self.status = format!("Removed {name}");
        }

        ui.add_space(8.0);
        ui.strong("Message bars");
        for (label, intent) in INTENTS {
            let article = if matches!(intent, Intent::Info | Intent::Error) {
                "An"
            } else {
                "A"
            };
            let _ = MessageBar::new(intent, format!("{article} {} message.", label.to_lowercase()))
                .title(label)
                .show(ui);
            ui.add_space(4.0);
        }

        ui.add_space(8.0);
        ui.strong("Cards");
        ui.horizontal_wrapped(|ui| {
            for (title, appearance) in [
                ("Filled card", CardAppearance::Filled),
                ("Outline card", CardAppearance::Outline),
                ("Subtle card", CardAppearance::Subtle),
            ] {
                ui.scope(|ui| {
                    ui.set_max_width(220.0);
                    Card::new().appearance(appearance).id_salt(title).show(ui, |ui| {
                        CardHeader::new(title)
                            .description("Card description")
                            .image(Icon::Document)
                            .show(ui);
                    });
                });
            }
            ui.scope(|ui| {
                ui.set_max_width(220.0);
                Card::new()
                    .id_salt("selectable card")
                    .label("Selectable card")
                    .selectable(&mut self.selected_card)
                    .show(ui, |ui| {
                        CardHeader::new("Selectable card")
                            .description("Click or press Space")
                            .image(Icon::Star)
                            .show(ui);
                    });
            });
        });

        ui.add_space(8.0);
        ui.strong("Tab lists");
        TabList::new("surfaces_tabs", &mut self.tab)
            .tab("Home")
            .tab("Insert")
            .tab("View")
            .show(ui);
        TabList::new("surfaces_subtle_tabs", &mut self.subtle_tab)
            .appearance(TabAppearance::Subtle)
            .tab("Day")
            .tab("Week")
            .tab("Month")
            .show(ui);
    }
}

impl SurfacesPage {
    fn dialogs(&mut self, ctx: &egui::Context) {
        let record = |status: &mut String, name: &str, result: DialogResult<()>| {
            if let DialogResult::Closed(reason) = result {
                *status = format!("{name} closed: {reason:?}");
            }
        };
        let was = self.modal;
        let result = Dialog::new("surfaces_modal")
            .title("Save changes?")
            .description("Your document has unsaved changes.")
            .actions(|actions| {
                actions.primary("save", "Save");
                actions.secondary("dont-save", "Don't save");
                actions.secondary("cancel", "Cancel");
            })
            .show(ctx, &mut self.modal, |ui| {
                ui.label("Your document has unsaved changes.");
            });
        if was {
            record(&mut self.status, "Modal", result);
        }

        let was = self.non_modal;
        let result = Dialog::new("surfaces_non_modal")
            .title("Find")
            .kind(DialogKind::Dismissible)
            .actions(|actions| actions.secondary("close", "Close"))
            .show(ctx, &mut self.non_modal, |ui| {
                ui.label(
                    "Escape closes this dialog; a backdrop click does not. The page behind stays blocked.",
                );
            });
        if was {
            record(&mut self.status, "Non-modal", result);
        }

        let was = self.alert;
        let result = Dialog::new("surfaces_alert")
            .title("Delete 3 files?")
            .kind(DialogKind::Alert)
            .actions(|actions| {
                actions.primary("delete", "Delete");
                actions.secondary("cancel", "Cancel");
            })
            .show(ctx, &mut self.alert, |ui| {
                ui.label("Only a button closes an alert dialog.");
            });
        if was {
            record(&mut self.status, "Alert", result);
        }

        let was = self.tall;
        let fields = &mut self.fields;
        let result = Dialog::new("surfaces_tall")
            .title("Connection settings")
            .actions(|actions| {
                actions.primary("ok", "OK");
                actions.secondary("cancel", "Cancel");
            })
            .show(ctx, &mut self.tall, |ui| {
                ui.label("The body scrolls when it is taller than the window.");
                for (index, field) in fields.iter_mut().enumerate() {
                    ui.add(
                        Input::new(field)
                            .label(format!("Setting {}", index + 1))
                            .placeholder(format!("Setting {}", index + 1)),
                    );
                }
            });
        if was {
            record(&mut self.status, "Tall", result);
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use fluent2_egui::Theme;

    use super::*;

    #[test]
    fn every_tab_stop_has_a_name() {
        let stops = crate::pages::controls::tests::tab_stops(SurfacesPage::default());
        assert!(
            stops.len() > 10,
            "the page has its controls in the Tab order: {stops:#?}"
        );
        for (role, label) in &stops {
            assert!(
                !label.trim().is_empty(),
                "{role:?} stop has no name; stops: {stops:#?}"
            );
        }
    }

    #[test]
    fn the_dialog_buttons_open_their_dialogs() {
        let mut page = SurfacesPage::default();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 1400.0))
            .build_ui(move |ui| {
                fluent2_egui::install(ui.ctx(), &Theme::default());
                page.ui(ui);
            });
        harness.run_steps(2);
        harness.get_by_label("Modal dialog").click();
        harness.run_steps(2);
        harness.run_steps(2);
        let _ = harness.get_by_role_and_label(Role::Dialog, "Save changes?");
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
        harness.run_steps(2);
        assert!(
            harness
                .query_by_role_and_label(Role::Dialog, "Save changes?")
                .is_none()
        );

        harness.get_by_label("Alert dialog").click();
        harness.run_steps(2);
        harness.run_steps(2);
        let _ = harness.get_by_role_and_label(Role::AlertDialog, "Delete 3 files?");
    }
}
