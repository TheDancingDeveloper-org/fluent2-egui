//! TreeView: a virtualised tree with lazy children (`docs/specs/data-views.md` §5).
//!
//! The app answers "what are the children of this key" for expanded nodes only. Each frame the
//! tree flattens those answers into rows with a depth, then paints only the rows in view. The
//! selection and roving focus follow the rules of the DataGrid (§3), keyed by the app's `K` rather
//! than by [`RowId`](crate::data::RowId).
//!
//! A node can carry a checkbox ([`TreeNode::checked`]) and the tree can show trailing cell
//! columns ([`TreeView::columns`]). The tree reports a toggle; the app owns the check states and
//! their tri-state propagation to parents.

use std::collections::BTreeSet;
use std::f32::consts::FRAC_PI_2;
use std::marker::PhantomData;

use egui::accesskit::Role;
use egui::emath::Rot2;
use egui::{
    Align, Align2, Color32, CornerRadius, Event, EventFilter, Id, Key, Layout, Modifiers, Pos2, Rect,
    ScrollArea, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetText,
};
use egui::{WidgetInfo, WidgetType};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::data_grid::SelectionMode;
use crate::controls::Spinner;
use crate::controls::checkbox::{CheckState, paint_box};
use crate::data::rows::visible_rows;
use crate::foundation::focus::paint_focus_ring;
use crate::foundation::icon::{Icon, IconName};
use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{DataMetrics, Theme, c};

/// Deepest level a node can be expanded to. Nodes at this depth are shown but not expanded (rule 9).
const MAX_DEPTH: usize = 64;
/// Width of the chevron slot, 24 pt (§5.1). The slot is the `spacing_horizontal::XXL` value.
const CHEVRON_SLOT: f32 = spacing_horizontal::XXL;
/// Side of the checkbox square: the 16 pt medium Fluent checkbox.
const CHECK_BOX: f32 = 16.0;

/// Whether a node has children, and whether they are still arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Children {
    /// A leaf: no chevron, and the app is not asked for its children.
    None,
    /// Has children. The app is asked for them when the node is expanded.
    Some,
    /// Expanded but the app is still loading. The tree shows a spinner row under the node until
    /// the app returns children for it.
    Loading,
}

/// One child returned by the app.
///
/// ```
/// use fluent2_egui::data::tree_view::{Children, TreeNode};
/// let node = TreeNode::new(7_u64, "Documents").children(Children::Some);
/// assert_eq!(node.key, 7);
/// ```
pub struct TreeNode<K> {
    /// The app's key for this node. Keys must be unique among siblings. A key seen twice in one
    /// frame is skipped the second time (D10).
    pub key: K,
    /// The text of the row.
    pub label: WidgetText,
    /// An optional icon drawn before the label.
    pub icon: Option<Icon>,
    /// Whether the node has children.
    pub children: Children,
    /// The node's checkbox. `None` draws no checkbox.
    pub checked: Option<CheckState>,
}

impl<K> TreeNode<K> {
    /// A leaf node with `label`. Use [`TreeNode::children`] to make it expandable.
    #[must_use]
    pub fn new(key: K, label: impl Into<WidgetText>) -> Self {
        Self {
            key,
            label: label.into(),
            icon: None,
            children: Children::None,
            checked: None,
        }
    }

    /// Draw a checkbox in `state` before the icon. Use [`CheckState::Mixed`] on a parent whose
    /// children are partly checked. A click or Space reports
    /// [`TreeViewResponse::check_toggled`]; the app sets the new states.
    #[must_use]
    pub fn checked(mut self, state: CheckState) -> Self {
        self.checked = Some(state);
        self
    }

    /// Set whether the node has children.
    #[must_use]
    pub fn children(mut self, children: Children) -> Self {
        self.children = children;
        self
    }

    /// Set the icon drawn before the label.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// The state of a TreeView. Store it in the app and pass `&mut` to every `show`.
///
/// ```
/// use fluent2_egui::data::tree_view::TreeState;
/// let state = TreeState::<u64>::default();
/// assert!(state.expanded.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeState<K: Ord> {
    /// Expanded nodes.
    pub expanded: BTreeSet<K>,
    /// Selected nodes.
    pub selection: BTreeSet<K>,
    /// Anchor for Shift ranges.
    pub anchor: Option<K>,
    /// Focused node. `None` before first focus.
    pub focus: Option<K>,
}

impl<K: Ord> Default for TreeState<K> {
    fn default() -> Self {
        Self {
            expanded: BTreeSet::new(),
            selection: BTreeSet::new(),
            anchor: None,
            focus: None,
        }
    }
}

/// Row height class of a TreeView (§10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TreeSize {
    /// 24 pt rows.
    Small,
    /// 32 pt rows.
    #[default]
    Medium,
}

impl TreeSize {
    fn row_height(self, metrics: &DataMetrics) -> f32 {
        match self {
            Self::Small => metrics.tree_row_height_small,
            Self::Medium => metrics.tree_row_height_medium,
        }
    }
}

/// A trailing cell column of a TreeView: a header title and a fixed width.
///
/// ```
/// use fluent2_egui::data::tree_view::TreeColumn;
/// let size = TreeColumn::new("Size", 80.0);
/// assert_eq!(size.width, 80.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TreeColumn {
    /// Header text.
    pub title: String,
    /// Width in points.
    pub width: f32,
}

