//! Docking (feature `docking`): a pane split tree with a [`TabStrip`] per pane, dragging a tab
//! into another pane or onto a pane's edge to split it, and collapsible side docks.
//!
//! See `docs/specs/data-views.md` §9. The model ([`DockTree`]) is plain data the app owns: it
//! holds the app's tab values, serialises with the `serde` feature, and every operation keeps
//! it valid (no empty panes, no splits with one child). [`DockArea`] paints it, with each
//! split drawn by [`SplitView`](crate::data::SplitView). [`SideDock`] puts one collapsible
//! panel on an edge of whatever it wraps.
//!
//! Design references only, nothing copied: fluent-gpui `crates/fluent-layout/src/{dock,
//! pane_group,pane,workspace}.rs` (Apache-2.0), Zed's workspace panes (GPL), egui_tiles.

use std::cell::RefCell;

use egui::epaint::RectShape;
use egui::{CornerRadius, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::data::split_view::{SplitAxis, SplitSize, SplitState, SplitView};
use crate::foundation::text::{self, TypeRamp};
use crate::surfaces::{StripTab, TabMove, TabStrip};
use crate::theme::{Theme, c};

/// Share of a pane's width or height, from each edge, that drops into a new split.
const EDGE_ZONE: f32 = 0.25;
/// Height of a side dock's header.
const HEADER_HEIGHT: f32 = 32.0;

/// Identifies a pane in a [`DockTree`]. Stable for the pane's life, and across serde.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PaneId(pub u64);

/// Which side of a pane a new pane goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockSide {
    /// Left of the pane.
    Left,
    /// Right of the pane.
    Right,
    /// Above the pane.
    Top,
    /// Below the pane.
    Bottom,
}

impl DockSide {
    fn axis(self) -> SplitAxis {
        match self {
            DockSide::Left | DockSide::Right => SplitAxis::Horizontal,
            DockSide::Top | DockSide::Bottom => SplitAxis::Vertical,
        }
    }

    fn new_is_first(self) -> bool {
        matches!(self, DockSide::Left | DockSide::Top)
    }
}

/// Where a moved tab goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTarget {
    /// Into `pane`'s tabs at `index` (clamped to the end).
    Tabs {
        /// Target pane.
        pane: PaneId,
        /// Index among the pane's tabs after the move.
        index: usize,
    },
    /// Into a new pane split off `pane` on `side`.
    Split {
        /// Pane to split.
        pane: PaneId,
        /// Side the new pane goes on.
        side: DockSide,
    },
}

/// One pane: a list of tabs and the selected one.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pane<T> {
    /// The pane's id.
    pub id: PaneId,
    /// The app's tab values, in strip order. Never empty inside a tree.
    pub tabs: Vec<T>,
    /// Index of the selected tab.
    pub selected: usize,
}

/// A node of the split tree.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockNode<T> {
    /// A leaf pane.
    Pane(Pane<T>),
    /// Two children side by side (`Horizontal`) or stacked (`Vertical`).
    Split {
        /// Direction of the split.
        axis: SplitAxis,
        /// Size of the first child, as SplitView keeps it.
        state: SplitState,
        /// Left or top child.
        first: Box<DockNode<T>>,
        /// Right or bottom child.
        second: Box<DockNode<T>>,
    },
}

/// The app's docking layout: a binary split tree of panes holding tab values `T`.
///
/// ```
/// # use fluent2_egui::surfaces::{DockSide, DockTree};
/// let mut tree = DockTree::new(vec!["main.rs", "lib.rs"]);
/// let first = tree.pane_ids()[0];
/// let right = tree.split(first, DockSide::Right, "notes.md").unwrap();
/// assert_eq!(tree.pane(right).unwrap().tabs, ["notes.md"]);
/// ```
///
/// A deserialised tree is checked on load (see [`DockTree::normalize`]), so a hand-edited or
/// stale saved layout cannot hold empty panes, duplicate pane ids or a stale id counter.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(from = "DockTreeData<T>", bound(deserialize = "T: serde::Deserialize<'de>"))
)]
pub struct DockTree<T> {
    root: Option<DockNode<T>>,
    next_id: u64,
}

/// The serialised form of a [`DockTree`], normalised on its way in.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct DockTreeData<T> {
    root: Option<DockNode<T>>,
    next_id: u64,
}

#[cfg(feature = "serde")]
impl<T> From<DockTreeData<T>> for DockTree<T> {
    fn from(data: DockTreeData<T>) -> Self {
        let mut tree = Self {
            root: data.root,
            next_id: data.next_id,
        };
        tree.normalize();
        tree
    }
}

impl<T> Default for DockTree<T> {
    fn default() -> Self {
        Self {
            root: None,
            next_id: 0,
        }
    }
}

impl<T> DockTree<T> {
    /// One pane holding `tabs`, or an empty tree when `tabs` is empty.
    pub fn new(tabs: Vec<T>) -> Self {
        let mut tree = Self::default();
        if !tabs.is_empty() {
            let id = tree.fresh_id();
            tree.root = Some(DockNode::Pane(Pane {
                id,
                tabs,
                selected: 0,
            }));
        }
        tree
    }

