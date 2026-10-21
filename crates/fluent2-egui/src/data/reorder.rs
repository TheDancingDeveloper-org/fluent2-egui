//! Reorder: drag to reorder items in a list and move them between lists, with a Fluent drop
//! indicator and a keyboard alternative.
//!
//! The helper never changes app data. It reports a [`ReorderMove`] and the app applies it (or
//! sends it to a server first, or refuses it). [`ReorderMove::apply`] and
//! [`ReorderMove::apply_between`] do the common `Vec` case.
//!
//! Pointer: drag an item; a `compound_brand_stroke` bar shows where it will land; release to
//! drop, Escape to cancel. Keyboard: with an item focused, Alt+Up/Down (Alt+Left/Right in a
//! horizontal container) moves it one place, and Alt+Left/Right (Alt+Up/Down in a horizontal
//! container) moves it to the neighbouring container, in the order the containers were shown.
//!
//! Read-only boards: show the containers in a disabled `Ui` (`ui.add_enabled_ui(false, …)`);
//! disabled items neither drag nor move by keyboard.
//!
//! Design reference: gpui `on_drag`/`drag_over` (`elements/div.rs`, fluent-gpui). No code is
//! adapted from it.

use egui::accesskit::Role;
use egui::{
    CornerRadius, CursorIcon, Id, InnerResponse, Key, LayerId, Modifiers, Order, Pos2, Rect, Sense, Ui,
    UiBuilder, WidgetInfo, WidgetType, emath::TSTransform,
};

use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

/// Where an item sits: a container and an index within it.
///
/// ```
/// let slot = fluent2_egui::data::reorder::Slot { container: 0, index: 2 };
/// assert_eq!(slot.index, 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Slot {
    /// The container, as passed to [`Reorder::container`].
    pub container: usize,
    /// Index within the container.
    pub index: usize,
}

/// How a move was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MoveSource {
    /// A pointer drag and drop.
    Pointer,
    /// Alt plus an arrow key.
    Keyboard,
}

/// A move the user asked for. The app applies it.
///
/// `to.index` is the index the item has **after** the move, so applying it is "remove at
/// `from.index`, insert at `to.index`".
///
/// ```
/// use fluent2_egui::data::reorder::{MoveSource, ReorderMove, Slot};
/// let mut items = vec!["a", "b", "c"];
/// let mv = ReorderMove {
///     from: Slot { container: 0, index: 0 },
///     to: Slot { container: 0, index: 2 },
///     source: MoveSource::Pointer,
/// };
/// assert!(mv.apply(&mut items));
/// assert_eq!(items, ["b", "c", "a"]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReorderMove {
    /// Where the item was.
    pub from: Slot,
    /// Where the item goes.
    pub to: Slot,
    /// Pointer or keyboard.
    pub source: MoveSource,
}

impl ReorderMove {
    /// Whether the item changes container.
    #[must_use]
    pub fn between_containers(&self) -> bool {
        self.from.container != self.to.container
    }

    /// Apply a move within one list. Returns `false`, leaving `list` alone, when the move
    /// changes container or an index is out of range.
    pub fn apply<T>(&self, list: &mut Vec<T>) -> bool {
        if self.between_containers() || self.from.index >= list.len() || self.to.index >= list.len() {
            return false;
        }
        let item = list.remove(self.from.index);
        list.insert(self.to.index, item);
        true
    }

    /// Apply a move across lists indexed by container. Returns `false`, leaving `lists` alone,
    /// when a container or index is out of range.
    ///
    /// ```
    /// use fluent2_egui::data::reorder::{MoveSource, ReorderMove, Slot};
    /// let mut columns = vec![vec![1, 2], vec![3]];
    /// let mv = ReorderMove {
    ///     from: Slot { container: 0, index: 1 },
    ///     to: Slot { container: 1, index: 0 },
    ///     source: MoveSource::Keyboard,
    /// };
    /// assert!(mv.apply_between(&mut columns));
    /// assert_eq!(columns, [vec![1], vec![2, 3]]);
    /// ```
    pub fn apply_between<T>(&self, lists: &mut [Vec<T>]) -> bool {
        if !self.between_containers() {
            return lists
                .get_mut(self.from.container)
                .is_some_and(|list| self.apply(list));
        }
        let from_ok = lists
            .get(self.from.container)
            .is_some_and(|list| self.from.index < list.len());
        let to_ok = lists
            .get(self.to.container)
            .is_some_and(|list| self.to.index <= list.len());
        if !from_ok || !to_ok {
            return false;
        }
        let Some(item) = lists
            .get_mut(self.from.container)
            .map(|list| list.remove(self.from.index))
        else {
            return false;
        };
        match lists.get_mut(self.to.container) {
            Some(list) => {
                list.insert(self.to.index, item);
                true
            }
            None => false,
        }
    }
}

