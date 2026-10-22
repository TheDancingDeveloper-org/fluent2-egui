//! TreeView: expansion by click and keyboard, lazy children, selection, virtualisation and
//! AccessKit, without the wgpu feature (`docs/specs/data-views.md` §5).

use std::cell::{Cell, RefCell};

use egui::accesskit::{Role, Toggled};
use egui::{Event, Key, Modifiers, PointerButton, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::SelectionMode;
use fluent2_egui::data::tree_view::{Children, TreeColumn, TreeNode, TreeState, TreeView};
use fluent2_egui::{CheckState, ProgressBar};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<TreeState<u64>>,
    /// Every `expanded` report, in order.
    expanded: RefCell<Vec<u64>>,
    /// Every `collapsed` report, in order.
    collapsed: RefCell<Vec<u64>>,
    /// Every key the children callback was asked for.
    asked: RefCell<Vec<Option<u64>>>,
    /// Key of the last `activated` report.
    activated: Cell<Option<u64>>,
    /// Whether the app answers the `expanded` report by loading node 9's children.
    auto_load: Cell<bool>,
    /// How many times the app started loading node 9.
    loads: Cell<usize>,
    /// Whether node 9's children have arrived.
    loaded: Cell<bool>,
    /// The tree is the 10,000-leaf tree rather than the small sample.
    big: Cell<bool>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(TreeState::default()),
        expanded: RefCell::new(Vec::new()),
        collapsed: RefCell::new(Vec::new()),
        asked: RefCell::new(Vec::new()),
        activated: Cell::new(None),
        auto_load: Cell::new(true),
        loads: Cell::new(0),
        loaded: Cell::new(false),
        big: Cell::new(false),
    }))
}

/// The sample tree:
///
/// ```text
/// Documents (1)
///   Reports (11)
///     Q1 (111)
///   Invoices (12)
/// Music (2)
///   Albums (21)
/// Notes (3)
/// Remote (9), loads its child on first expand
/// ```
fn sample(probe: &Probe, parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>) {
    match parent {
        None => {
            out.push(TreeNode::new(1, "Documents").children(Children::Some));
            out.push(TreeNode::new(2, "Music").children(Children::Some));
            out.push(TreeNode::new(3, "Notes"));
            let remote = if probe.loaded.get() {
                Children::Some
            } else {
                Children::Loading
            };
            out.push(TreeNode::new(9, "Remote").children(remote));
        }
        Some(1) => {
            out.push(TreeNode::new(11, "Reports").children(Children::Some));
            out.push(TreeNode::new(12, "Invoices"));
        }
        Some(11) => out.push(TreeNode::new(111, "Q1")),
        Some(2) => out.push(TreeNode::new(21, "Albums")),
        Some(9) => {
            if probe.loaded.get() {
                out.push(TreeNode::new(91, "Remote child"));
            }
        }
        Some(_) => {}
    }
}

/// Ten thousand leaves under node 0.
fn big(parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>) {
    match parent {
        None => out.push(TreeNode::new(0, "Root").children(Children::Some)),
        Some(0) => {
            for key in 1..=10_000 {
                out.push(TreeNode::new(key, format!("Leaf {key}")));
            }
        }
        Some(_) => {}
    }
}

fn harness(probe: &'static Probe, mode: SelectionMode) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(480.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let mut state = probe.state.borrow_mut();
            let response = TreeView::new("tree").label("Files").selection_mode(mode).show(
                ui,
                &mut state,
                |parent, out| {
                    probe.asked.borrow_mut().push(parent.copied());
                    if probe.big.get() {
                        big(parent, out);
                    } else {
                        sample(probe, parent, out);
                    }
                },
            );
            if let Some(key) = response.expanded {
                probe.expanded.borrow_mut().push(key);
                if key == 9 && probe.auto_load.get() && !probe.loaded.get() {
                    probe.loads.set(probe.loads.get() + 1);
                    probe.loaded.set(true);
                }
            }
            if let Some(key) = response.collapsed {
                probe.collapsed.borrow_mut().push(key);
            }
            if let Some(key) = response.activated {
                probe.activated.set(Some(key));
            }
        })
}