    /// The root node, `None` when every tab is closed.
    pub fn root(&self) -> Option<&DockNode<T>> {
        self.root.as_ref()
    }

    /// Whether the tree has no tabs.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Pane ids in reading order (left to right, top to bottom).
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            collect_ids(root, &mut out);
        }
        out
    }

    /// The pane with `id`.
    pub fn pane(&self, id: PaneId) -> Option<&Pane<T>> {
        self.root.as_ref().and_then(|root| find_pane(root, id))
    }

    /// The pane with `id`, mutably.
    pub fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane<T>> {
        self.root.as_mut().and_then(|root| find_pane_mut(root, id))
    }

    /// Add `tab` to `pane` (or to the first pane when `pane` is `None` or unknown) and select
    /// it. In an empty tree this makes a new pane. Returns the pane it went into.
    pub fn add_tab(&mut self, pane: Option<PaneId>, tab: T) -> PaneId {
        let target = pane
            .filter(|id| self.pane(*id).is_some())
            .or_else(|| self.pane_ids().first().copied());
        if let Some(id) = target
            && let Some(pane) = self.pane_mut(id)
        {
            pane.tabs.push(tab);
            pane.selected = pane.tabs.len() - 1;
            return id;
        }
        let id = self.fresh_id();
        self.root = Some(DockNode::Pane(Pane {
            id,
            tabs: vec![tab],
            selected: 0,
        }));
        id
    }

    /// Split `pane`, putting a new pane holding `tab` on `side`. The two share the space
    /// evenly. Returns the new pane, or gives `tab` back as the error when `pane` does not
    /// exist.
    pub fn split(&mut self, pane: PaneId, side: DockSide, tab: T) -> Result<PaneId, T> {
        let id = self.fresh_id();
        let Some(node) = self.root.as_mut().and_then(|root| find_node_mut(root, pane)) else {
            return Err(tab);
        };
        let placeholder = DockNode::Pane(Pane {
            id: PaneId(u64::MAX),
            tabs: Vec::new(),
            selected: 0,
        });
        let old = std::mem::replace(node, placeholder);
        let new = DockNode::Pane(Pane {
            id,
            tabs: vec![tab],
            selected: 0,
        });
        let (first, second) = if side.new_is_first() {
            (new, old)
        } else {
            (old, new)
        };
        *node = DockNode::Split {
            axis: side.axis(),
            state: SplitState::default(),
            first: Box::new(first),
            second: Box::new(second),
        };
        Ok(id)
    }

    /// Close tab `index` of `pane` and return it. A pane left empty is removed, and its
    /// sibling takes the parent split's place. The selection stays on the same tab when it
    /// can, else moves to the tab now at `index`, or the last one.
    pub fn close_tab(&mut self, pane: PaneId, index: usize) -> Option<T> {
        let tab = take_tab(self.pane_mut(pane)?, index)?;
        self.prune();
        Some(tab)
    }

    /// Move tab `index` of pane `from` to `target`. Returns `false`, changing nothing, when a
    /// pane or index does not exist or when the move would split a pane off itself with its
    /// only tab. The moved tab is selected where it lands.
    pub fn move_tab(&mut self, from: PaneId, index: usize, target: DropTarget) -> bool {
        let Some(source) = self.pane(from) else {
            return false;
        };
        if index >= source.tabs.len() {
            return false;
        }
        let source_len = source.tabs.len();
        match target {
            DropTarget::Tabs { pane, index: to } if pane == from => {
                let to = to.min(source_len - 1);
                if let Some(pane) = self.pane_mut(from) {
                    reorder(pane, TabMove { from: index, to });
                }
                true
            }
            DropTarget::Tabs { pane, index: to } => {
                if self.pane(pane).is_none() {
                    return false;
                }
                let Some(tab) = self.pane_mut(from).and_then(|source| take_tab(source, index)) else {
                    return false;
                };
                if let Some(target) = self.pane_mut(pane) {
                    let at = to.min(target.tabs.len());
                    target.tabs.insert(at, tab);
                    target.selected = at;
                }
                self.prune();
                true
            }
            DropTarget::Split { pane, side } => {
                if self.pane(pane).is_none() || (pane == from && source_len == 1) {
                    return false;
                }
                let Some(tab) = self.pane_mut(from).and_then(|source| take_tab(source, index)) else {
                    return false;
                };
                // The source keeps at least one tab when it is also the target, so the split
                // never sees an empty pane; elsewhere the empty source is pruned afterwards.
                let ok = self.split(pane, side, tab).is_ok();
                self.prune();
                ok
            }
        }
    }

    /// Reorder tabs inside `pane`, keeping the selection on the same tab.
    pub fn reorder_tab(&mut self, pane: PaneId, moved: TabMove) -> bool {
        match self.pane_mut(pane) {
            Some(pane) if moved.from < pane.tabs.len() && moved.to < pane.tabs.len() => {
                reorder(pane, moved);
                true
            }
            _ => false,
        }
    }

    /// Restore the tree's invariants: drop empty panes, collapse one-child splits, clamp
    /// selections, give a pane whose id repeats an earlier one a fresh id, and move the id
    /// counter past every id in use. Every operation keeps these already; this is for trees
    /// built by hand or loaded from storage, and runs on every serde load.
    pub fn normalize(&mut self) {
        self.prune();
        let ids = self.pane_ids();
        let max = ids.iter().map(|id| id.0).filter(|id| *id != u64::MAX).max();
        if let Some(max) = max {
            self.next_id = self.next_id.max(max.saturating_add(1));
        }
        let mut seen = std::collections::HashSet::new();
        let mut next_id = self.next_id;
        if let Some(root) = self.root.as_mut() {
            dedupe_ids(root, &mut seen, &mut next_id);
        }
        self.next_id = next_id;
    }

    fn fresh_id(&mut self) -> PaneId {
        let id = PaneId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Remove empty panes and collapse splits with one child.
    fn prune(&mut self) {
        self.root = self.root.take().and_then(prune);
    }
}

