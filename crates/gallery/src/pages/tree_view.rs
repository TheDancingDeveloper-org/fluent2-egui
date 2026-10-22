//! TreeView: a torrent's files, 5,000 of them in 50 folders, with tri-state checks and size,
//! progress and priority columns. The page owns the checks and derives each folder's state.

use std::collections::BTreeSet;

use fluent2_egui::controls::{CheckState, Dropdown, ProgressBar};
use fluent2_egui::data::SelectionMode;
use fluent2_egui::data::tree_view::{Children, TreeColumn, TreeNode, TreeState, TreeView};

use super::Page;

const FOLDERS: u64 = 50;
const FILES_PER_FOLDER: u64 = 100;
const PRIORITIES: [&str; 3] = ["Low", "Normal", "High"];

/// Folders are keys `0..FOLDERS`; files are `FOLDERS + folder * FILES_PER_FOLDER + index`.
fn files_of(folder: u64) -> std::ops::Range<u64> {
    let first = FOLDERS + folder * FILES_PER_FOLDER;
    first..first + FILES_PER_FOLDER
}

pub struct TreeViewPage {
    state: TreeState<u64>,
    /// Checked files. Folder checks are derived.
    checked: BTreeSet<u64>,
    /// Priority index per file, by `key - FOLDERS`.
    priority: Vec<Option<usize>>,
}

impl Default for TreeViewPage {
    fn default() -> Self {
        let mut state = TreeState::default();
        state.expanded.insert(0);
        Self {
            state,
            checked: files_of(0).step_by(3).collect(),
            priority: vec![Some(1); (FOLDERS * FILES_PER_FOLDER) as usize],
        }
    }
}

impl TreeViewPage {
    fn folder_state(&self, folder: u64) -> CheckState {
        let on = files_of(folder)
            .filter(|file| self.checked.contains(file))
            .count() as u64;
        match on {
            0 => CheckState::Unchecked,
            n if n == FILES_PER_FOLDER => CheckState::Checked,
            _ => CheckState::Mixed,
        }
    }

    fn toggle(&mut self, key: u64) {
        if key < FOLDERS {
            // Mixed and unchecked folders check every file; a checked folder clears them.
            let check = self.folder_state(key) != CheckState::Checked;
            for file in files_of(key) {
                if check {
                    self.checked.insert(file);
                } else {
                    self.checked.remove(&file);
                }
            }
        } else if !self.checked.remove(&key) {
            self.checked.insert(key);
        }
    }
}

/// A made-up size in KB for `key`.
fn size_kb(key: u64) -> u64 {
    if key < FOLDERS {
        files_of(key).map(size_kb).sum()
    } else {
        (key * 37) % 9_000 + 12
    }
}

impl Page for TreeViewPage {
    fn name(&self) -> &'static str {
        "Tree view"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tree view");
        ui.label(format!(
            "{} files in {FOLDERS} folders. Space or a click toggles a check; folders show mixed when partly checked.",
            FOLDERS * FILES_PER_FOLDER
        ));
        ui.add_space(8.0);

        let columns = [
            TreeColumn::new("Size", 90.0),
            TreeColumn::new("Progress", 120.0),
            TreeColumn::new("Priority", 110.0),
        ];
        let folder_states: Vec<CheckState> = (0..FOLDERS).map(|folder| self.folder_state(folder)).collect();
        let checked = &self.checked;
        let priority = &mut self.priority;
        let response = TreeView::new("gallery-tree")
            .label("Torrent files")
            .selection_mode(SelectionMode::Multi)
            .columns(&columns, |ui, &key, column| match column {
                0 => {
                    ui.label(format!("{} KB", size_kb(key)));
                }
                1 => {
                    let done = (key % 11) as f32 / 10.0;
                    ui.add(ProgressBar::new(Some(done.min(1.0))));
                }
                _ => {
                    if let Some(slot) = key
                        .checked_sub(FOLDERS)
                        .and_then(|index| priority.get_mut(index as usize))
                    {
                        ui.add(Dropdown::new(("priority", key), slot, &PRIORITIES).width(96.0));
                    }
                }
            })
            .show(ui, &mut self.state, |parent, out| match parent {
                None => {
                    for (folder, state) in (0..FOLDERS).zip(folder_states.iter()) {
                        out.push(
                            TreeNode::new(folder, format!("Folder {folder}"))
                                .children(Children::Some)
                                .checked(*state),
                        );
                    }
                }
                Some(&folder) if folder < FOLDERS => {
                    for file in files_of(folder) {
                        out.push(
                            TreeNode::new(file, format!("file-{file}.bin"))
                                .checked(checked.contains(&file).into()),
                        );
                    }
                }
                Some(_) => {}
            });
        if let Some(key) = response.check_toggled {
            self.toggle(key);
        }
    }
}