impl TreeColumn {
    /// A column titled `title`, `width` points wide.
    #[must_use]
    pub fn new(title: impl Into<String>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
        }
    }

    /// The width, finite and not negative.
    fn width(&self) -> f32 {
        if self.width.is_finite() {
            self.width.max(0.0)
        } else {
            0.0
        }
    }
}

/// Draws one cell: `(ui, key, column index)`.
type CellFn<'a, K> = Box<dyn FnMut(&mut Ui, &K, usize) + 'a>;

/// What a TreeView reports after `show`.
#[derive(Clone, Debug)]
pub struct TreeViewResponse<K> {
    /// The whole tree. `has_focus` while the tree takes the keyboard.
    pub response: egui::Response,
    /// The selection changed this frame.
    pub selection_changed: bool,
    /// A node was expanded this frame. An app with lazy data starts loading here. When several
    /// nodes expand at once (`*`), this is the last one; diff `TreeState::expanded` for the rest.
    pub expanded: Option<K>,
    /// A node was collapsed this frame.
    pub collapsed: Option<K>,
    /// Enter on the focused node.
    pub activated: Option<K>,
    /// Secondary click or Shift+F10 on a node. Attach a context menu to this.
    pub context_menu: Option<K>,
    /// The checkbox of this node was clicked, or Space was pressed on it. The app sets the new
    /// state and propagates it to children and parents.
    pub check_toggled: Option<K>,
}

/// A virtualised tree. Build it, then call [`TreeView::show`].
///
/// ```
/// # use fluent2_egui::data::SelectionMode;
/// # use fluent2_egui::data::tree_view::{Children, TreeNode, TreeState, TreeView};
/// # fn demo(ui: &mut egui::Ui) {
/// let mut state = TreeState::<u64>::default();
/// let _response = TreeView::new("files")
///     .label("Files")
///     .selection_mode(SelectionMode::Multi)
///     .show(ui, &mut state, |parent, out| {
///         // Roots when `parent` is `None`; the children of `parent` otherwise.
///         if parent.is_none() {
///             out.push(TreeNode::new(1, "Documents").children(Children::Some));
///         }
///     });
/// # }
/// ```
pub struct TreeView<'a, K> {
    id: Id,
    selection_mode: SelectionMode,
    size: TreeSize,
    label: Option<String>,
    columns: &'a [TreeColumn],
    cell: Option<CellFn<'a, K>>,
    marker: PhantomData<&'a K>,
}