fn dedupe_ids<T>(node: &mut DockNode<T>, seen: &mut std::collections::HashSet<PaneId>, next_id: &mut u64) {
    match node {
        DockNode::Pane(pane) => {
            if pane.id.0 == u64::MAX || !seen.insert(pane.id) {
                pane.id = PaneId(*next_id);
                *next_id = next_id.saturating_add(1);
                seen.insert(pane.id);
            }
        }
        DockNode::Split { first, second, .. } => {
            dedupe_ids(first, seen, next_id);
            dedupe_ids(second, seen, next_id);
        }
    }
}

fn collect_ids<T>(node: &DockNode<T>, out: &mut Vec<PaneId>) {
    match node {
        DockNode::Pane(pane) => out.push(pane.id),
        DockNode::Split { first, second, .. } => {
            collect_ids(first, out);
            collect_ids(second, out);
        }
    }
}

fn find_pane<T>(node: &DockNode<T>, id: PaneId) -> Option<&Pane<T>> {
    match node {
        DockNode::Pane(pane) => (pane.id == id).then_some(pane),
        DockNode::Split { first, second, .. } => find_pane(first, id).or_else(|| find_pane(second, id)),
    }
}

fn find_pane_mut<T>(node: &mut DockNode<T>, id: PaneId) -> Option<&mut Pane<T>> {
    match node {
        DockNode::Pane(pane) => (pane.id == id).then_some(pane),
        DockNode::Split { first, second, .. } => match find_pane_mut(first, id) {
            Some(pane) => Some(pane),
            None => find_pane_mut(second, id),
        },
    }
}

fn find_node_mut<T>(node: &mut DockNode<T>, id: PaneId) -> Option<&mut DockNode<T>> {
    if matches!(node, DockNode::Pane(pane) if pane.id == id) {
        return Some(node);
    }
    match node {
        DockNode::Pane(_) => None,
        DockNode::Split { first, second, .. } => match find_node_mut(first, id) {
            Some(found) => Some(found),
            None => find_node_mut(second, id),
        },
    }
}

fn prune<T>(node: DockNode<T>) -> Option<DockNode<T>> {
    match node {
        DockNode::Pane(pane) if pane.tabs.is_empty() => None,
        DockNode::Pane(mut pane) => {
            pane.selected = pane.selected.min(pane.tabs.len() - 1);
            Some(DockNode::Pane(pane))
        }
        DockNode::Split {
            axis,
            state,
            first,
            second,
        } => match (prune(*first), prune(*second)) {
            (Some(first), Some(second)) => Some(DockNode::Split {
                axis,
                state,
                first: Box::new(first),
                second: Box::new(second),
            }),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        },
    }
}

/// Remove tab `index` from `pane`, keeping the selection on the same tab where it can.
fn take_tab<T>(pane: &mut Pane<T>, index: usize) -> Option<T> {
    if index >= pane.tabs.len() {
        return None;
    }
    let tab = pane.tabs.remove(index);
    if index < pane.selected {
        pane.selected -= 1;
    }
    pane.selected = pane.selected.min(pane.tabs.len().saturating_sub(1));
    Some(tab)
}

fn reorder<T>(pane: &mut Pane<T>, moved: TabMove) {
    let crate::surfaces::TabMove { from, to } = moved;
    if from >= pane.tabs.len() || to >= pane.tabs.len() || from == to {
        return;
    }
    let tab = pane.tabs.remove(from);
    pane.tabs.insert(to, tab);
    let s = pane.selected;
    pane.selected = if s == from {
        to
    } else if from < s && s <= to {
        s - 1
    } else if to <= s && s < from {
        s + 1
    } else {
        s
    };
}

/// What a [`DockArea`] needs from the app for each tab.
pub trait DockViewer<T> {
    /// The strip tab for `tab`: label, icon, dirty, closable.
    fn tab(&mut self, tab: &T) -> StripTab;

