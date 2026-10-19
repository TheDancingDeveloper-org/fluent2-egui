//! Every component on one page, in Tab order, for screen-reader walks (WI-1586).
//!
//! `cargo run -p gallery --example a11y_walk`. Tab moves through the controls, surfaces and
//! data views in the order `docs/a11y-report.md` §4 lists them. The Dialog and Toast buttons
//! open their surfaces so the reader's announcement of each can be recorded.

use eframe::egui;
use fluent2_egui::controls::Button;
use fluent2_egui::data::SelectionMode;
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::data::tree_view::{Children, TreeNode, TreeState, TreeView};
use fluent2_egui::surfaces::{Intent, MenuItem, MessageBar, Toast, ToastPosition, Toaster};
use fluent2_egui::{
    CheckState, Checkbox, Combobox, Dialog, Dropdown, Input, Link, ProgressBar, RadioGroup, Slider,
    SpinButton, Switch, TabList, Textarea, Theme, ToggleButton, install,
};

fn main() -> eframe::Result {
    eframe::run_native(
        "fluent2-egui accessibility walk",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 980.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            install(&cc.egui_ctx, &Theme::default());
            Ok(Box::<Walk>::default())
        }),
    )
}

#[derive(Default)]
struct Walk {
    toggle: bool,
    check: CheckState,
    radio: usize,
    switch: bool,
    name: String,
    notes: String,
    volume: f64,
    copies: f64,
    colour: Option<usize>,
    font: String,
    tab: usize,
    dialog: bool,
    tree: TreeState<u32>,
    list: ListState,
}

impl eframe::App for Walk {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.page(ui));
        });
        let _ = Dialog::new("walk-dialog")
            .title("Save changes?")
            .actions(|actions| {
                actions.primary("Save");
                actions.secondary("Cancel");
            })
            .show(ctx, &mut self.dialog, |ui| {
                ui.label("Your document has unsaved changes.");
            });
        Toaster::show(ctx, ToastPosition::BottomEnd);
    }
}

impl Walk {
    fn page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(Button::new("Save"));
            ui.add(ToggleButton::new(&mut self.toggle, "Bold"));
            if ui.add(Button::new("Open dialog")).clicked() {
                self.dialog = true;
            }
            if ui.add(Button::new("Show toast")).clicked() {
                let _ = Toaster::push(
                    ui.ctx(),
                    Toast {
                        intent: Intent::Success,
                        title: "Saved".to_owned(),
                        body: Some("All changes are saved.".to_owned()),
                        timeout: None,
                        action: None,
                    },
                );
            }
            ui.menu_button("File", |ui| {
                let _ = ui.add(MenuItem::new("New"));
                let _ = ui.add(MenuItem::new("Close"));
            });
        });
        ui.add(Checkbox::new(&mut self.check, "Agree to the terms"));
        RadioGroup::new(&mut self.radio)
            .label("Size")
            .option("Small")
            .option("Large")
            .horizontal()
            .show(ui);
        ui.add(Switch::new(&mut self.switch, "Wi-Fi"));
        ui.add(Input::new(&mut self.name).placeholder("Name"));
        ui.add(Textarea::new(&mut self.notes).label("Notes"));
        ui.add(Slider::new(&mut self.volume, 0.0..=10.0).label("Volume"));
        ui.add(
            SpinButton::new(&mut self.copies)
                .range(1.0..=99.0)
                .label("Copies"),
        );
        ui.add(Dropdown::new("colour", &mut self.colour, &["Red", "Green", "Blue"]).placeholder("Colour"));
        ui.add(Combobox::new("font", &mut self.font, &["Arial", "Calibri"]).placeholder("Font"));
        TabList::new("tabs", &mut self.tab)
            .tab("Home")
            .tab("Insert")
            .tab("View")
            .show(ui);
        ui.add(Link::new("Help"));
        ui.add(ProgressBar::new(Some(0.4)));
        let _ = MessageBar::new(Intent::Warning, "Disk is almost full")
            .title("Storage")
            .show(ui);
        TreeView::new("tree")
            .label("Files")
            .selection_mode(SelectionMode::Single)
            .show(ui, &mut self.tree, |parent, out| match parent {
                None => {
                    out.push(TreeNode::new(1, "Documents").children(Children::Some));
                    out.push(TreeNode::new(2, "Notes"));
                }
                Some(1) => out.push(TreeNode::new(11, "Report")),
                _ => {}
            });
        let rows = ["Inbox", "Sent", "Drafts"];
        List::new("list", rows.len())
            .label("Folders")
            .selection_mode(SelectionMode::Single)
            .show(ui, &mut self.list, |cell, ui| {
                ui.label(rows.get(cell.row).copied().unwrap_or_default());
            });
    }
}