impl<'a, K: Clone + Ord + std::hash::Hash> TreeView<'a, K> {
    /// A tree. `id_salt` keeps the tree's id stable across frames.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id: Id::new(hashed(&id_salt)),
            selection_mode: SelectionMode::Single,
            size: TreeSize::Medium,
            label: None,
            columns: &[],
            cell: None,
            marker: PhantomData,
        }
    }

    /// Which nodes can be selected. The default is [`SelectionMode::Single`].
    #[must_use]
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Row height class. The default is [`TreeSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: TreeSize) -> Self {
        self.size = size;
        self
    }

    /// Accessible name of the tree.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Trailing cell columns after the label, with a header row above the tree. `cell(ui, key,
    /// column)` draws one cell, for visible rows only, in a left-to-right `ui` the column's width.
    ///
    /// ```
    /// # use fluent2_egui::data::tree_view::{TreeColumn, TreeNode, TreeState, TreeView};
    /// # fn demo(ui: &mut egui::Ui, state: &mut TreeState<u64>) {
    /// let columns = [TreeColumn::new("Size", 80.0)];
    /// TreeView::new("files")
    ///     .columns(&columns, |ui, key, _column| {
    ///         ui.label(format!("{key} KB"));
    ///     })
    ///     .show(ui, state, |parent, out| {
    ///         if parent.is_none() {
    ///             out.push(TreeNode::new(1, "a.txt"));
    ///         }
    ///     });
    /// # }
    /// ```
    #[must_use]
    pub fn columns(mut self, columns: &'a [TreeColumn], cell: impl FnMut(&mut Ui, &K, usize) + 'a) -> Self {
        self.columns = columns;
        self.cell = Some(Box::new(cell));
        self
    }

    /// Draw the tree.
    ///
    /// `children(None, out)` fills the roots. It is called once per expanded node per frame with
    /// `Some(&key)`, and never for a collapsed node. Selection changes at once. An expansion shows
    /// its rows from the next frame, so the children are first asked for on that frame.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut TreeState<K>,
        mut children: impl FnMut(Option<&K>, &mut Vec<TreeNode<K>>),
    ) -> TreeViewResponse<K> {
        let theme = Theme::get(ui.ctx());
        let requested = self.size.row_height(&theme.data);
        let row_height = if requested.is_finite() {
            requested.max(1.0)
        } else {
            1.0
        };
        let fill_hover = c(theme.color.subtle_background_hover);
        let fill_pressed = c(theme.color.subtle_background_pressed);
        let fill_selected = c(theme.color.subtle_background_selected);
        let text_hover = c(theme.color.neutral_foreground_2_hover);
        let text_idle = c(theme.color.neutral_foreground_2);
        let text_selected = c(theme.color.neutral_foreground_1_selected);
        let icon_color = c(theme.color.neutral_foreground_3);
        let ring_radius = CornerRadius::same(border_radius::MEDIUM as u8);

        let mut rows = Vec::new();
        flatten(
            state,
            &mut children,
            None,
            None,
            0,
            &mut rows,
            &mut BTreeSet::new(),
        );

        let id = self.id;
        let mode = self.selection_mode;
        let label = self.label;
        let columns = self.columns;
        let mut cell = self.cell;
        let cells_width: f32 = columns.iter().map(TreeColumn::width).sum();
        if !columns.is_empty() {
            paint_header(
                ui,
                columns,
                row_height,
                cells_width,
                c(theme.color.neutral_foreground_3),
            );
        }
        let enabled = ui.is_enabled();
        let before = state.selection.clone();
        let output = ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let (content, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), rows.len() as f32 * row_height),
                    Sense::hover(),
                );
                let origin = content.min;
                let width = content.width();
                let clip = ui.clip_rect();
                let tree_response = ui.interact(clip, id, Sense::click());
                let visible = visible_rows(viewport, 0.0, row_height, rows.len());
                let page = visible.len().saturating_sub(1).max(1);
                let focused = tree_response.has_focus();
                let mut report = Reports::default();

                if focused {
                    // Arrow keys move focus in the tree, not to a neighbouring widget (§5.2).
                    let lock = EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    };
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(id, lock));
                    for (key, modifiers) in pressed_keys(ui) {
                        handle_key(state, &rows, mode, key, modifiers, page, &mut report);
                    }
                    for _ in 0..typed_stars(ui) {
                        let current = focus_index(state, &rows);
                        expand_siblings(state, &rows, current, &mut report);
                    }
                }

                let focus_row = focus_index(state, &rows);
                // The focused row, for the tree's active descendant: arrow keys move focus
                // between rows the tree owns, and without this screen readers say nothing.
                let mut active_row = None;
                if report.moved
                    && let Some(index) = focus_row
                {
                    // Keyboard moves bring the focused row into view (§5.2).
                    let top = origin.y + index as f32 * row_height;
                    ui.scroll_to_rect(
                        Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, row_height)),
                        None,
                    );
                }
                let mut clicked = false;
                for index in visible.clone() {
                    let Some(row) = rows.get(index) else { continue };
                    let top = origin.y + index as f32 * row_height;
                    let row_rect =
                        Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, row_height));
                    let visible_rect = row_rect.intersect(clip);
                    let slot = Rect::from_min_size(
                        Pos2::new(
                            origin.x + spacing_horizontal::S + row.depth as f32 * spacing_horizontal::XXL,
                            top,
                        ),
                        Vec2::new(CHEVRON_SLOT, row_height),
                    );
                    let Some(key) = row.key.as_ref() else {
                        // The spinner row under a loading node: a busy TreeItem, not focusable.
                        let hit = ui.interact(visible_rect, id.with(("loading", index)), Sense::hover());
                        hit.widget_info(|| WidgetInfo::labeled(WidgetType::Other, enabled, "Loading"));
                        ui.ctx().accesskit_node_builder(hit.id, |node| {
                            node.set_role(Role::TreeItem);
                            node.set_label("Loading");
                            node.set_busy();
                            node.set_level(row.depth + 1);
                        });
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(Rect::from_min_max(
                                    Pos2::new(slot.max.x + spacing_horizontal::XS, top),
                                    Pos2::new(row_rect.max.x, row_rect.max.y),
                                ))
                                .layout(Layout::left_to_right(Align::Center)),
                        );
                        child.set_clip_rect(visible_rect);
                        child.add(Spinner::new());
                        continue;
                    };

                    let hit = ui.interact(visible_rect, id.with(("row", hashed(key))), Sense::CLICK);
                    if focus_row == Some(index) {
                        active_row = Some(hit.id.accesskit_id());
                    }
                    let text = row.label.text();
                    hit.widget_info(|| WidgetInfo::labeled(WidgetType::Other, enabled, text));
                    let chevron = row.has_children.then(|| {
                        ui.interact(
                            slot.intersect(clip),
                            id.with(("chevron", hashed(key))),
                            Sense::CLICK,
                        )
                    });
                    let check_slot =
                        Rect::from_min_size(Pos2::new(slot.max.x, top), Vec2::new(CHEVRON_SLOT, row_height));
                    let check = row.checked.map(|state| {
                        let hit = ui.interact(
                            check_slot.intersect(clip),
                            id.with(("check", hashed(key))),
                            Sense::CLICK,
                        );
                        (state, hit)
                    });
                    if let Some((_, check)) = &check
                        && check.clicked()
                    {
                        clicked = true;
                        state.focus = Some(key.clone());
                        report.check_toggled = Some(key.clone());
                    }
                    if hit.clicked() {
                        clicked = true;
                        let modifiers = ui.input(|input| input.modifiers);
                        report.selection_changed |= click(state, &rows, mode, index, modifiers);
                    }
                    if hit.secondary_clicked() {
                        clicked = true;
                        report.context_menu = Some(key.clone());
                    }
                    if let Some(chevron) = &chevron
                        && chevron.clicked()
                    {
                        clicked = true;
                        toggle_at(state, &rows, index, &mut report);
                    }

                    let selected = state.selection.contains(key);
                    let fill = if selected {
                        Some(fill_selected)
                    } else if hit.is_pointer_button_down_on() {
                        Some(fill_pressed)
                    } else if hit.hovered() {
                        Some(fill_hover)
                    } else {
                        None
                    };
                    if let Some(fill) = fill {
                        ui.painter().rect_filled(row_rect, ring_radius, fill);
                    }

                    // High contrast fills a selected row with the highlight colour; its text,
                    // chevron and icon take the matching foreground.
                    let color = if selected {
                        text_selected
                    } else if hit.hovered() {
                        text_hover
                    } else {
                        text_idle
                    };
                    ui.ctx().accesskit_node_builder(hit.id, |node| {
                        node.set_role(Role::TreeItem);
                        node.set_label(text);
                        node.set_level(row.depth + 1);
                        if row.has_children {
                            node.set_expanded(row.expanded);
                        }
                        if mode != SelectionMode::None {
                            node.set_selected(selected);
                        }
                        node.set_position_in_set(row.set_index + 1);
                        node.set_size_of_set(row.set_size);
                        if let Some(checked) = row.checked {
                            node.set_toggled(toggled(checked));
                        }
                    });
                    if let Some((checked, check)) = &check {
                        let check_label = format!("Check {text}");
                        check.widget_info(|| {
                            let mut info =
                                WidgetInfo::labeled(WidgetType::Checkbox, enabled, check_label.as_str());
                            info.selected = Some(*checked == CheckState::Checked);
                            info
                        });
                        ui.ctx().accesskit_node_builder(check.id, |node| {
                            node.set_role(Role::CheckBox);
                            node.set_label(check_label.as_str());
                            node.set_toggled(toggled(*checked));
                        });
                        let square = Rect::from_center_size(check_slot.center(), Vec2::splat(CHECK_BOX));
                        if visible_rect.intersects(square) {
                            paint_box(
                                ui,
                                square,
                                *checked,
                                &theme,
                                enabled,
                                check.hovered(),
                                check.is_pointer_button_down_on(),
                            );
                        }
                    }
                    if let Some(chevron) = &chevron {
                        let name = if row.expanded { "Collapse" } else { "Expand" };
                        let chevron_label = format!("{name} {text}");
                        chevron.widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Button, enabled, chevron_label.as_str())
                        });
                        ui.ctx().accesskit_node_builder(chevron.id, |node| {
                            node.set_role(Role::Button);
                            node.set_label(chevron_label.as_str());
                        });
                        let turn = motion::animate(
                            ui.ctx(),
                            id.with(("turn", hashed(key))),
                            if row.expanded { 1.0 } else { 0.0 },
                            Duration::Fast,
                            Curve::DecelerateMid,
                        );
                        paint_chevron(ui, slot, turn, color);
                    }

                    let lead = if row.checked.is_some() {
                        check_slot.max.x
                    } else {
                        slot.max.x
                    };
                    let text_x = lead + spacing_horizontal::XS;
                    let mut x = text_x;
                    let cells_x = (row_rect.max.x - cells_width).max(text_x);
                    let label_clip = visible_rect.intersect(Rect::from_min_max(
                        row_rect.min,
                        Pos2::new(cells_x - spacing_horizontal::XS, row_rect.max.y),
                    ));
                    let mut child = ui.new_child(
                        UiBuilder::new()
                            .max_rect(Rect::from_min_max(
                                Pos2::new(text_x, top),
                                Pos2::new(row_rect.max.x, row_rect.max.y),
                            ))
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    child.set_clip_rect(label_clip);
                    if let Some(name) = row.icon {
                        let icon =
                            child.add(Icon::new(name).tint(Some(if selected { color } else { icon_color })));
                        x = icon.rect.max.x + spacing_horizontal::XS;
                    }
                    ui.painter_at(label_clip).text(
                        Pos2::new(x, row_rect.center().y),
                        Align2::LEFT_CENTER,
                        text,
                        font_id(TypeRamp::Body1),
                        color,
                    );
                    if let Some(cell) = cell.as_mut() {
                        let mut left = cells_x;
                        for (column, spec) in columns.iter().enumerate() {
                            let cell_rect = Rect::from_min_size(
                                Pos2::new(left, top),
                                Vec2::new(spec.width(), row_height),
                            );
                            left = cell_rect.max.x;
                            let mut cell_ui = ui.new_child(
                                UiBuilder::new()
                                    .id_salt(("cell", hashed(key), column))
                                    .max_rect(cell_rect.shrink2(Vec2::new(spacing_horizontal::XS, 0.0)))
                                    .layout(Layout::left_to_right(Align::Center)),
                            );
                            cell_ui.set_clip_rect(visible_rect.intersect(cell_rect));
                            cell(&mut cell_ui, key, column);
                        }
                    }
                }

                if focused && let Some(index) = focus_row {
                    let top = origin.y + index as f32 * row_height;
                    let rect = Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, row_height));
                    paint_focus_ring(ui, rect.shrink(2.0), ring_radius);
                }
                if clicked {
                    tree_response.request_focus();
                }

                tree_response.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Other, enabled, label.as_deref().unwrap_or_default())
                });
                ui.ctx().accesskit_node_builder(tree_response.id, |node| {
                    node.set_role(Role::Tree);
                    if let Some(row) = active_row {
                        node.set_active_descendant(row);
                    }
                    if let Some(label) = &label {
                        node.set_label(label.as_str());
                    }
                    if mode == SelectionMode::Multi {
                        node.set_multiselectable();
                    }
                });

                if report.expanded.is_some() || report.collapsed.is_some() {
                    // The rows for the new expansion are built on the next frame.
                    ui.ctx().request_repaint();
                }
                (tree_response, report)
            });

        let (response, report) = output.inner;
        TreeViewResponse {
            response,
            selection_changed: state.selection != before || report.selection_changed,
            expanded: report.expanded,
            collapsed: report.collapsed,
            activated: report.activated,
            context_menu: report.context_menu,
            check_toggled: report.check_toggled,
        }
    }
}

