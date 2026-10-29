//! Inter-pane drag and drop: move a typed payload from one pane to another.
//!
//! [`DragSource`] starts a drag from a widget the app has already drawn. Past egui's drag
//! threshold the payload is stored in egui's `DragAndDrop` and a ghost follows the pointer.
//! [`DropTarget`] wraps any other widget. An accepting target under the pointer gets a brand
//! highlight; a refusing one shows the not-allowed cursor. Releasing over an accepting target
//! reports a [`PaneDrop`]. Escape cancels, and so does a release anywhere else.
//!
//! Drag and drop has no keyboard form, so [`move_menu`] lists the same targets in a menu
//! (secondary click or Shift+F10). Choosing one reports the same drop with
//! [`MoveSource::Keyboard`].
//!
//! A polite AccessKit live region announces when a drag starts, when it hovers a target, and
//! when it is dropped or cancelled. Call [`announce`] once a frame from the source pane.
//!
//! Nothing here changes app data: the app reads the payload and the target and applies the
//! change. OS file drops stay in [`crate::platform::file_drop`]. Row drags inside a DataGrid
//! stay in [`crate::data::row_drag`].

use egui::{
    Align2, CornerRadius, CursorIcon, Id, LayerId, Order, Response, Sense, Stroke, StrokeKind, Ui, WidgetText,
};

use crate::data::reorder::MoveSource;
use crate::foundation::text::{TypeRamp, font_id};
use crate::surfaces::{MenuItem, submenu};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

/// What a pane drag is doing, for the AccessKit announcement.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DragPhase {
    Idle,
    Dragging { label: String },
    Hovering { label: String, target: String },
    Dropped { label: String, target: String },
    Cancelled { label: String },
}

/// The id of the polite live region that describes the drag.
#[must_use]
pub fn live_region_id() -> Id {
    Id::unique("fluent2_pane_drag_live")
}

fn phase_id() -> Id {
    Id::unique("fluent2_pane_drag_phase")
}

/// Say what the drag is doing, once a frame, from the source pane.
///
/// The region is created the first time a drag starts, so an app that never drags has no extra
/// node. The text changes when the phase changes: dragging, hovering a target, dropped, or
/// cancelled.
pub fn announce(ui: &mut Ui) {
    let phase = ui.data(|data| data.get_temp::<DragPhase>(phase_id()).unwrap_or(DragPhase::Idle));
    let text = match &phase {
        DragPhase::Idle => return,
        DragPhase::Dragging { label } => format!("Dragging {label}. Press Escape to cancel."),
        DragPhase::Hovering { label, target } => format!("Dragging {label} over {target}."),
        DragPhase::Dropped { label, target } => format!("Dropped {label} on {target}."),
        DragPhase::Cancelled { label } => format!("Cancelled drag of {label}."),
    };
    // Allocate a widget so the Status node is in the tree kittest and AccessKit walk.
    let (rect, response) = ui.allocate_exact_size(egui::vec2(1.0, 1.0), Sense::hover());
    let bounds = egui::accesskit::Rect {
        x0: f64::from(rect.min.x),
        y0: f64::from(rect.min.y),
        x1: f64::from(rect.max.x),
        y1: f64::from(rect.max.y),
    };
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_live(egui::accesskit::Live::Polite);
        node.set_label(text);
        node.set_bounds(bounds);
    });
    let _ = live_region_id();
}

fn set_phase(ui: &Ui, phase: DragPhase) {
    ui.data_mut(|data| data.insert_temp(phase_id(), phase));
}

fn accepted_id() -> Id {
    Id::unique("fluent2_pane_drag_accepted")
}

fn escape_id(source: Id) -> Id {
    source.with("fluent2_pane_drag_escape")
}