    /// Paint the body of the selected `tab`.
    fn ui(&mut self, ui: &mut Ui, tab: &mut T);
}

/// What a [`DockArea`] did this frame.
#[derive(Debug)]
pub struct DockResponse<T> {
    /// Tabs closed this frame, already removed from the tree. The app may keep or drop them.
    pub closed: Vec<T>,
    /// The tree changed shape or order this frame (close, move, split, reorder, resize).
    pub changed: bool,
}

/// Paints a [`DockTree`]: a [`TabStrip`] and body per pane, a draggable divider per split.
///
/// Drag a tab out of its strip and drop it on another pane to move it there, or near a
/// pane's edge to split that pane.
///
/// ```
/// # use fluent2_egui::surfaces::{DockArea, DockTree, DockViewer, StripTab};
/// struct Docs;
/// impl DockViewer<String> for Docs {
///     fn tab(&mut self, tab: &String) -> StripTab { StripTab::new(tab.clone()) }
///     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
/// }
/// # fn demo(ui: &mut egui::Ui, tree: &mut DockTree<String>) {
/// let response = DockArea::new("editor").show(ui, tree, &mut Docs);
/// # }
/// ```
pub struct DockArea {
    id_salt: Id,
}

/// One pane's frame report, gathered while painting.
struct PaneReport {
    pane: PaneId,
    rect: Rect,
    /// The pane's tab strip: a drop here joins the pane's tabs.
    strip: Rect,
    /// Visible tabs of the strip, (index, rect).
    tab_rects: Vec<(usize, Rect)>,
    tab_count: usize,
    closed: Option<usize>,
    moved: Option<TabMove>,
    dragging: Option<usize>,
    dropped: Option<(usize, Pos2)>,
}

struct Paint<'v, T> {
    viewer: &'v mut dyn DockViewer<T>,
    reports: Vec<PaneReport>,
    resized: bool,
    id: Id,
}

impl DockArea {
    /// A dock area. `id_salt` is the widget id.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
        }
    }

    /// Paint `tree` in the available space and apply what the user did to it.
    pub fn show<T>(
        self,
        ui: &mut Ui,
        tree: &mut DockTree<T>,
        viewer: &mut dyn DockViewer<T>,
    ) -> DockResponse<T> {
        let id = ui.make_persistent_id(self.id_salt);
        let rect = ui.available_rect_before_wrap();
        let paint = RefCell::new(Paint {
            viewer,
            reports: Vec::new(),
            resized: false,
            id,
        });
        if let Some(root) = tree.root.as_mut() {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                paint_node(ui, root, &paint);
            });
        }
        ui.allocate_rect(rect, Sense::hover());
        let Paint { reports, resized, .. } = paint.into_inner();

        // Reports were gathered while painting this frame and are applied in turn. A frame
        // carries at most one pointer interaction, so they do not interfere.
        // A tab dragged out of its strip: show where it would land.
        let theme = Theme::get(ui.ctx());
        let pointer = ui.input(|input| input.pointer.latest_pos());
        if let Some(source) = reports.iter().find(|report| report.dragging.is_some())
            && let Some(pos) = pointer
            && let Some((report, zone)) = locate(&reports, pos)
            // Inside its own strip the drag is a reorder, which the strip shows itself.
            && !(report.pane == source.pane && matches!(zone, Zone::Strip(_)))
        {
            paint_drop_preview(ui, report, zone, &theme);
        }

        let mut closed = Vec::new();
        let mut changed = resized;
        for report in &reports {
            if let Some(moved) = report.moved {
                changed |= tree.reorder_tab(report.pane, moved);
            }
        }
        for report in &reports {
            if let Some((index, pos)) = report.dropped
                && let Some(target) = drop_target(&reports, pos)
            {
                changed |= tree.move_tab(report.pane, index, target);
            }
        }
        for report in &reports {
            if let Some(index) = report.closed
                && let Some(tab) = tree.close_tab(report.pane, index)
            {
                closed.push(tab);
                changed = true;
            }
        }
        if changed {
            ui.ctx().request_repaint();
        }
        DockResponse { closed, changed }
    }
}

fn paint_node<T>(ui: &mut Ui, node: &mut DockNode<T>, paint: &RefCell<Paint<'_, T>>) {
    match node {
        DockNode::Pane(pane) => paint_pane(ui, pane, paint),
        DockNode::Split {
            axis,
            state,
            first,
            second,
        } => {
            let id = paint.borrow().id;
            // Keyed by the first pane of each child: the pair is unique per split (a pane is
            // the first of a second child under exactly one split) and survives resizes and
            // edits elsewhere in the tree.
            let split_id = id.with(("split", first_id(first), first_id(second)));
            let response = SplitView::new(split_id, *axis).label("Resize panes").show(
                ui,
                state,
                |ui| paint_node(ui, first, paint),
                |ui| paint_node(ui, second, paint),
            );
            if response.resized {
                paint.borrow_mut().resized = true;
            }
        }
    }
}