/// The AccessKit tristate of a check state: screen readers say "checked", "not checked" or
/// "partially checked".
fn toggled(state: CheckState) -> egui::accesskit::Toggled {
    match state {
        CheckState::Checked => egui::accesskit::Toggled::True,
        CheckState::Unchecked => egui::accesskit::Toggled::False,
        CheckState::Mixed => egui::accesskit::Toggled::Mixed,
    }
}

/// The header row above a tree with columns: "Name" over the labels, then each column's title.
fn paint_header(ui: &mut Ui, columns: &[TreeColumn], height: f32, cells_width: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let font = font_id(TypeRamp::Caption1);
    let mut left = (rect.max.x - cells_width).max(rect.min.x);
    painter.text(
        Pos2::new(rect.min.x + spacing_horizontal::S, rect.center().y),
        Align2::LEFT_CENTER,
        "Name",
        font.clone(),
        color,
    );
    for column in columns {
        painter.text(
            Pos2::new(left + spacing_horizontal::XS, rect.center().y),
            Align2::LEFT_CENTER,
            column.title.as_str(),
            font.clone(),
            color,
        );
        left += column.width();
    }
    painter.hline(
        rect.x_range(),
        rect.max.y - stroke_width::THIN / 2.0,
        Stroke::new(stroke_width::THIN, color.gamma_multiply(0.4)),
    );
}