/// Direction items flow in a container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReorderAxis {
    /// Top to bottom (a list or a board column).
    #[default]
    Vertical,
    /// Left to right (a tab strip).
    Horizontal,
}

/// What [`Reorder::finish`] reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReorderResponse {
    /// A drop or a keyboard move this frame.
    pub moved: Option<ReorderMove>,
    /// The item being dragged, while a drag is in progress.
    pub dragging: Option<Slot>,
    /// Where the dragged item would land if dropped now (final index, as in [`ReorderMove`]).
    pub target: Option<Slot>,
}

/// The drag payload, stored in egui's `DragAndDrop`.
#[derive(Clone, Copy, Debug)]
struct Payload {
    reorder: Id,
    from: Slot,
}

#[derive(Clone, Copy, Debug)]
enum KeyMove {
    Back,
    Forward,
    PrevContainer,
    NextContainer,
}

#[derive(Clone, Copy, Debug)]
struct ContainerRecord {
    container: usize,
    len: usize,
}

/// A drag-reorder session for one frame: create it, show containers, then [`Reorder::finish`].
///
/// ```
/// # use fluent2_egui::data::reorder::{Reorder, ReorderAxis};
/// # fn demo(ui: &mut egui::Ui, columns: &mut Vec<Vec<String>>) {
/// let mut reorder = Reorder::new("board");
/// for (col, cards) in columns.iter().enumerate() {
///     reorder.container(ui, col, ReorderAxis::Vertical, |list, ui| {
///         for card in cards {
///             list.item(ui, card, card, |ui| {
///                 ui.label(card);
///             });
///         }
///     });
/// }
/// if let Some(mv) = reorder.finish(ui).moved {
///     mv.apply_between(columns);
/// }
/// # }
/// ```
pub struct Reorder {
    id: Id,
    containers: Vec<ContainerRecord>,
    key_move: Option<(Slot, KeyMove)>,
    moved: Option<ReorderMove>,
    target: Option<Slot>,
}

impl Reorder {
    /// Start a reorder session. Items can only be dropped into containers of the same session.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(("fluent2_reorder", id_salt)),
            containers: Vec::new(),
            key_move: None,
            moved: None,
            target: None,
        }
    }

    /// Show one container. Call [`ReorderList::item`] for each item, in order, inside `add`.
    ///
    /// Items are laid out along `axis`. An empty container accepts drops anywhere in its rect,
    /// so give it a minimum size.
    pub fn container<R>(
        &mut self,
        ui: &mut Ui,
        container: usize,
        axis: ReorderAxis,
        add: impl FnOnce(&mut ReorderList, &mut Ui) -> R,
    ) -> R {
        let mut list = ReorderList {
            reorder: self.id,
            container,
            axis,
            items: Vec::new(),
            key_move: None,
        };
        // Items flow along the container's axis, whatever the parent layout.
        let InnerResponse { inner, response } = match axis {
            ReorderAxis::Vertical => ui.vertical(|ui| add(&mut list, ui)),
            ReorderAxis::Horizontal => ui.horizontal(|ui| add(&mut list, ui)),
        };
        if let Some(key_move) = list.key_move {
            self.key_move = Some(key_move);
        }
        self.containers.push(ContainerRecord {
            container,
            len: list.items.len(),
        });
        self.drop_target(ui, response.rect, &list);
        inner
    }

    /// Hover and drop handling for one container after its items are laid out.
    fn drop_target(&mut self, ui: &Ui, rect: Rect, list: &ReorderList) {
        let Some(payload) = egui::DragAndDrop::payload::<Payload>(ui.ctx()) else {
            return;
        };
        if payload.reorder != self.id || !ui.is_enabled() {
            return;
        }
        let Some(pointer) = ui.ctx().pointer_interact_pos() else {
            return;
        };
        if !rect.contains(pointer) {
            return;
        }
        let slot = insertion_slot(&list.items, list.axis, pointer);
        let to_index = if payload.from.container == list.container && slot > payload.from.index {
            slot - 1
        } else {
            slot
        };
        let to = Slot {
            container: list.container,
            index: to_index,
        };
        self.target = Some(to);
        if to != payload.from {
            paint_indicator(ui, rect, &list.items, list.axis, slot);
        }
        if ui.input(|input| input.pointer.any_released()) {
            egui::DragAndDrop::clear_payload(ui.ctx());
            if to != payload.from {
                self.moved = Some(ReorderMove {
                    from: payload.from,
                    to,
                    source: MoveSource::Pointer,
                });
            }
        }
    }

    /// End the session and report what happened this frame.
    pub fn finish(self, ui: &Ui) -> ReorderResponse {
        let dragging = egui::DragAndDrop::payload::<Payload>(ui.ctx())
            .filter(|payload| payload.reorder == self.id)
            .map(|payload| payload.from);
        let moved = self.moved.or_else(|| {
            let (from, key_move) = self.key_move?;
            keyboard_target(&self.containers, from, key_move).map(|to| ReorderMove {
                from,
                to,
                source: MoveSource::Keyboard,
            })
        });
        ReorderResponse {
            moved,
            dragging: if moved.is_some() { None } else { dragging },
            target: if dragging.is_some() { self.target } else { None },
        }
    }
}