/// Id of the first pane under a node, to key its split's divider stably.
fn first_id<T>(node: &DockNode<T>) -> u64 {
    match node {
        DockNode::Pane(pane) => pane.id.0,
        DockNode::Split { first, .. } => first_id(first),
    }
}

fn paint_pane<T>(ui: &mut Ui, pane: &mut Pane<T>, paint: &RefCell<Paint<'_, T>>) {
    let rect = ui.max_rect();
    let (id, tabs) = {
        let mut paint = paint.borrow_mut();
        let tabs: Vec<StripTab> = pane.tabs.iter().map(|tab| paint.viewer.tab(tab)).collect();
        (paint.id, tabs)
    };
    let mut selected = pane.selected;
    let strip = ui
        .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            TabStrip::new(id.with(("pane", pane.id.0)), &mut selected)
                .tabs(tabs)
                .label(format!("Pane {} tabs", pane.id.0 + 1))
                .show(ui)
        })
        .inner;
    pane.selected = selected;
    let body = Rect::from_min_max(egui::pos2(rect.min.x, strip.response.rect.max.y), rect.max);
    if body.is_positive()
        && let Some(tab) = pane.tabs.get_mut(pane.selected)
    {
        let mut child = ui.new_child(UiBuilder::new().max_rect(body.shrink(spacing_horizontal::S)));
        paint.borrow_mut().viewer.ui(&mut child, tab);
    }
    paint.borrow_mut().reports.push(PaneReport {
        pane: pane.id,
        rect,
        strip: strip.response.rect,
        tab_rects: strip.tab_rects,
        tab_count: pane.tabs.len(),
        closed: strip.closed,
        moved: strip.moved,
        dragging: strip.dragging,
        dropped: strip.dropped_outside,
    });
}

/// Where in a pane a drop at `pos` lands: `None` for the centre (join the pane's tabs).
fn zone_of(rect: Rect, pos: Pos2) -> Option<DockSide> {
    if !rect.is_positive() {
        return None;
    }
    let x = (pos.x - rect.min.x) / rect.width();
    let y = (pos.y - rect.min.y) / rect.height();
    let distances = [
        (x, DockSide::Left),
        (1.0 - x, DockSide::Right),
        (y, DockSide::Top),
        (1.0 - y, DockSide::Bottom),
    ];
    distances
        .into_iter()
        .filter(|(distance, _)| distance.is_finite() && *distance < EDGE_ZONE)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, side)| side)
}

/// Where over a pane a drop lands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Zone {
    /// Over the tab strip: join the pane's tabs at this index.
    Strip(usize),
    /// Over the body: split on a side, or join at the end in the centre.
    Body(Option<DockSide>),
}

fn locate(reports: &[PaneReport], pos: Pos2) -> Option<(&PaneReport, Zone)> {
    let report = reports.iter().find(|report| report.rect.contains(pos))?;
    if report.strip.contains(pos) {
        let index = report
            .tab_rects
            .iter()
            .find(|(_, rect)| pos.x < rect.center().x)
            .map(|(index, _)| *index)
            .or_else(|| report.tab_rects.last().map(|(index, _)| index + 1))
            .unwrap_or(report.tab_count);
        return Some((report, Zone::Strip(index)));
    }
    let body = Rect::from_min_max(egui::pos2(report.rect.min.x, report.strip.max.y), report.rect.max);
    Some((report, Zone::Body(zone_of(body, pos))))
}

fn drop_target(reports: &[PaneReport], pos: Pos2) -> Option<DropTarget> {
    let (report, zone) = locate(reports, pos)?;
    let pane = report.pane;
    Some(match zone {
        Zone::Strip(index) => DropTarget::Tabs { pane, index },
        Zone::Body(Some(side)) => DropTarget::Split { pane, side },
        Zone::Body(None) => DropTarget::Tabs {
            pane,
            index: report.tab_count,
        },
    })
}

fn paint_drop_preview(ui: &Ui, report: &PaneReport, zone: Zone, theme: &Theme) {
    let pane = report.rect;
    let half_w = pane.width() / 2.0;
    let half_h = pane.height() / 2.0;
    let area = match zone {
        Zone::Strip(_) => report.strip,
        Zone::Body(None) => pane,
        Zone::Body(Some(DockSide::Left)) => Rect::from_min_size(pane.min, Vec2::new(half_w, pane.height())),
        Zone::Body(Some(DockSide::Right)) => {
            Rect::from_min_max(egui::pos2(pane.min.x + half_w, pane.min.y), pane.max)
        }
        Zone::Body(Some(DockSide::Top)) => Rect::from_min_size(pane.min, Vec2::new(pane.width(), half_h)),
        Zone::Body(Some(DockSide::Bottom)) => {
            Rect::from_min_max(egui::pos2(pane.min.x, pane.min.y + half_h), pane.max)
        }
    };
    let radius = CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        ui.scope_id().with("dock_drop_preview"),
    ));
    painter.add(RectShape::new(
        area.shrink(stroke_width::THICK),
        radius,
        c(theme.color.subtle_background_selected).gamma_multiply(0.6),
        Stroke::new(stroke_width::THICK, c(theme.color.compound_brand_stroke)),
        StrokeKind::Inside,
    ));
}