fn selected(probe: &Probe) -> Vec<u64> {
    probe.state.borrow().selection.iter().copied().collect()
}

fn expanded(probe: &Probe) -> Vec<u64> {
    probe.state.borrow().expanded.iter().copied().collect()
}

/// Click the centre of the node labelled `label`, with `modifiers` held down.
fn click(harness: &mut Harness<'static>, label: &str, modifiers: Modifiers) {
    let pos = harness.get_by_label(label).rect().center();
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::ModifiersChanged(modifiers));
    harness.run_steps(1);
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.run_steps(3);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(1);
}

/// Press `key` and let the expansion it causes land in the next frame.
fn press(harness: &mut Harness<'static>, key: Key) {
    harness.key_press(key);
    harness.run_steps(2);
}

#[test]
fn chevron_click_expands_and_collapses_without_selecting() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    harness.get_by_label("Expand Documents").click();
    harness.run_steps(3);
    assert_eq!(expanded(probe), vec![1]);
    assert_eq!(*probe.expanded.borrow(), vec![1]);
    assert!(selected(probe).is_empty(), "the chevron must not select its row");
    assert!(harness.query_by_label("Reports").is_some());

    harness.get_by_label("Collapse Documents").click();
    harness.run_steps(3);
    assert!(expanded(probe).is_empty());
    assert_eq!(*probe.collapsed.borrow(), vec![1]);
    assert!(harness.query_by_label("Reports").is_none());
}

#[test]
fn clicking_a_row_selects_it_in_single_mode() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    click(&mut harness, "Notes", Modifiers::NONE);
    assert_eq!(selected(probe), vec![3]);
    assert_eq!(probe.state.borrow().focus, Some(3));
}

#[test]
fn right_expands_then_enters_the_first_child() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    click(&mut harness, "Documents", Modifiers::NONE);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(expanded(probe), vec![1]);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(probe.state.borrow().focus, Some(11));
    assert_eq!(selected(probe), vec![11]);
}

#[test]
fn left_on_a_child_moves_to_its_parent_and_left_on_an_open_node_collapses_it() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    click(&mut harness, "Documents", Modifiers::NONE);
    press(&mut harness, Key::ArrowRight);
    click(&mut harness, "Reports", Modifiers::NONE);
    press(&mut harness, Key::ArrowLeft);
    assert_eq!(
        probe.state.borrow().focus,
        Some(1),
        "Left on a leaf goes to the parent"
    );
    assert_eq!(selected(probe), vec![1]);

    // Left on an open node collapses it instead of moving.
    press(&mut harness, Key::ArrowLeft);
    assert!(expanded(probe).is_empty());
    assert_eq!(*probe.collapsed.borrow(), vec![1]);
}

#[test]
fn up_and_down_walk_the_visible_rows_only() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    click(&mut harness, "Documents", Modifiers::NONE);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        selected(probe),
        vec![2],
        "Documents is collapsed, so Music is next"
    );
    press(&mut harness, Key::ArrowUp);
    assert_eq!(selected(probe), vec![1]);
}

#[test]
fn star_expands_every_sibling_that_has_children() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    click(&mut harness, "Documents", Modifiers::NONE);
    harness.event(Event::Text("*".to_owned()));
    harness.run_steps(3);
    assert_eq!(
        expanded(probe),
        vec![1, 2, 9],
        "Notes is a leaf and is not expanded"
    );
}

#[test]
fn lazy_children_are_asked_for_only_once_the_node_is_expanded() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    assert!(
        !probe.asked.borrow().contains(&Some(9)),
        "a collapsed node is never asked for its children"
    );
    harness.get_by_label("Expand Remote").click();
    harness.run_steps(3);
    assert_eq!(probe.loads.get(), 1, "the app starts loading on the first expand");
    assert!(harness.query_by_label("Remote child").is_some());

    // Collapse and expand again: the app already holds the children, so nothing is requested.
    harness.get_by_label("Collapse Remote").click();
    harness.run_steps(3);
    harness.get_by_label("Expand Remote").click();
    harness.run_steps(3);
    assert_eq!(probe.loads.get(), 1);
    assert_eq!(
        *probe.expanded.borrow(),
        vec![9, 9],
        "each expand is reported once"
    );
}

