//! Docking page: a DockArea of document panes between an Explorer and an Output side dock.
//!
//! Drag a tab onto another pane to move it, or near a pane's edge to split that pane.

use fluent2_egui::controls::Button;
use fluent2_egui::surfaces::{
    DockArea, DockEdge, DockSide, DockTree, DockViewer, SideDock, SideDockState, StripTab,
};

use fluent2_egui::platform::LayoutStore;

use super::Page;

/// Files the Explorer offers.
const FILES: [&str; 6] = [
    "main.rs",
    "lib.rs",
    "dock.rs",
    "README.md",
    "Cargo.toml",
    "notes.txt",
];

/// Gallery page for docking.
pub struct DockingPage {
    tree: DockTree<String>,
    explorer: SideDockState,
    output: SideDockState,
    log: Vec<String>,
}

impl Default for DockingPage {
    fn default() -> Self {
        let mut tree = DockTree::new(vec!["main.rs".to_owned(), "lib.rs".to_owned()]);
        if let Some(first) = tree.pane_ids().first().copied() {
            let _ = tree.split(first, DockSide::Right, "README.md".to_owned());
        }
        Self {
            tree,
            explorer: SideDockState {
                open: true,
                size: 180.0,
            },
            output: SideDockState {
                open: true,
                size: 120.0,
            },
            log: vec!["Drag a tab between panes or to a pane edge.".to_owned()],
        }
    }
}

struct Viewer;

impl DockViewer<String> for Viewer {
    fn tab(&mut self, tab: &String) -> StripTab {
        StripTab::new(tab.clone())
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) {
        ui.label(format!("Contents of {tab}"));
    }
}

impl Page for DockingPage {
    fn name(&self) -> &'static str {
        "Docking"
    }

    fn save_layout(&self, store: &mut LayoutStore) {
        store.put("docking_tree", &self.tree);
        store.put("docking_explorer", &self.explorer);
        store.put("docking_output", &self.output);
    }

    fn load_layout(&mut self, store: &LayoutStore) {
        store.restore("docking_tree", &mut self.tree);
        store.restore("docking_explorer", &mut self.explorer);
        store.restore("docking_output", &mut self.output);
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Docking");
        ui.horizontal(|ui| {
            if !self.explorer.open
                && ui
                    .add(Button::new("Show Explorer").id_salt("show_explorer"))
                    .clicked()
            {
                self.explorer.open = true;
            }
            if !self.output.open
                && ui
                    .add(Button::new("Show Output").id_salt("show_output"))
                    .clicked()
            {
                self.output.open = true;
            }
        });
        ui.add_space(8.0);
        // Side docks run their closures in one call, so each records what happened and the
        // page applies it afterwards.
        let recent: Vec<String> = self.log.iter().rev().take(6).cloned().collect();
        let mut opened = None;
        let mut closed = Vec::new();
        let tree = &mut self.tree;
        let output = &mut self.output;
        SideDock::new("gallery_explorer", DockEdge::Left, "Explorer").show(
            ui,
            &mut self.explorer,
            |ui| {
                for file in FILES {
                    if ui.selectable_label(false, file).clicked() {
                        opened = Some(file);
                    }
                }
            },
            |ui| {
                SideDock::new("gallery_output", DockEdge::Bottom, "Output").show(
                    ui,
                    output,
                    |ui| {
                        for line in &recent {
                            ui.label(line.as_str());
                        }
                    },
                    |ui| {
                        if tree.is_empty() {
                            ui.label("Every tab is closed. Open a file from the Explorer.");
                        }
                        closed = DockArea::new("gallery_dock").show(ui, tree, &mut Viewer).closed;
                    },
                );
            },
        );
        if let Some(file) = opened {
            let first = self.tree.pane_ids().first().copied();
            self.tree.add_tab(first, file.to_owned());
            self.log.push(format!("Opened {file}"));
        }
        for tab in closed {
            self.log.push(format!("Closed {tab}"));
        }
    }
}