/// Edge a [`SideDock`] sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockEdge {
    /// Left of the content.
    Left,
    /// Right of the content.
    Right,
    /// Below the content.
    Bottom,
}

/// State of a [`SideDock`]: whether it is open and how large, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SideDockState {
    /// Whether the dock shows.
    pub open: bool,
    /// Width (left, right) or height (bottom) of the dock.
    pub size: f32,
}

impl Default for SideDockState {
    fn default() -> Self {
        Self {
            open: true,
            size: 240.0,
        }
    }
}

/// What a [`SideDock`] did this frame.
#[derive(Debug)]
pub struct SideDockResponse<A, B> {
    /// The dock body's result, `None` when the dock is closed.
    pub dock: Option<A>,
    /// The wrapped content's result.
    pub content: B,
    /// The user closed the dock this frame.
    pub closed: bool,
}

/// A collapsible panel on one edge of some content, with a titled header, a close button and
/// a draggable divider. The app reopens it by setting [`SideDockState::open`].
///
/// ```
/// # use fluent2_egui::surfaces::{DockEdge, SideDock, SideDockState};
/// # fn demo(ui: &mut egui::Ui, state: &mut SideDockState) {
/// SideDock::new("explorer", DockEdge::Left, "Explorer").show(
///     ui,
///     state,
///     |ui| ui.label("files"),
///     |ui| ui.label("editor"),
/// );
/// # }
/// ```
pub struct SideDock {
    id_salt: Id,
    edge: DockEdge,
    title: String,
}

impl SideDock {
    /// A dock on `edge` titled `title`. `id_salt` is the widget id.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        edge: DockEdge,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            edge,
            title: title.into(),
        }
    }

    /// Paint the dock (when open) beside `content`.
    pub fn show<A, B>(
        self,
        ui: &mut Ui,
        state: &mut SideDockState,
        dock: impl FnOnce(&mut Ui) -> A,
        content: impl FnOnce(&mut Ui) -> B,
    ) -> SideDockResponse<A, B> {
        if !state.open {
            let content = content(ui);
            return SideDockResponse {
                dock: None,
                content,
                closed: false,
            };
        }
        let size = if state.size.is_finite() && state.size > 0.0 {
            state.size
        } else {
            SideDockState::default().size
        };
        let (axis, mut split) = match self.edge {
            DockEdge::Left => (SplitAxis::Horizontal, SplitSize::FixedFirst(size)),
            DockEdge::Right => (SplitAxis::Horizontal, SplitSize::FixedSecond(size)),
            DockEdge::Bottom => (SplitAxis::Vertical, SplitSize::FixedSecond(size)),
        };
        let mut split_state = SplitState { size: split };
        let view = SplitView::new(self.id_salt, axis).label(format!("Resize {}", self.title));
        let id = self.id_salt;
        let title = self.title;
        let panel = |ui: &mut Ui| dock_panel(ui, id, &title, dock);
        let ((dock_out, closed), content) = if self.edge == DockEdge::Left {
            let out = view.show(ui, &mut split_state, panel, content);
            (out.first, out.second)
        } else {
            let out = view.show(ui, &mut split_state, content, panel);
            (out.second, out.first)
        };
        split = split_state.size;
        state.size = match split {
            SplitSize::FixedFirst(v) | SplitSize::FixedSecond(v) => v,
            SplitSize::Ratio(_) => size,
        };
        if closed {
            state.open = false;
        }
        SideDockResponse {
            dock: Some(dock_out),
            content,
            closed,
        }
    }
}