/// The final slot of a keyboard move, or `None` at an edge.
fn keyboard_target(containers: &[ContainerRecord], from: Slot, key_move: KeyMove) -> Option<Slot> {
    let position = containers
        .iter()
        .position(|record| record.container == from.container)?;
    let len = containers.get(position)?.len;
    match key_move {
        KeyMove::Back => from.index.checked_sub(1).map(|index| Slot { index, ..from }),
        KeyMove::Forward => (from.index + 1 < len).then_some(Slot {
            index: from.index + 1,
            ..from
        }),
        KeyMove::PrevContainer | KeyMove::NextContainer => {
            let neighbour = if matches!(key_move, KeyMove::PrevContainer) {
                containers.get(position.checked_sub(1)?)?
            } else {
                containers.get(position + 1)?
            };
            Some(Slot {
                container: neighbour.container,
                index: from.index.min(neighbour.len),
            })
        }
    }
}

/// The insertion point among `items` (0..=len) for a pointer at `pointer`: after every item
/// whose centre is before the pointer along the axis.
fn insertion_slot(items: &[Rect], axis: ReorderAxis, pointer: Pos2) -> usize {
    items
        .iter()
        .filter(|rect| match axis {
            ReorderAxis::Vertical => rect.center().y < pointer.y,
            ReorderAxis::Horizontal => rect.center().x < pointer.x,
        })
        .count()
}

/// Paint the drop bar at insertion point `slot`, between items or at either end.
fn paint_indicator(ui: &Ui, container: Rect, items: &[Rect], axis: ReorderAxis, slot: usize) {
    let theme = Theme::get(ui.ctx());
    let gap = ui.spacing().item_spacing;
    let thickness = stroke_width::THICK;
    let before = slot.checked_sub(1).and_then(|index| items.get(index));
    let after = items.get(slot);
    let bar = match axis {
        ReorderAxis::Vertical => {
            let y = match (before, after) {
                (Some(a), Some(b)) => (a.bottom() + b.top()) / 2.0,
                (Some(a), None) => a.bottom() + gap.y / 2.0,
                (None, Some(b)) => b.top() - gap.y / 2.0,
                (None, None) => container.top() + thickness,
            };
            Rect::from_min_max(
                Pos2::new(container.left(), y - thickness / 2.0),
                Pos2::new(container.right(), y + thickness / 2.0),
            )
        }
        ReorderAxis::Horizontal => {
            let x = match (before, after) {
                (Some(a), Some(b)) => (a.right() + b.left()) / 2.0,
                (Some(a), None) => a.right() + gap.x / 2.0,
                (None, Some(b)) => b.left() - gap.x / 2.0,
                (None, None) => container.left() + thickness,
            };
            Rect::from_min_max(
                Pos2::new(x - thickness / 2.0, container.top()),
                Pos2::new(x + thickness / 2.0, container.bottom()),
            )
        }
    };
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Foreground, ui.id().with("reorder_bar")));
    painter.rect_filled(
        bar,
        CornerRadius::same(rounded_u8(border_radius::CIRCULAR)),
        c(theme.color.compound_brand_stroke),
    );
}