/// Call once at the end of the frame, after every [`DropTarget`], from the same `Ui` scope as
/// [`announce`]. A pointer release that no accepting target took cancels the drag.
pub fn end_frame<T: 'static + Clone + Send + Sync>(ui: &Ui) {
    let Some(drag) = PaneDrag::<T>::current(ui.ctx()) else {
        ui.data_mut(|data| data.remove_temp::<bool>(accepted_id()));
        return;
    };
    let accepted = ui.data(|data| data.get_temp::<bool>(accepted_id()).unwrap_or(false));
    let escaped = ui.data(|data| data.get_temp::<bool>(escape_id(drag.source)).unwrap_or(false));
    ui.data_mut(|data| {
        data.remove_temp::<bool>(accepted_id());
        data.remove_temp::<bool>(escape_id(drag.source));
    });
    // Apply after every widget so a hovered label cannot keep the text cursor.
    let released = ui.input(|input| input.pointer.any_released());
    if (escaped || released) && !accepted {
        set_phase(
            ui,
            DragPhase::Cancelled {
                label: drag.label.clone(),
            },
        );
        egui::DragAndDrop::clear_payload(ui.ctx());
    }
}

/// A widget that can start a drag carrying `T`.
///
/// `T` must be `'static + Clone + Send + Sync`, which is what egui stores in `DragAndDrop`.
///
/// ```
/// # use fluent2_egui::data::pane_drag::DragSource;
/// # fn demo(ui: &mut egui::Ui) {
/// let card = ui.label("Invoice 12");
/// DragSource::new(&card, "invoice-12", "Invoice 12").show(ui);
/// # }
/// ```
pub struct DragSource<'a, T> {
    response: &'a Response,
    payload: T,
    label: String,
}

impl<'a, T> DragSource<'a, T> {
    /// Drag `payload` from `response`. `label` is the ghost text and the announcement.
    #[must_use]
    pub fn new(response: &'a Response, payload: T, label: impl Into<String>) -> Self {
        Self {
            response,
            payload,
            label: label.into(),
        }
    }
}

impl<T: 'static + Clone + Send + Sync> DragSource<'_, T> {
    /// Start or continue the drag. Call every frame after the source widget is drawn.
    ///
    /// Returns `true` while this source's payload is the one being dragged.
    pub fn show(self, ui: &Ui) -> bool {
        // Escape is consumed here so a later release the same frame cannot also drop.
        // `end_frame` announces the cancel once the payload is still present.
        if ui.input(|input| input.key_pressed(egui::Key::Escape))
            && PaneDrag::<T>::current(ui.ctx()).is_some()
        {
            ui.ctx()
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            ui.data_mut(|data| data.insert_temp(escape_id(self.response.id), true));
        }
        let dragging = self.response.dragged() && ui.is_enabled();
        if dragging && PaneDrag::<T>::current(ui.ctx()).is_none() {
            let drag = PaneDrag {
                source: self.response.id,
                payload: self.payload.clone(),
                label: self.label.clone(),
            };
            egui::DragAndDrop::set_payload(ui.ctx(), drag);
            set_phase(
                ui,
                DragPhase::Dragging {
                    label: self.label.clone(),
                },
            );
        }
        let mine = PaneDrag::<T>::current(ui.ctx()).is_some_and(|drag| drag.source == self.response.id);
        if mine {
            if self.response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
            if let Some(drag) = PaneDrag::<T>::current(ui.ctx()) {
                paint_ghost(ui, &drag.label);
            }
        }
        mine
    }
}

/// The payload of a pane drag, stored in egui's `DragAndDrop` while the drag lasts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneDrag<T> {
    /// The widget the drag started on.
    pub source: Id,
    /// The value the app supplied.
    pub payload: T,
    /// Ghost and announcement text.
    pub label: String,
}

impl<T: 'static + Clone + Send + Sync> PaneDrag<T> {
    /// The pane drag in progress, if its payload is `T`.
    #[must_use]
    pub fn current(ctx: &egui::Context) -> Option<Self> {
        egui::DragAndDrop::payload::<Self>(ctx).map(|payload| (*payload).clone())
    }
}