/// The dock's header (title and close button) and body.
fn dock_panel<A>(ui: &mut Ui, id: Id, title: &str, body: impl FnOnce(&mut Ui) -> A) -> (A, bool) {
    let theme = Theme::get(ui.ctx());
    crate::controls::button::ensure_type_ramp(ui);
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, c(theme.color.neutral_background_2));
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), HEADER_HEIGHT));
    let mut closed = false;
    ui.scope_builder(
        UiBuilder::new().max_rect(header.shrink2(Vec2::new(spacing_horizontal::M, 0.0))),
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    text::rich(title.to_owned(), TypeRamp::Body1Strong)
                        .color(c(theme.color.neutral_foreground_1)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.add(
                        crate::controls::Button::icon_only(
                            fluent2_icons::Icon::Dismiss,
                            format!("Close {title}"),
                        )
                        .appearance(crate::controls::Appearance::Subtle)
                        .id_salt(id.with("close")),
                    );
                    closed = close.clicked();
                });
            });
        },
    );
    let body_rect = Rect::from_min_max(egui::pos2(rect.min.x, header.max.y), rect.max);
    let mut child = ui.new_child(UiBuilder::new().max_rect(body_rect.shrink(spacing_horizontal::S)));
    (body(&mut child), closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split, or a dangling id the following asserts catch.
    fn split(tree: &mut DockTree<&'static str>, pane: PaneId, side: DockSide, tab: &'static str) -> PaneId {
        tree.split(pane, side, tab).unwrap_or(PaneId(u64::MAX))
    }

    fn selected_tab(tree: &DockTree<&'static str>, id: PaneId) -> Option<&'static str> {
        tree.pane(id)
            .and_then(|pane| pane.tabs.get(pane.selected).copied())
    }

    fn tabs(tree: &DockTree<&'static str>) -> Vec<Vec<&'static str>> {
        tree.pane_ids()
            .into_iter()
            .filter_map(|id| tree.pane(id).map(|pane| pane.tabs.clone()))
            .collect()
    }

    #[test]
    fn split_puts_the_new_pane_on_the_side() {
        let mut tree = DockTree::new(vec!["a"]);
        let first = tree.pane_ids()[0];
        let left = split(&mut tree, first, DockSide::Left, "l");
        let bottom = split(&mut tree, first, DockSide::Bottom, "b");
        assert_eq!(tree.pane_ids(), [left, first, bottom]);
        assert_eq!(tabs(&tree), [vec!["l"], vec!["a"], vec!["b"]]);
        let shape = match tree.root() {
            Some(DockNode::Split { axis, second, .. }) => match &**second {
                DockNode::Split { axis: inner, .. } => Some((*axis, *inner)),
                DockNode::Pane(_) => None,
            },
            _ => None,
        };
        assert_eq!(shape, Some((SplitAxis::Horizontal, SplitAxis::Vertical)));
        assert_eq!(tree.split(PaneId(99), DockSide::Top, "x"), Err("x"));
    }

    #[test]
    fn closing_the_last_tab_collapses_the_pane() {
        let mut tree = DockTree::new(vec!["a", "b"]);
        let first = tree.pane_ids()[0];
        let right = split(&mut tree, first, DockSide::Right, "r");
        assert_eq!(tree.close_tab(right, 0), Some("r"));
        assert_eq!(tree.pane_ids(), [first]);
        assert!(
            matches!(tree.root(), Some(DockNode::Pane(_))),
            "the split collapsed"
        );
        assert_eq!(tree.close_tab(first, 5), None);
        tree.close_tab(first, 0);
        tree.close_tab(first, 0);
        assert!(tree.is_empty());
        let again = tree.add_tab(None, "new");
        assert_eq!(tabs(&tree), [vec!["new"]]);
        assert_ne!(again, first, "pane ids are not reused");
    }

    #[test]
    fn close_keeps_the_selection_on_the_same_tab() {
        let mut tree = DockTree::new(vec!["a", "b", "c"]);
        let id = tree.pane_ids()[0];
        if let Some(pane) = tree.pane_mut(id) {
            pane.selected = 2;
        }
        tree.close_tab(id, 0);
        assert_eq!(selected_tab(&tree, id), Some("c"));
    }

    #[test]
    fn move_between_panes_and_collapse_the_source() {
        let mut tree = DockTree::new(vec!["a", "b"]);
        let first = tree.pane_ids()[0];
        let right = split(&mut tree, first, DockSide::Right, "r");
        assert!(tree.move_tab(
            right,
            0,
            DropTarget::Tabs {
                pane: first,
                index: 1
            }
        ));
        assert_eq!(tabs(&tree), [vec!["a", "r", "b"]]);
        assert_eq!(selected_tab(&tree, first), Some("r"), "the moved tab is selected");
    }

    #[test]
    fn move_into_a_split() {
        let mut tree = DockTree::new(vec!["a", "b"]);
        let first = tree.pane_ids()[0];
        assert!(tree.move_tab(
            first,
            1,
            DropTarget::Split {
                pane: first,
                side: DockSide::Top
            }
        ));
        assert_eq!(tabs(&tree), [vec!["b"], vec!["a"]]);
        // The only tab cannot split its own pane.
        let top = tree.pane_ids()[0];
        assert!(!tree.move_tab(
            top,
            0,
            DropTarget::Split {
                pane: top,
                side: DockSide::Left
            }
        ));
        // But it can split another pane, and its own pane then disappears.
        assert!(tree.move_tab(
            top,
            0,
            DropTarget::Split {
                pane: first,
                side: DockSide::Right
            }
        ));
        assert_eq!(tabs(&tree), [vec!["a"], vec!["b"]]);
        assert_eq!(tree.pane_ids().len(), 2);
    }

    #[test]
    fn move_within_a_pane_reorders() {
        let mut tree = DockTree::new(vec!["a", "b", "c"]);
        let id = tree.pane_ids()[0];
        assert!(tree.move_tab(id, 0, DropTarget::Tabs { pane: id, index: 9 }));
        assert_eq!(tabs(&tree), [vec!["b", "c", "a"]]);
        assert!(!tree.move_tab(id, 7, DropTarget::Tabs { pane: id, index: 0 }));
        assert!(!tree.move_tab(PaneId(42), 0, DropTarget::Tabs { pane: id, index: 0 }));
        assert!(!tree.move_tab(
            id,
            0,
            DropTarget::Tabs {
                pane: PaneId(42),
                index: 0
            }
        ));
        assert_eq!(tabs(&tree), [vec!["b", "c", "a"]], "failed moves change nothing");
    }

    #[test]
    fn zones_split_near_edges_and_join_in_the_middle() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0));
        assert_eq!(zone_of(rect, egui::pos2(200.0, 100.0)), None);
        assert_eq!(zone_of(rect, egui::pos2(10.0, 100.0)), Some(DockSide::Left));
        assert_eq!(zone_of(rect, egui::pos2(390.0, 100.0)), Some(DockSide::Right));
        assert_eq!(zone_of(rect, egui::pos2(200.0, 10.0)), Some(DockSide::Top));
        assert_eq!(zone_of(rect, egui::pos2(200.0, 190.0)), Some(DockSide::Bottom));
        assert_eq!(zone_of(Rect::NOTHING, egui::pos2(1.0, 1.0)), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let mut tree = DockTree::new(vec!["a".to_owned(), "b".to_owned()]);
        let first = tree.pane_ids()[0];
        let right = tree.split(first, DockSide::Right, "r".to_owned()).ok();
        assert!(right.is_some());
        let json = serde_json::to_string(&tree).unwrap_or_default();
        let back: Option<DockTree<String>> = serde_json::from_str(&json).ok();
        assert_eq!(back.as_ref(), Some(&tree));
        // Ids keep counting after a restore.
        let mut back = back.unwrap_or_default();
        let next = right.and_then(|right| back.split(right, DockSide::Bottom, "n".to_owned()).ok());
        assert!(next > right);
    }

    #[test]
    fn locate_treats_the_strip_as_join_and_measures_edges_over_the_body() {
        let report = PaneReport {
            pane: PaneId(7),
            rect: Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 232.0)),
            strip: Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 32.0)),
            tab_rects: vec![
                (
                    0,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 32.0)),
                ),
                (
                    1,
                    Rect::from_min_max(egui::pos2(100.0, 0.0), egui::pos2(200.0, 32.0)),
                ),
            ],
            tab_count: 2,
            closed: None,
            moved: None,
            dragging: None,
            dropped: None,
        };
        let reports = [report];
        let at = |x: f32, y: f32| drop_target(&reports, egui::pos2(x, y));
        let pane = PaneId(7);
        assert_eq!(at(20.0, 16.0), Some(DropTarget::Tabs { pane, index: 0 }));
        assert_eq!(at(160.0, 16.0), Some(DropTarget::Tabs { pane, index: 2 }));
        assert_eq!(at(350.0, 16.0), Some(DropTarget::Tabs { pane, index: 2 }));
        // Just under the strip is the body's top edge zone.
        assert_eq!(
            at(200.0, 40.0),
            Some(DropTarget::Split {
                pane,
                side: DockSide::Top
            })
        );
        assert_eq!(at(200.0, 132.0), Some(DropTarget::Tabs { pane, index: 2 }));
        assert_eq!(at(500.0, 16.0), None);
    }

    #[test]
    fn normalize_prunes_dedupes_and_advances_the_counter() {
        let mut tree = DockTree {
            root: Some(DockNode::Split {
                axis: SplitAxis::Horizontal,
                state: SplitState::default(),
                first: Box::new(DockNode::Pane(Pane {
                    id: PaneId(3),
                    tabs: vec!["a"],
                    selected: 9,
                })),
                second: Box::new(DockNode::Split {
                    axis: SplitAxis::Vertical,
                    state: SplitState::default(),
                    first: Box::new(DockNode::Pane(Pane {
                        id: PaneId(3),
                        tabs: vec!["b"],
                        selected: 0,
                    })),
                    second: Box::new(DockNode::Pane(Pane {
                        id: PaneId(1),
                        tabs: Vec::new(),
                        selected: 0,
                    })),
                }),
            }),
            next_id: 0,
        };
        tree.normalize();
        assert_eq!(tree.pane_ids(), [PaneId(3), PaneId(4)]);
        assert_eq!(tree.pane(PaneId(3)).map(|pane| pane.selected), Some(0));
        let new = tree.split(PaneId(4), DockSide::Right, "c");
        assert_eq!(new, Ok(PaneId(5)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialising_normalizes_a_bad_layout() {
        let json = r#"{"root":{"Split":{"axis":"Horizontal","state":{"size":{"Ratio":0.5}},
            "first":{"Pane":{"id":2,"tabs":["a"],"selected":0}},
            "second":{"Split":{"axis":"Vertical","state":{"size":{"Ratio":0.5}},
                "first":{"Pane":{"id":2,"tabs":["b"],"selected":0}},
                "second":{"Pane":{"id":0,"tabs":[],"selected":0}}}}}},"next_id":1}"#;
        let mut tree: DockTree<String> = serde_json::from_str(json).unwrap_or_default();
        assert_eq!(tree.pane_ids(), [PaneId(2), PaneId(3)]);
        assert_eq!(
            tree.split(PaneId(2), DockSide::Top, "c".to_owned()),
            Ok(PaneId(4))
        );
    }
}