/// One row of the flattened tree for one frame.
struct Row<K> {
    /// The node's key. `None` for the spinner row under a loading node, which takes no focus.
    key: Option<K>,
    /// Indentation level, from 0 for roots and at most [`MAX_DEPTH`].
    depth: usize,
    /// Row index of the parent.
    parent: Option<usize>,
    has_children: bool,
    expanded: bool,
    /// Position among the siblings, from 0.
    set_index: usize,
    /// Number of siblings.
    set_size: usize,
    label: WidgetText,
    icon: Option<IconName>,
    checked: Option<CheckState>,
}

/// The app's answer for one node: the children of `Some(&key)`, or the roots for `None`.
type ChildrenSource<'f, K> = dyn FnMut(Option<&K>, &mut Vec<TreeNode<K>>) + 'f;

/// Flatten the expanded tree into `rows`, depth first. Each expanded node is asked for its
/// children once per frame. Skips a key already seen this frame, which also breaks cycles (D10).
fn flatten<K: Clone + Ord>(
    state: &TreeState<K>,
    children: &mut ChildrenSource<'_, K>,
    parent_key: Option<&K>,
    parent: Option<usize>,
    depth: usize,
    rows: &mut Vec<Row<K>>,
    seen: &mut BTreeSet<K>,
) {
    let mut nodes = Vec::new();
    children(parent_key, &mut nodes);
    let count = nodes.len();
    for (index, node) in nodes.into_iter().enumerate() {
        let TreeNode {
            key,
            label,
            icon,
            children: kind,
            checked,
        } = node;
        if !seen.insert(key.clone()) {
            continue;
        }
        let has_children = kind != Children::None;
        let expanded = has_children && depth < MAX_DEPTH && state.expanded.contains(&key);
        let own = rows.len();
        rows.push(Row {
            key: Some(key.clone()),
            depth,
            parent,
            has_children,
            expanded,
            set_index: index,
            set_size: count,
            label,
            icon: icon.map(|icon| icon.name),
            checked,
        });
        if expanded {
            let before = rows.len();
            flatten(state, children, Some(&key), Some(own), depth + 1, rows, seen);
            if kind == Children::Loading && rows.len() == before {
                rows.push(Row {
                    key: None,
                    depth: depth + 1,
                    parent: Some(own),
                    has_children: false,
                    expanded: false,
                    set_index: 0,
                    set_size: 1,
                    label: WidgetText::from("Loading"),
                    icon: None,
                    checked: None,
                });
            }
        }
    }
}

/// Keyboard, click and expansion reports gathered inside the scroll area.
struct Reports<K> {
    /// Focus moved, so the tree scrolls it into view.
    moved: bool,
    selection_changed: bool,
    expanded: Option<K>,
    collapsed: Option<K>,
    activated: Option<K>,
    context_menu: Option<K>,
    check_toggled: Option<K>,
}

