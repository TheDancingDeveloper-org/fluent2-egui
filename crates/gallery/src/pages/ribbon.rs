//! Ribbon page: a Home-like tab (Clipboard, Font and Paragraph groups) and an Insert-like tab,
//! with the events the ribbon reports listed under it (spec §4, WI-1563). Labels are generic.

use fluent2_egui::IconName;
use fluent2_egui::office::ribbon::{
    FileTab, GroupBuilder, Ribbon, RibbonCheckbox, RibbonCombobox, RibbonDialogLauncher, RibbonDropdown,
    RibbonEvent, RibbonLarge, RibbonMenu, RibbonSmall, RibbonSplit, RibbonState, RibbonTab, RibbonToggle,
    TabBuilder,
};

use super::Page;

/// How many events the log under the ribbon keeps.
const LOG_LEN: usize = 8;

/// The values the ribbon's controls are bound to.
#[derive(Default)]
struct Values {
    bold: bool,
    italic: bool,
    underline: bool,
    bullets: bool,
    ruler: bool,
    font: usize,
    size: String,
}

/// Gallery page for the ribbon's groups and controls (WI-1563).
#[derive(Default)]
pub struct RibbonPage {
    state: RibbonState,
    values: Values,
    log: Vec<String>,
}

impl Page for RibbonPage {
    fn name(&self) -> &'static str {
        "Ribbon"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let response = Ribbon::new("gallery-ribbon")
            .file_tab(FileTab::new("file", "File"))
            .tab(home(&mut self.values))
            .tab(insert())
            .show(ui, &mut self.state);

        for event in &response.events {
            self.push_log(event);
        }

        ui.add_space(8.0);
        ui.label("Events, newest last:");
        for line in &self.log {
            ui.monospace(line);
        }
    }
}

impl RibbonPage {
    fn push_log(&mut self, event: &RibbonEvent) {
        self.log.push(format!("{} {:?}", event.key, event.kind));
        if self.log.len() > LOG_LEN {
            self.log.remove(0);
        }
    }
}

/// The Home-like tab: Clipboard, Font and Paragraph groups.
fn home<'a>(values: &'a mut Values) -> RibbonTab<'a> {
    let Values {
        bold,
        italic,
        underline,
        bullets,
        ruler,
        font,
        size,
    } = values;
    RibbonTab::new("home", "Home", move |tab: &mut TabBuilder<'a>| {
        tab.group("clipboard", "Clipboard", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste").shortcut("Ctrl+V"));
            group.add(RibbonSmall::new("cut", IconName::Cut, "Cut").shortcut("Ctrl+X"));
            group.add(RibbonSmall::new("copy", IconName::Copy, "Copy").shortcut("Ctrl+C"));
            group.add(RibbonSplit::new(
                "paste_more",
                IconName::ClipboardPaste,
                "Paste as",
                |ui| {
                    ui.label("Plain text");
                    ui.label("Picture");
                },
            ));
            group.dialog_launcher(RibbonDialogLauncher::new("clipboard_dialog"));
        });
        tab.group("font", "Font", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonDropdown::new(
                "font_name",
                "Font",
                &["Sans", "Serif", "Mono"],
                font,
            ));
            group.add(RibbonCombobox::new(
                "font_size",
                "Size",
                &["8", "10", "12", "14"],
                size,
            ));
            group.add(RibbonToggle::new("bold", IconName::TextBold, "Bold", bold).shortcut("Ctrl+B"));
            group.add(RibbonToggle::new("italic", IconName::TextItalic, "Italic", italic).shortcut("Ctrl+I"));
            group.add(RibbonToggle::new(
                "underline",
                IconName::TextUnderline,
                "Underline",
                underline,
            ));
            group.add(
                RibbonMenu::new("more", IconName::Settings, "More", |ui| {
                    ui.label("Strikethrough");
                    ui.label("Superscript");
                })
                .small(),
            );
            group.dialog_launcher(RibbonDialogLauncher::new("font_dialog"));
        });
        tab.group("paragraph", "Paragraph", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonToggle::new("bullets", IconName::Star, "Bullets", bullets));
            group.add(RibbonCheckbox::new("ruler", "Ruler", ruler));
            group.add(RibbonSmall::new("indent", IconName::ArrowRight, "Indent"));
            group.add(RibbonSmall::new("outdent", IconName::ArrowLeft, "Outdent"));
        });
    })
}

/// The Insert-like tab: one group of large and small controls.
fn insert<'a>() -> RibbonTab<'a> {
    RibbonTab::new("insert", "Insert", |tab: &mut TabBuilder<'a>| {
        tab.group(
            "illustrations",
            "Illustrations",
            |group: &mut GroupBuilder<'a>| {
                group.add(RibbonLarge::new("table", IconName::Table, "Table"));
                group.add(RibbonLarge::new("picture", IconName::Image, "Picture"));
                group.add(RibbonSmall::new("shape", IconName::Star, "Shape"));
                group.add(RibbonSmall::new("home", IconName::Home, "Home shape"));
            },
        );
    })
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use fluent2_egui::Theme;

    use super::*;

    #[test]
    fn pressing_bold_reports_it_under_the_ribbon() {
        let mut page = RibbonPage::default();
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::install(ui.ctx(), &Theme::default());
            page.ui(ui);
        });
        harness.run();
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Bold")
            .click();
        harness.run();
        assert!(harness.query_by_label("bold Changed").is_some());
    }
}