#[test]
fn a_loading_node_shows_a_busy_loading_row_until_children_arrive() {
    let probe = probe();
    probe.auto_load.set(false);
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    harness.get_by_label("Expand Remote").click();
    harness.run_steps(3);
    let loading = harness.get_by_role_and_label(Role::TreeItem, "Loading");
    assert!(loading.accesskit_node().data().is_busy());
}

#[test]
fn ten_thousand_expanded_nodes_render_fewer_than_500_rows() {
    let probe = probe();
    probe.big.set(true);
    probe.state.borrow_mut().expanded.insert(0);
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(3);
    let rows = harness.query_all_by_role(Role::TreeItem).count();
    assert!(rows > 0, "some rows are in view");
    assert!(rows < 500, "{rows} rows were built for one frame");
}

#[test]
fn accesskit_tree_and_items_carry_level_and_expanded() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single);
    harness.run_steps(2);
    assert!(harness.query_by_role(Role::Tree).is_some());
    harness.get_by_label("Expand Documents").click();
    harness.run_steps(3);
    let documents = harness.get_by_role_and_label(Role::TreeItem, "Documents");
    assert_eq!(documents.accesskit_node().data().level(), Some(1));
    assert_eq!(documents.accesskit_node().data().is_expanded(), Some(true));
    let reports = harness.get_by_role_and_label(Role::TreeItem, "Reports");
    assert_eq!(reports.accesskit_node().data().level(), Some(2));
}

#[test]
fn multi_selection_uses_ctrl_and_shift_as_in_the_grid() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi);
    harness.run_steps(2);
    click(&mut harness, "Notes", Modifiers::NONE);
    click(&mut harness, "Documents", Modifiers::CTRL);
    assert_eq!(selected(probe), vec![1, 3]);
    click(&mut harness, "Music", Modifiers::SHIFT);
    assert_eq!(
        selected(probe),
        vec![1, 2],
        "Shift replaces with the range from the anchor (Documents)"
    );
}

#[test]
fn enter_reports_activation_and_none_mode_enter_toggles_a_parent() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None);
    harness.run_steps(2);
    click(&mut harness, "Documents", Modifiers::NONE);
    press(&mut harness, Key::Enter);
    assert_eq!(probe.activated.get(), Some(1));
    assert_eq!(
        expanded(probe),
        vec![1],
        "Enter on a parent in None mode expands it"
    );
    assert!(selected(probe).is_empty());
}

/// A checked tree with four cell columns: folders 0..50 of 100 files each (5,050 nodes), every
/// folder expanded. Folder checks are derived from their files, as an app does.
struct Checks {
    state: RefCell<TreeState<u64>>,
    /// Checked files.
    files: RefCell<std::collections::BTreeSet<u64>>,
    /// Every `check_toggled` report.
    toggled: RefCell<Vec<u64>>,
    /// How many cells were drawn in the last frame.
    cells: Cell<usize>,
}

fn checks() -> &'static Checks {
    let mut state = TreeState::default();
    state.expanded.extend(0..50);
    Box::leak(Box::new(Checks {
        state: RefCell::new(state),
        files: RefCell::new(std::collections::BTreeSet::new()),
        toggled: RefCell::new(Vec::new()),
        cells: Cell::new(0),
    }))
}

/// File keys of folder `folder`.
fn files_of(folder: u64) -> std::ops::Range<u64> {
    let first = 1_000 + folder * 100;
    first..first + 100
}

fn folder_state(checks: &Checks, folder: u64) -> CheckState {
    let files = checks.files.borrow();
    let on = files_of(folder).filter(|file| files.contains(file)).count();
    match on {
        0 => CheckState::Unchecked,
        100 => CheckState::Checked,
        _ => CheckState::Mixed,
    }
}