impl<K> Default for Reports<K> {
    fn default() -> Self {
        Self {
            moved: false,
            selection_changed: false,
            expanded: None,
            collapsed: None,
            activated: None,
            context_menu: None,
            check_toggled: None,
        }
    }
}

/// Hash a key or salt to the `u64` that names its widget ids. Keys need not be `Debug`.
fn hashed<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// How a move changes the selection, from the modifier keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Extend {
    /// Shift: select the range from the anchor.
    shift: bool,
    /// Ctrl: move focus without selecting.
    ctrl: bool,
}

impl Extend {
    fn from(modifiers: Modifiers) -> Self {
        Self {
            shift: modifiers.shift,
            ctrl: modifiers.command || modifiers.ctrl,
        }
    }
}

/// Keys pressed this frame, repeats included, with their modifiers.
fn pressed_keys(ui: &Ui) -> Vec<(Key, Modifiers)> {
    ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

/// How many `*` characters were typed this frame (`*` expands siblings, §5.2).
fn typed_stars(ui: &Ui) -> usize {
    ui.input(|input| {
        input
            .events
            .iter()
            .filter(|event| matches!(event, Event::Text(text) if text == "*"))
            .count()
    })
}

/// The row index of the focused node, if it is shown this frame.
fn focus_index<K: Ord>(state: &TreeState<K>, rows: &[Row<K>]) -> Option<usize> {
    let focus = state.focus.as_ref()?;
    rows.iter().position(|row| row.key.as_ref() == Some(focus))
}

/// The nearest focusable row from `from`, searching in `dir` (`1` or `-1`).
fn nearest<K>(rows: &[Row<K>], from: usize, dir: isize) -> Option<usize> {
    let mut index = from as isize;
    while index >= 0 && (index as usize) < rows.len() {
        if rows.get(index as usize).is_some_and(|row| row.key.is_some()) {
            return Some(index as usize);
        }
        index += dir;
    }
    None
}

/// The focusable row `step` rows away from `current`, clamped to the ends. Spinner rows are
/// skipped. With no focus yet, the first focusable row.
fn step_row<K>(rows: &[Row<K>], current: Option<usize>, step: isize) -> Option<usize> {
    let Some(current) = current else {
        return nearest(rows, 0, 1);
    };
    let last = rows.len().checked_sub(1)? as isize;
    let target = (current as isize + step).clamp(0, last) as usize;
    let dir = step.signum();
    if dir == 0 {
        return Some(current);
    }
    nearest(rows, target, dir).or_else(|| nearest(rows, target, -dir))
}

/// Make the node at `to` the focus and apply the selection rule for a move (§3.3).
fn move_focus<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    mode: SelectionMode,
    from: Option<usize>,
    to: usize,
    extend: Extend,
) {
    let Some(key) = rows.get(to).and_then(|row| row.key.clone()) else {
        return;
    };
    state.focus = Some(key.clone());
    match mode {
        SelectionMode::None => {}
        SelectionMode::Single => select_only(state, key),
        SelectionMode::Multi if extend.shift => select_span(state, rows, from, to, false),
        SelectionMode::Multi if extend.ctrl => {}
        SelectionMode::Multi => select_only(state, key),
    }
}

/// Make `key` the only selected node and the anchor.
fn select_only<K: Clone + Ord>(state: &mut TreeState<K>, key: K) {
    state.selection.clear();
    state.selection.insert(key.clone());
    state.anchor = Some(key);
}

/// Toggle `key` and make it the anchor (Ctrl+click, Space).
fn toggle_key<K: Clone + Ord>(state: &mut TreeState<K>, key: K) {
    if !state.selection.remove(&key) {
        state.selection.insert(key.clone());
    }
    state.anchor = Some(key);
}

/// Select the visible nodes between the anchor and row `to`. The range replaces the selection
/// unless `additive`. The anchor is the row `from` when none is set.
fn select_span<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    from: Option<usize>,
    to: usize,
    additive: bool,
) {
    if state.anchor.is_none() {
        state.anchor = from.and_then(|row| rows.get(row)).and_then(|row| row.key.clone());
    }
    let anchor = state
        .anchor
        .as_ref()
        .and_then(|anchor| rows.iter().position(|row| row.key.as_ref() == Some(anchor)))
        .or(from)
        .unwrap_or(to);
    if !additive {
        state.selection.clear();
    }
    let (low, high) = if anchor <= to { (anchor, to) } else { (to, anchor) };
    if let Some(span) = rows.get(low..=high) {
        for key in span.iter().filter_map(|row| row.key.clone()) {
            state.selection.insert(key);
        }
    }
}

/// A click on the row at `index`: select by the mode and modifiers, and focus it. Returns whether
/// the selection changed.
fn click<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    mode: SelectionMode,
    index: usize,
    modifiers: Modifiers,
) -> bool {
    let Some(key) = rows.get(index).and_then(|row| row.key.clone()) else {
        return false;
    };
    let before = state.selection.clone();
    let extend = Extend::from(modifiers);
    state.focus = Some(key.clone());
    match mode {
        SelectionMode::None => {}
        SelectionMode::Single => select_only(state, key),
        SelectionMode::Multi if extend.shift => select_span(state, rows, None, index, extend.ctrl),
        SelectionMode::Multi if extend.ctrl => toggle_key(state, key),
        SelectionMode::Multi => select_only(state, key),
    }
    state.selection != before
}