/// A payload released on a target, by pointer or through [`move_menu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneDrop<T> {
    /// The widget the drag started on. Keyboard moves use the source the app names.
    pub source: Id,
    /// The value that was dropped.
    pub payload: T,
    /// The target's label.
    pub target: String,
    /// Pointer drop or keyboard (menu) choice.
    pub how: MoveSource,
}

/// What [`DropTarget::show`] reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropTargetResponse<T> {
    /// A pane drag of `T` is over the target this frame.
    pub hovered: bool,
    /// The payload was released over the target this frame.
    pub dropped: Option<PaneDrop<T>>,
}

/// Turns any [`Response`] into a target for a [`PaneDrag<T>`].
///
/// ```
/// # use fluent2_egui::data::pane_drag::DropTarget;
/// # fn demo(ui: &mut egui::Ui) {
/// let folder = ui.label("Inbox");
/// if let Some(drop) = DropTarget::<String>::new(&folder, "Inbox").show(ui).dropped {
///     let _ = drop.payload;
/// }
/// # }
/// ```
pub struct DropTarget<'a, T> {
    response: &'a Response,
    label: String,
    accepts: bool,
    _payload: std::marker::PhantomData<T>,
}

impl<'a, T> DropTarget<'a, T> {
    /// A target over `response`'s rect. `label` is used in the highlight announcement.
    #[must_use]
    pub fn new(response: &'a Response, label: impl Into<String>) -> Self {
        Self {
            response,
            label: label.into(),
            accepts: true,
            _payload: std::marker::PhantomData,
        }
    }

    /// Whether the target takes the drop. A refusing target shows the not-allowed cursor while
    /// a drag hovers it and never reports a drop. Default `true`. A disabled `Ui` refuses too.
    #[must_use]
    pub fn accepts(mut self, accepts: bool) -> Self {
        self.accepts = accepts;
        self
    }
}

impl<T> Default for DropTargetResponse<T> {
    fn default() -> Self {
        Self {
            hovered: false,
            dropped: None,
        }
    }
}

impl<T: 'static + Clone + Send + Sync> DropTarget<'_, T> {
    /// Paint the hover state and report a drop. Call every frame after the widget is drawn.
    pub fn show(self, ui: &Ui) -> DropTargetResponse<T> {
        let Some(drag) = PaneDrag::<T>::current(ui.ctx()) else {
            return DropTargetResponse::default();
        };
        let rect = self.response.rect;
        let over = ui
            .ctx()
            .pointer_interact_pos()
            .is_some_and(|pointer| rect.contains(pointer));
        if !over {
            return DropTargetResponse::default();
        }
        let accepts = self.accepts && ui.is_enabled();
        if !accepts {
            ui.ctx().set_cursor_icon(CursorIcon::NotAllowed);
            return DropTargetResponse {
                hovered: true,
                dropped: None,
            };
        }
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        paint_highlight(ui, rect);
        set_phase(
            ui,
            DragPhase::Hovering {
                label: drag.label.clone(),
                target: self.label.clone(),
            },
        );
        let released = ui.input(|input| input.pointer.any_released());
        let dropped = released.then(|| {
            set_phase(
                ui,
                DragPhase::Dropped {
                    label: drag.label.clone(),
                    target: self.label.clone(),
                },
            );
            // The source sees this and does not also cancel the same release.
            ui.data_mut(|data| data.insert_temp(accepted_id(), true));
            egui::DragAndDrop::clear_payload(ui.ctx());
            PaneDrop {
                source: drag.source,
                payload: drag.payload.clone(),
                target: self.label.clone(),
                how: MoveSource::Pointer,
            }
        });
        DropTargetResponse {
            hovered: true,
            dropped,
        }
    }
}

/// The brand fill and stroke of an accepting target under a drag.
fn paint_highlight(ui: &Ui, rect: egui::Rect) {
    let theme = Theme::get(ui.ctx());
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Foreground,
        ui.scope_id().with("pane_drop_target"),
    ));
    painter.rect(
        rect,
        radius,
        c(theme.color.brand_background_2),
        Stroke::new(stroke_width::THICK, c(theme.color.compound_brand_stroke)),
        StrokeKind::Inside,
    );
}