/// A token radius as egui's `u8`, clamped.
fn rounded_u8(value: f32) -> u8 {
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

/// One container's items. Passed to the closure of [`Reorder::container`].
pub struct ReorderList {
    reorder: Id,
    container: usize,
    axis: ReorderAxis,
    items: Vec<Rect>,
    key_move: Option<(Slot, KeyMove)>,
}

impl ReorderList {
    /// Show one draggable item. `id` must be stable for the item wherever it sits (not its
    /// index), so focus follows it after a move. `label` names it for assistive technology.
    ///
    /// Interactive children (buttons) inside the item keep their clicks; a drag anywhere on
    /// the item moves it.
    pub fn item<R>(
        &mut self,
        ui: &mut Ui,
        id: impl std::hash::Hash + std::fmt::Debug,
        label: impl Into<String>,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let item_id = self.reorder.with(("item", id));
        let slot = Slot {
            container: self.container,
            index: self.items.len(),
        };
        let enabled = ui.is_enabled();
        let dragged = enabled && ui.ctx().is_being_dragged(item_id);

        let InnerResponse { inner, response } = if dragged {
            egui::DragAndDrop::set_payload(
                ui.ctx(),
                Payload {
                    reorder: self.reorder,
                    from: slot,
                },
            );
            // The item follows the pointer on a top layer; its slot keeps its space.
            let layer = LayerId::new(Order::Tooltip, item_id);
            let shown = ui.scope_builder(UiBuilder::new().layer_id(layer), add);
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                let delta = pointer - shown.response.rect.center();
                ui.ctx()
                    .transform_layer_shapes(layer, TSTransform::from_translation(delta));
            }
            shown
        } else {
            ui.scope(add)
        };
        // While dragged, the content's rect follows the layer transform; the slot keeps the
        // rect the item had at rest.
        let rest_id = item_id.with("rest_rect");
        let rect = if dragged {
            ui.ctx()
                .data(|data| data.get_temp::<Rect>(rest_id))
                .unwrap_or(response.rect)
        } else {
            ui.ctx().data_mut(|data| data.insert_temp(rest_id, response.rect));
            response.rect
        };
        self.items.push(rect);

        let mut item = ui.interact(rect, item_id, Sense::drag());
        if enabled {
            item = item.on_hover_cursor(CursorIcon::Grab);
        }
        if item.drag_started() {
            item.request_focus();
        }
        let label = label.into();
        item.widget_info(|| WidgetInfo::labeled(WidgetType::Unknown, enabled, &label));
        ui.ctx().accesskit_node_builder(item_id, |node| {
            node.set_role(Role::ListItem);
            node.set_description("Alt+arrow keys to move");
        });

        if enabled && item.has_focus() {
            let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::ALT, key));
            let (back, forward, prev, next) = match self.axis {
                ReorderAxis::Vertical => (Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight),
                ReorderAxis::Horizontal => (Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown),
            };
            let key_move = if pressed(back) {
                Some(KeyMove::Back)
            } else if pressed(forward) {
                Some(KeyMove::Forward)
            } else if pressed(prev) {
                Some(KeyMove::PrevContainer)
            } else if pressed(next) {
                Some(KeyMove::NextContainer)
            } else {
                None
            };
            if let Some(key_move) = key_move {
                self.key_move = Some((slot, key_move));
            }
        }
        if focus_visible(ui, &item) {
            paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }

        // While dragged, the content is on another layer and cannot be merged with the item.
        if dragged {
            InnerResponse::new(inner, item)
        } else {
            InnerResponse::new(inner, item | response)
        }
    }

    /// Number of items shown so far in this container.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no item has been shown yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(container: usize, len: usize) -> ContainerRecord {
        ContainerRecord { container, len }
    }

    #[test]
    fn keyboard_edges_return_none() {
        let containers = [rec(0, 3), rec(1, 0)];
        let first = Slot {
            container: 0,
            index: 0,
        };
        assert_eq!(keyboard_target(&containers, first, KeyMove::Back), None);
        assert_eq!(keyboard_target(&containers, first, KeyMove::PrevContainer), None);
        let last = Slot {
            container: 0,
            index: 2,
        };
        assert_eq!(keyboard_target(&containers, last, KeyMove::Forward), None);
        assert_eq!(
            keyboard_target(&containers, last, KeyMove::NextContainer),
            Some(Slot {
                container: 1,
                index: 0
            })
        );
    }

    #[test]
    fn insertion_slot_counts_items_before_pointer() {
        let items = [
            Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(10.0, 10.0)),
            Rect::from_min_size(Pos2::new(0.0, 12.0), egui::vec2(10.0, 10.0)),
        ];
        assert_eq!(
            insertion_slot(&items, ReorderAxis::Vertical, Pos2::new(5.0, 2.0)),
            0
        );
        assert_eq!(
            insertion_slot(&items, ReorderAxis::Vertical, Pos2::new(5.0, 8.0)),
            1
        );
        assert_eq!(
            insertion_slot(&items, ReorderAxis::Vertical, Pos2::new(5.0, 30.0)),
            2
        );
    }
}