/// Expand the node at `index` if it is collapsed, collapse it if it is open.
fn toggle_at<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    index: usize,
    report: &mut Reports<K>,
) {
    let Some(row) = rows.get(index) else { return };
    let Some(key) = row.key.clone() else { return };
    if row.expanded {
        collapse(state, rows, index, report);
    } else if row.has_children && state.expanded.insert(key.clone()) {
        report.expanded = Some(key);
    }
}

/// Collapse the node at `index`. Focus moves onto it when the focus was inside it.
fn collapse<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    index: usize,
    report: &mut Reports<K>,
) {
    let Some(key) = rows.get(index).and_then(|row| row.key.clone()) else {
        return;
    };
    if !state.expanded.remove(&key) {
        return;
    }
    if let Some(focus) = focus_index(state, rows) {
        let mut cursor = Some(focus);
        while let Some(row) = cursor {
            if row == index {
                state.focus = Some(key.clone());
                break;
            }
            cursor = rows.get(row).and_then(|row| row.parent);
        }
    }
    report.collapsed = Some(key);
}

/// Apply one key press to the selection, focus and expansion (§5.2).
fn handle_key<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    mode: SelectionMode,
    key: Key,
    modifiers: Modifiers,
    page: usize,
    report: &mut Reports<K>,
) {
    let extend = Extend::from(modifiers);
    let current = focus_index(state, rows);
    let page = page as isize;
    let moved = match key {
        Key::ArrowUp => Some(step_row(rows, current, -1)),
        Key::ArrowDown => Some(step_row(rows, current, 1)),
        Key::PageUp => Some(step_row(rows, current, -page)),
        Key::PageDown => Some(step_row(rows, current, page)),
        Key::Home => Some(nearest(rows, 0, 1)),
        Key::End => Some(rows.len().checked_sub(1).and_then(|last| nearest(rows, last, -1))),
        _ => None,
    };
    if let Some(target) = moved {
        if let Some(to) = target {
            move_focus(state, rows, mode, current, to, extend);
            report.moved = true;
        }
        return;
    }
    let Some(index) = current else {
        return;
    };
    let Some(row) = rows.get(index) else { return };
    let Some(row_key) = row.key.clone() else { return };
    match key {
        Key::ArrowRight => {
            if row.has_children && !row.expanded {
                if state.expanded.insert(row_key.clone()) {
                    report.expanded = Some(row_key);
                }
            } else if row.expanded
                && rows
                    .get(index + 1)
                    .is_some_and(|child| child.parent == Some(index) && child.key.is_some())
            {
                move_focus(state, rows, mode, current, index + 1, extend);
            }
        }
        Key::ArrowLeft => {
            if row.expanded {
                collapse(state, rows, index, report);
            } else if let Some(parent) = row.parent {
                move_focus(state, rows, mode, current, parent, extend);
            }
        }
        Key::Enter => {
            report.activated = Some(row_key.clone());
            if mode == SelectionMode::None && row.has_children {
                toggle_at(state, rows, index, report);
            }
        }
        Key::Space if row.checked.is_some() => report.check_toggled = Some(row_key),
        Key::Space if mode == SelectionMode::Multi => {
            state.focus = Some(row_key.clone());
            toggle_key(state, row_key);
            report.selection_changed = true;
        }
        Key::A if extend.ctrl && mode == SelectionMode::Multi => {
            for row in rows {
                if let Some(key) = &row.key {
                    state.selection.insert(key.clone());
                }
            }
        }
        Key::F10 if modifiers.shift => report.context_menu = Some(row_key),
        _ => {}
    }
}

/// `*`: expand every collapsed sibling of the focused node that has children (§5.2).
fn expand_siblings<K: Clone + Ord>(
    state: &mut TreeState<K>,
    rows: &[Row<K>],
    current: Option<usize>,
    report: &mut Reports<K>,
) {
    let Some(parent) = current.and_then(|index| rows.get(index)).map(|row| row.parent) else {
        return;
    };
    for row in rows
        .iter()
        .filter(|row| row.parent == parent && row.has_children && !row.expanded)
    {
        if let Some(key) = &row.key
            && state.expanded.insert(key.clone())
        {
            report.expanded = Some(key.clone());
        }
    }
}

