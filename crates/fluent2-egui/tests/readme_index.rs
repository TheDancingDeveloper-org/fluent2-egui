//! The README's component index lists every public widget (WI-1933).
//!
//! Every CamelCase name re-exported by `controls`, `surfaces` and `data` must either appear in
//! the README in backticks or be listed here as a supporting type (an option enum, a response,
//! a state). A new export fails this test until it gets a README row or an entry below.

use std::collections::BTreeSet;

/// Re-exported names that are not components in their own right.
const SUPPORTING: &[&str] = &[
    // controls
    "Appearance",
    "AvatarColor",
    "AvatarGroupLayout",
    "AvatarShape",
    "BadgeAppearance",
    "BadgeColor",
    "BadgeSize",
    "ButtonSize",
    "CalendarDate",
    "CalendarResponse",
    "CalendarStrings",
    "CheckState",
    "ComboboxOptions",
    "FilterProbe",
    "ModelCombobox",
    "SliceOptions",
    "DayContent",
    "FileCardResponse",
    "InputAppearance",
    "PageItem",
    "PieceState",
    "PagerResponse",
    "PickerResponse",
    "Presence",
    "ProgressBarSize",
    "SeriesSummary",
    "Shape",
    "SkeletonAnimation",
    "SkeletonItem",
    "SpinnerSize",
    "SplitButtonResponse",
    "TagAppearance",
    "TagGroupResponse",
    "TagPickerResponse",
    "TagResponse",
    "TagSize",
    "TagSuggestion",
    "TextareaResize",
    "TimeOfDay",
    "ValidationState",
    // surfaces
    "CardAppearance",
    "CloseReason",
    "DialogActions",
    "DialogKind",
    "DialogResult",
    "DockEdge",
    "DockNode",
    "DockResponse",
    "DockSide",
    "DockViewer",
    "DrawerPosition",
    "DrawerSize",
    "DropTarget",
    "Intent",
    "MessageBarResponse",
    "PaletteCommand",
    "PaletteMatch",
    "Pane",
    "PaneId",
    "SideDockResponse",
    "SideDockState",
    "StripTab",
    "TabAppearance",
    "TabMove",
    "TabSize",
    "TabStripResponse",
    "Toast",
    "ToastId",
    "ToastPosition",
    "TooltipExt",
    "TooltipTone",
    // data
    "CellCtx",
    "CellResponse",
    "DataGridResponse",
    "DataGridSize",
    "DataGridState",
    "DropChoice",
    "DropTargetResponse",
    "FocusMode",
    "MoveSource",
    "NavResponse",
    "NavState",
    "PaneMode",
    "ReorderAxis",
    "ReorderList",
    "ReorderMove",
    "ReorderResponse",
    "RowDrag",
    "RowDrop",
    "RowId",
    "SelectionMode",
    "Slot",
    "SplitAxis",
    "SplitResponse",
    "SplitSize",
    "SplitState",
];

/// CamelCase names in the `pub use` statements of a module file.
fn exported(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_use = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("pub use ") {
            in_use = true;
        }
        if in_use {
            let list = line
                .rsplit_once('{')
                .map_or_else(|| line.rsplit("::").next().unwrap_or(""), |(_, rest)| rest);
            for word in list.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')) {
                if word.chars().next().is_some_and(char::is_uppercase) && word.chars().any(char::is_lowercase)
                {
                    names.insert(word.to_owned());
                }
            }
            if line.ends_with(';') {
                in_use = false;
            }
        }
    }
    names
}

#[test]
fn every_public_widget_is_in_the_readme_index() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/");
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).unwrap_or_default();
    assert!(
        readme.contains("## Components"),
        "README has no Components section"
    );
    let mut missing = Vec::new();
    for module in ["controls", "surfaces", "data"] {
        let source = std::fs::read_to_string(format!("{root}src/{module}/mod.rs")).unwrap_or_default();
        let names = exported(&source);
        assert!(!names.is_empty(), "no exports read from {module}/mod.rs");
        for name in names {
            if SUPPORTING.contains(&name.as_str()) {
                continue;
            }
            if !readme.contains(&format!("`{name}`")) && !readme.contains(&format!("::{name}`")) {
                missing.push(format!("{module}::{name}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "public widgets missing from the README Components section (add a row, or list the name in \
         SUPPORTING if it is not a component): {missing:#?}"
    );
}

#[test]
fn supporting_names_are_still_exported() {
    // Keeps SUPPORTING from collecting names that no longer exist.
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/");
    let mut all = BTreeSet::new();
    for module in ["controls", "surfaces", "data"] {
        let source = std::fs::read_to_string(format!("{root}src/{module}/mod.rs")).unwrap_or_default();
        all.extend(exported(&source));
    }
    let stale: Vec<_> = SUPPORTING.iter().filter(|name| !all.contains(**name)).collect();
    assert!(
        stale.is_empty(),
        "SUPPORTING lists names no longer exported: {stale:?}"
    );
}

#[test]
fn readme_does_not_list_a_shipped_widget_as_missing() {
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).unwrap_or_default();
    let components = readme
        .split("## Components")
        .nth(1)
        .and_then(|rest| rest.split("### Not yet provided").next())
        .unwrap_or("");
    let missing = readme.split("### Not yet provided").nth(1).unwrap_or("");
    let mut contradictions = Vec::new();
    for line in missing.lines() {
        let Some(name) = line
            .trim()
            .strip_prefix("| ")
            .and_then(|rest| rest.split('|').next())
        else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || name == "Fluent 2" || name.starts_with('-') {
            continue;
        }
        if components.contains(&format!("`{name}`")) {
            contradictions.push(name.to_owned());
        }
    }
    assert!(
        contradictions.is_empty(),
        "README lists these as both shipped and not yet provided: {contradictions:?}"
    );
}