fn checks_harness(checks: &'static Checks) -> Harness<'static> {
    let columns = [
        TreeColumn::new("Size", 70.0),
        TreeColumn::new("Progress", 90.0),
        TreeColumn::new("Priority", 70.0),
        TreeColumn::new("Done", 50.0),
    ];
    Harness::builder()
        .with_size(Vec2::new(640.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            checks.cells.set(0);
            let mut state = checks.state.borrow_mut();
            let response = TreeView::new("checks")
                .label("Files")
                .columns(&columns, |ui, key, column| {
                    checks.cells.set(checks.cells.get() + 1);
                    match column {
                        0 => ui.label(format!("{key} KB")),
                        1 => ui.add(ProgressBar::new(Some(0.5))),
                        _ => ui.label("Normal"),
                    };
                })
                .show(ui, &mut state, |parent, out| match parent {
                    None => {
                        for folder in 0..50 {
                            out.push(
                                TreeNode::new(folder, format!("Folder {folder}"))
                                    .children(Children::Some)
                                    .checked(folder_state(checks, folder)),
                            );
                        }
                    }
                    Some(&folder) if folder < 50 => {
                        let files = checks.files.borrow();
                        for file in files_of(folder) {
                            out.push(
                                TreeNode::new(file, format!("File {file}"))
                                    .checked(files.contains(&file).into()),
                            );
                        }
                    }
                    Some(_) => {}
                });
            if let Some(key) = response.check_toggled {
                checks.toggled.borrow_mut().push(key);
                let mut files = checks.files.borrow_mut();
                if key < 50 {
                    // A folder: Mixed and Unchecked become Checked, Checked becomes Unchecked.
                    let all = files_of(key).all(|file| files.contains(&file));
                    for file in files_of(key) {
                        if all {
                            files.remove(&file);
                        } else {
                            files.insert(file);
                        }
                    }
                } else if !files.remove(&key) {
                    files.insert(key);
                }
            }
        })
}

#[test]
fn a_five_thousand_node_checked_tree_draws_only_visible_cells() {
    let checks = checks();
    let mut harness = checks_harness(checks);
    harness.run_steps(2);
    let cells = checks.cells.get();
    assert!(cells > 0, "visible rows draw their cells");
    assert!(cells < 4 * 40, "only visible rows draw cells, drew {cells}");
    assert!(harness.query_by_label("File 1000").is_some());
    assert!(
        harness.query_by_label("File 5999").is_none(),
        "far rows are not built"
    );
}

#[test]
fn clicking_a_file_check_reports_it_and_the_folder_turns_mixed() {
    let checks = checks();
    let mut harness = checks_harness(checks);
    harness.run_steps(2);
    harness.get_by_label("Check File 1001").click();
    harness.run_steps(3);
    assert_eq!(*checks.toggled.borrow(), vec![1001]);
    assert!(
        checks.state.borrow().selection.is_empty(),
        "a check click does not select"
    );
    let folder = harness.get_by_role_and_label(Role::TreeItem, "Folder 0");
    assert_eq!(folder.accesskit_node().toggled(), Some(Toggled::Mixed));
    let file = harness.get_by_role_and_label(Role::TreeItem, "File 1001");
    assert_eq!(file.accesskit_node().toggled(), Some(Toggled::True));
    let other = harness.get_by_role_and_label(Role::CheckBox, "Check File 1002");
    assert_eq!(other.accesskit_node().toggled(), Some(Toggled::False));

    // The Mixed folder's check makes every file checked.
    harness.get_by_label("Check Folder 0").click();
    harness.run_steps(3);
    let folder = harness.get_by_role_and_label(Role::TreeItem, "Folder 0");
    assert_eq!(folder.accesskit_node().toggled(), Some(Toggled::True));
}

#[test]
fn space_toggles_the_check_of_the_focused_node() {
    let checks = checks();
    let mut harness = checks_harness(checks);
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::TreeItem, "Folder 0").click();
    harness.run_steps(2);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Space);
    assert_eq!(*checks.toggled.borrow(), vec![1000]);
    assert!(checks.files.borrow().contains(&1000));
    let file = harness.get_by_role_and_label(Role::TreeItem, "File 1000");
    assert_eq!(file.accesskit_node().toggled(), Some(Toggled::True));
}