/// The ghost that follows the pointer.
fn paint_ghost(ui: &Ui, text: &str) {
    let Some(pointer) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    let theme = Theme::get(ui.ctx());
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Tooltip,
        Id::unique("fluent2_pane_drag_ghost"),
    ));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        font_id(TypeRamp::Body1),
        c(theme.color.neutral_foreground_on_brand),
    );
    let pad = egui::vec2(8.0, 4.0);
    let rect = Align2::LEFT_TOP.anchor_size(pointer + egui::vec2(12.0, 12.0), galley.size() + pad * 2.0);
    painter.rect_filled(
        rect,
        CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
        c(theme.color.brand_background),
    );
    painter.galley(rect.min + pad, galley, c(theme.color.neutral_foreground_on_brand));
}

/// One entry of [`move_menu`]: a target's label and whether it takes the payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveChoice<'a> {
    /// Menu text, usually the target's own label.
    pub label: &'a str,
    /// A refusing target is shown disabled.
    pub accepts: bool,
}

/// The keyboard alternative to dragging: a submenu titled `title` (for example "Move to")
/// listing `choices`. Returns the index of the chosen entry. The app builds the [`PaneDrop`].
///
/// ```
/// # use fluent2_egui::data::pane_drag::{MoveChoice, move_menu};
/// # fn demo(ui: &mut egui::Ui) {
/// let choices = [
///     MoveChoice { label: "Inbox", accepts: true },
///     MoveChoice { label: "Archive", accepts: false },
/// ];
/// if let Some(index) = move_menu(ui, "Move to", &choices) {
///     let _ = choices[index].label;
/// }
/// # }
/// ```
pub fn move_menu(ui: &mut Ui, title: impl Into<WidgetText>, choices: &[MoveChoice<'_>]) -> Option<usize> {
    submenu(ui, title, None, |ui| move_choices(ui, choices))
        .inner
        .flatten()
}

/// The entries of [`move_menu`] without the submenu.
pub fn move_choices(ui: &mut Ui, choices: &[MoveChoice<'_>]) -> Option<usize> {
    let mut chosen = None;
    for (index, choice) in choices.iter().enumerate() {
        let item = MenuItem::new(choice.label).id_salt(("fluent2_pane_move", index));
        if ui.add_enabled(choice.accepts, item).clicked() {
            chosen = Some(index);
        }
    }
    chosen
}

/// Make a label both a drag source and a hover target for tests and simple lists.
///
/// Returns the [`Response`] of the label so the caller can open a context menu on it.
pub fn draggable_label(ui: &mut Ui, text: impl Into<String>, payload: impl Into<String>) -> Response {
    let text = text.into();
    let payload = payload.into();
    let response = ui.add(egui::Label::new(text.clone()).sense(Sense::click_and_drag()));
    DragSource::new(&response, payload, text).show(ui);
    response
}

/// A token radius as egui's `u8`, clamped.
fn rounded_u8(value: f32) -> u8 {
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::{MoveChoice, PaneDrag, PaneDrop};
    use crate::data::MoveSource;

    #[test]
    fn drop_keeps_the_payload() {
        let drag = PaneDrag {
            source: egui::Id::unique("card"),
            payload: "invoice-12".to_owned(),
            label: "Invoice 12".to_owned(),
        };
        let drop = PaneDrop {
            source: drag.source,
            payload: drag.payload.clone(),
            target: "Inbox".to_owned(),
            how: MoveSource::Keyboard,
        };
        assert_eq!(drop.payload, "invoice-12");
        assert_eq!(drop.how, MoveSource::Keyboard);
        let choices = [MoveChoice {
            label: "Inbox",
            accepts: true,
        }];
        assert!(choices[0].accepts);
    }
}