/// Paint the chevron of a row, turned from pointing right (collapsed) to down (expanded) by
/// `turn` in `0..=1`. The arms are a quarter of the spacing S, so the glyph is the 16 pt
/// ChevronRight of the catalogue.
fn paint_chevron(ui: &Ui, slot: Rect, turn: f32, color: Color32) {
    let arm_x = spacing_horizontal::S / 4.0;
    let arm_y = spacing_horizontal::S / 2.0;
    let rotation = Rot2::from_angle(turn.clamp(0.0, 1.0) * FRAC_PI_2);
    let centre = slot.center().to_vec2();
    let points = [
        Vec2::new(-arm_x, -arm_y),
        Vec2::new(arm_x, 0.0),
        Vec2::new(-arm_x, arm_y),
    ]
    .map(|offset| (centre + rotation * offset).to_pos2());
    ui.painter()
        .line(points.to_vec(), Stroke::new(stroke_width::THIN, color));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Children, MAX_DEPTH, Reports, Row, TreeNode, TreeState, flatten, step_row};

    fn children(parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>) {
        match parent {
            None => {
                out.push(TreeNode::new(1, "a").children(Children::Some));
                out.push(TreeNode::new(2, "b"));
            }
            Some(1) => {
                out.push(TreeNode::new(11, "a1"));
                out.push(TreeNode::new(12, "a2"));
            }
            Some(_) => {}
        }
    }

    fn flat(state: &TreeState<u64>) -> Vec<Row<u64>> {
        let mut rows = Vec::new();
        let mut ask = children;
        flatten(state, &mut ask, None, None, 0, &mut rows, &mut BTreeSet::new());
        rows
    }

    #[test]
    fn collapsed_nodes_are_never_asked_for_children() {
        let mut asked = Vec::new();
        let state = TreeState::<u64>::default();
        let mut record = |parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>| {
            asked.push(parent.copied());
            children(parent, out);
        };
        let mut rows = Vec::new();
        flatten(
            &state,
            &mut record,
            None,
            None,
            0,
            &mut rows,
            &mut BTreeSet::new(),
        );
        assert_eq!(asked, vec![None]);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn expanded_nodes_flatten_depth_first_with_parents() {
        let mut state = TreeState::<u64>::default();
        state.expanded.insert(1);
        let rows = flat(&state);
        let keys: Vec<u64> = rows.iter().filter_map(|row| row.key).collect();
        assert_eq!(keys, vec![1, 11, 12, 2]);
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[1].parent, Some(0));
        assert_eq!(rows[3].parent, None);
        assert_eq!(rows[1].set_size, 2);
        assert_eq!(rows[2].set_index, 1);
    }

    #[test]
    fn a_key_seen_twice_in_a_frame_is_skipped() {
        let mut state = TreeState::<u64>::default();
        state.expanded.insert(1);
        let mut dup = |parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>| {
            if parent.is_none() {
                out.push(TreeNode::new(1, "a").children(Children::Some));
                out.push(TreeNode::new(1, "again"));
            } else {
                out.push(TreeNode::new(1, "cycle").children(Children::Some));
            }
        };
        let mut rows = Vec::new();
        flatten(&state, &mut dup, None, None, 0, &mut rows, &mut BTreeSet::new());
        assert_eq!(
            rows.len(),
            1,
            "the second root and the child that repeats key 1 are skipped"
        );
    }

    #[test]
    fn nodes_at_the_depth_cap_do_not_expand() {
        // A chain where every node is expanded: the walk stops at MAX_DEPTH.
        let mut state = TreeState::<u64>::default();
        let mut chain = |parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>| {
            let next = parent.map_or(0, |key| key + 1);
            out.push(TreeNode::new(next, "n").children(Children::Some));
        };
        for key in 0..=MAX_DEPTH as u64 {
            state.expanded.insert(key);
        }
        let mut rows = Vec::new();
        flatten(&state, &mut chain, None, None, 0, &mut rows, &mut BTreeSet::new());
        assert_eq!(rows.len(), MAX_DEPTH + 1);
        assert!(
            rows.last()
                .is_some_and(|row| row.depth == MAX_DEPTH && !row.expanded)
        );
    }

    #[test]
    fn a_loading_node_gets_a_spinner_row_until_children_arrive() {
        let mut state = TreeState::<u64>::default();
        state.expanded.insert(9);
        let mut slow = |parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>| {
            if parent.is_none() {
                out.push(TreeNode::new(9, "remote").children(Children::Loading));
            }
        };
        let mut rows = Vec::new();
        flatten(&state, &mut slow, None, None, 0, &mut rows, &mut BTreeSet::new());
        assert_eq!(rows.len(), 2);
        assert!(rows[1].key.is_none());
        assert_eq!(rows[1].depth, 1);
    }

    #[test]
    fn stepping_skips_spinner_rows_and_clamps() {
        let mut state = TreeState::<u64>::default();
        state.expanded.insert(9);
        let mut slow = |parent: Option<&u64>, out: &mut Vec<TreeNode<u64>>| {
            if parent.is_none() {
                out.push(TreeNode::new(9, "remote").children(Children::Loading));
                out.push(TreeNode::new(8, "other"));
            }
        };
        let mut rows = Vec::new();
        flatten(&state, &mut slow, None, None, 0, &mut rows, &mut BTreeSet::new());
        // Rows: 9, spinner, 8. Down from 9 skips the spinner to 8; Up from 8 skips it back to 9.
        assert_eq!(step_row(&rows, Some(0), 1), Some(2));
        assert_eq!(step_row(&rows, Some(2), -1), Some(0));
        assert_eq!(step_row(&rows, Some(2), 10), Some(2));
        assert_eq!(step_row(&rows, None, 1), Some(0));
        let empty: Vec<Row<u64>> = Vec::new();
        assert_eq!(step_row(&empty, None, 1), None);
    }

    #[test]
    fn reports_start_empty() {
        let report = Reports::<u64>::default();
        assert!(report.expanded.is_none() && !report.selection_changed);
    }
}
