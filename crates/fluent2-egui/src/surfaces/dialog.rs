//! Fluent dialog, built on [`egui::Modal`].
//!
//! The backdrop, input blocking and topmost-layer tracking come from egui. This module adds the
//! Fluent surface, the focus trap, Escape and default-button policy, focus restoration and the
//! AccessKit role. The behaviour is specified in `docs/specs/dialog.md`.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Key, Layout, Modal, ModalResponse, Rect, Sense, Stroke, Ui,
    Vec2, WidgetText,
};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::controls::{Appearance, Button};
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, c};

/// Screen width below which the action row stacks.
const NARROW_SCREEN: f32 = 480.0;
/// Default maximum surface width.
const DEFAULT_WIDTH: f32 = 600.0;
/// Space kept between a full-height surface and the screen's top and bottom edges.
const SCREEN_MARGIN: f32 = spacing_vertical::XXL;
/// Title and action row height assumed on the first frame, before they have been measured.
const CHROME_ESTIMATE: f32 = 96.0;
/// The body is never squeezed below this, even on a very short screen.
const MIN_BODY_HEIGHT: f32 = 48.0;

/// How the dialog sits over the page, and what closes it.
///
/// [`DialogKind::Modal`] and [`DialogKind::Alert`] are modal: a backdrop blocks input to the UI
/// behind them. [`DialogKind::NonModal`] is not: there is no backdrop, and the page behind stays
/// interactive. Escape closes a modal or non-modal dialog; only a button closes an alert.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogKind {
    /// A backdrop click and Escape both close the dialog. The default.
    ///
    /// The backdrop blocks the UI behind the dialog.
    #[default]
    Modal,
    /// Escape closes the dialog. The page behind stays interactive.
    ///
    /// There is no backdrop, so a click outside the dialog reaches the page and does not close
    /// the dialog. [`DialogKind::Dismissible`] is the old name, from when this kind was modal.
    NonModal,
    /// Only a button closes the dialog. Backdrop clicks and Escape are ignored.
    ///
    /// The backdrop blocks the UI behind the dialog.
    Alert,
}

#[allow(non_upper_case_globals)]
impl DialogKind {
    /// The previous name of [`DialogKind::NonModal`].
    ///
    /// Kept so code that passed `Dismissible` still builds. Prefer [`DialogKind::NonModal`].
    #[deprecated(
        since = "0.3.0",
        note = "renamed to DialogKind::NonModal, which no longer blocks the page"
    )]
    pub const Dismissible: Self = Self::NonModal;

    /// Whether a backdrop click closes a dialog of this kind. Only [`DialogKind::Modal`] does.
    ///
    /// A non-modal dialog has no backdrop.
    #[must_use]
    pub const fn backdrop_dismiss(self) -> bool {
        matches!(self, Self::Modal)
    }

    /// Whether the dialog blocks the UI behind it. Only [`DialogKind::NonModal`] does not.
    #[must_use]
    pub const fn blocks_background(self) -> bool {
        !matches!(self, Self::NonModal)
    }
}

/// Why a dialog closed.
///
/// [`CloseReason::Button`] carries the action's stable id, not its display text, so two actions
/// can share a label and a translated label still closes as the same action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseReason {
    /// Escape, on a [`DialogKind::Modal`] or [`DialogKind::NonModal`] with no popup open.
    Escape,
    /// A click on the backdrop of a [`DialogKind::Modal`].
    Backdrop,
    /// An action button. The string is the id passed to [`DialogActions::primary`] or
    /// [`DialogActions::secondary`], never the label.
    Button(String),
    /// `open` was cleared by the app, or [`DialogActions::close`] was called.
    Programmatic,
}

/// Whether the dialog is still open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogResult<R> {
    /// The body ran. `R` is what the body closure returned.
    Open(R),
    /// The dialog closed on this frame. The body did not run.
    Closed(CloseReason),
}

/// The action declaration closure accepted by [`Dialog::actions`].
type ActionBuild<'a> = Box<dyn FnOnce(&mut DialogActions<'_>) + 'a>;

/// One footer button: a stable id and the text drawn on it.
struct DialogAction {
    id: String,
    text: WidgetText,
}

/// Footer buttons declared by [`Dialog::actions`].
pub struct DialogActions<'a> {
    primary: Option<DialogAction>,
    secondary: Vec<DialogAction>,
    close_requested: bool,
    dialog: Id,
    _lifetime: std::marker::PhantomData<&'a mut Ui>,
}

impl DialogActions<'_> {
    /// The primary action. Enter activates it unless focus is in a multiline field or on another
    /// button. Drawn first.
    ///
    /// `id` is stable across frames and languages. It is what [`CloseReason::Button`] returns and
    /// what the button's widget id is derived from. `text` is only what the button shows, and may
    /// be borrowed or owned.
    pub fn primary(&mut self, id: impl Into<String>, text: impl Into<WidgetText>) {
        self.primary = Some(DialogAction {
            id: id.into(),
            text: text.into(),
        });
    }

    /// A secondary action, drawn after the primary. `id` and `text` are as for [`Self::primary`].
    pub fn secondary(&mut self, id: impl Into<String>, text: impl Into<WidgetText>) {
        self.secondary.push(DialogAction {
            id: id.into(),
            text: text.into(),
        });
    }

    /// Close without a button, as [`CloseReason::Programmatic`].
    pub fn close(&mut self) {
        self.close_requested = true;
    }

    /// The egui id of a button in this dialog, for example to focus it or drive it in a test.
    ///
    /// `id` is the action id passed to [`Self::primary`] or [`Self::secondary`], not the label.
    #[must_use]
    pub fn button_id(&self, id: impl std::hash::Hash + std::fmt::Debug) -> egui::Id {
        self.dialog.with("action").with(id)
    }
}

/// A Fluent dialog.
///
/// ```
/// # use fluent2_egui::surfaces::{Dialog, DialogKind};
/// # fn demo(ctx: &egui::Context, open: &mut bool) {
/// Dialog::new("save")
///     .title("Save")
///     .kind(DialogKind::Modal)
///     .actions(|actions| {
///         actions.primary("save", "Save");
///         actions.secondary("cancel", "Cancel");
///     })
///     .show(ctx, open, |ui| {
///         ui.label("Save this document?");
///     });
/// # }
/// ```
pub struct Dialog<'a> {
    id_salt: Id,
    title: WidgetText,
    kind: DialogKind,
    width: f32,
    actions: Option<ActionBuild<'a>>,
    description: Option<String>,
}

impl<'a> Dialog<'a> {
    /// A dialog whose id comes from `id_salt`, never from the title.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            title: WidgetText::default(),
            kind: DialogKind::Modal,
            width: DEFAULT_WIDTH,
            actions: None,
            description: None,
        }
    }

    /// Title. It is also the accessible name.
    #[must_use]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = title.into();
        self
    }

    /// Dismissal policy. The default is [`DialogKind::Modal`].
    ///
    /// Every dialog is modal and blocks background input. The kind only chooses whether a
    /// backdrop click or Escape closes it.
    #[must_use]
    pub fn kind(mut self, kind: DialogKind) -> Self {
        self.kind = kind;
        self
    }

    /// Maximum surface width, in points. The default is 600. Non-finite values use the default.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Plain-text accessible description, read with the title when the dialog opens.
    ///
    /// Without it the dialog is still described by its body (AccessKit `described_by`), but a
    /// screen reader that does not follow that relation reads only the focused button on open
    /// (WI-1850). Use the body's main sentence, for example "Your document has unsaved changes."
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Footer buttons. See [`DialogActions`].
    #[must_use]
    pub fn actions(mut self, build: impl FnOnce(&mut DialogActions<'_>) + 'a) -> Self {
        self.actions = Some(Box::new(build));
        self
    }

    /// Show the dialog while `*open` is `true`.
    ///
    /// On the frame it closes, the body does not run and the result is [`DialogResult::Closed`].
    /// Otherwise the result is [`DialogResult::Open`] with the body's value.
    pub fn show<R>(self, ctx: &Context, open: &mut bool, body: impl FnOnce(&mut Ui) -> R) -> DialogResult<R> {
        let id = self.id_salt;
        let surface_id = id.with("surface");
        let title_id = id.with("title");
        let Some(opening) = track_open(ctx, id, *open) else {
            return DialogResult::Closed(CloseReason::Programmatic);
        };
        let theme = Theme::get(ctx);

        let width = if self.width.is_finite() && self.width > 0.0 {
            self.width.min(10_000.0)
        } else {
            DEFAULT_WIDTH
        };
        let stack_actions = ctx.content_rect().width() < NARROW_SCREEN;
        let radius = CornerRadius::same(rounded_u8(border_radius::X_LARGE));
        let frame = Frame::new()
            .fill(c(theme.color.neutral_background_1))
            // Fluent's surface border: invisible in light and dark, the window-text edge in
            // high contrast, where the fill alone does not separate the dialog from the page.
            .stroke(Stroke::new(stroke_width::THIN, c(theme.color.transparent_stroke)))
            .corner_radius(radius)
            .inner_margin(Vec2::new(spacing_horizontal::XXL, spacing_vertical::XXL))
            .shadow(shadow64(&theme));

        let blocks = self.kind.blocks_background();
        let content = |ui: &mut Ui| {
            ui.set_max_width(width);
            ui.set_min_width(width.min(ui.available_width().max(0.0)));

            // Strong steps need the semibold family, which may not be bound yet. Draw in the
            // proportional family until it is, as the buttons do.
            crate::controls::ensure_type_ramp(ui);
            let title_text = self.title.text().to_owned();
            let title_response = ui
                .push_id(title_id, |ui| {
                    ui.label(text::rich(title_text.clone(), TypeRamp::Subtitle1))
                })
                .inner;

            // The body scrolls when it is taller than the screen leaves room for, so the
            // title and actions stay visible and every control stays inside the surface
            // (WI-1912). The title and action heights come from the previous frame.
            let chrome_key = id.with("chrome_height");
            let chrome = ui
                .ctx()
                .data(|data| data.get_temp::<f32>(chrome_key))
                .unwrap_or(CHROME_ESTIMATE);
            let max_body =
                (ui.ctx().content_rect().height() - 2.0 * (SCREEN_MARGIN + spacing_vertical::XXL) - chrome)
                    .max(MIN_BODY_HEIGHT);
            // egui 0.37's ScrollArea clips children to the viewport even when the content
            // fits. On the first frame that viewport is shorter than the body, so a button
            // in the body never receives the click and is missing from AccessKit, and the
            // click falls through to the backdrop. Scroll only once a measured body is
            // taller than the room left for it; otherwise paint the body directly.
            let height_key = id.with("body_content_height");
            let known_height = ui.ctx().data(|data| data.get_temp::<f32>(height_key));
            let scroll = known_height.is_some_and(|height| height > max_body);
            let before = ui.min_rect();
            // One call site: `body` is `FnOnce`, so both branches cannot name it.
            let paint = |ui: &mut Ui| show_body(ui, id, body);
            let (shown, content_height) = if scroll {
                let scrolled = egui::ScrollArea::vertical()
                    .id_salt(id.with("body"))
                    .max_height(max_body)
                    .auto_shrink([false, true])
                    .show(ui, paint);
                // `content_size` is the body's height, not the clipped viewport.
                ((scrolled.inner, scrolled.inner_rect), scrolled.content_size.y)
            } else {
                let shown = paint(ui);
                let after = ui.min_rect();
                let body_rect = Rect::from_min_max(before.min.min(after.min), after.max);
                ((shown, body_rect), after.height() - before.height())
            };
            let (value, body_node) = shown.0;
            let body_rect = shown.1;
            if known_height.is_none_or(|height| (height - content_height).abs() > 0.5) {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(height_key, content_height));
                ui.ctx().request_discard("dialog body height changed");
            }

            let mut declared = DialogActions {
                primary: None,
                secondary: Vec::new(),
                close_requested: false,
                dialog: id,
                _lifetime: std::marker::PhantomData,
            };
            if let Some(build) = self.actions {
                build(&mut declared);
            }
            let primary_id = declared.primary.as_ref().map(|action| action.id.clone());
            let shown_actions = show_actions(ui, declared, stack_actions, opening);
            // Measured from the layout cursor, not `min_rect`: the surface keeps last
            // frame's size, so its rect would feed the old height back in and never shrink.
            let measured =
                ((body_rect.top() - ui.max_rect().top()) + (ui.cursor().top() - body_rect.bottom())).max(0.0);
            if (measured - chrome).abs() > 0.5 {
                ui.ctx().data_mut(|data| data.insert_temp(chrome_key, measured));
                ui.ctx().request_discard("dialog chrome height changed");
            }

            // A focusable surface, so Tab has a stop even when the body has no widgets.
            // It is allocated last, which makes it the last stop; the title is not a stop.
            // The area's size can lag the footer by a frame. Grow the node so AccessKit
            // bounds cover the actions as well as the body.
            let surface_rect = ui.min_rect().union(body_rect).union(ui.max_rect());
            let surface = ui.interact(surface_rect, surface_id, Sense::focusable_noninteractive());
            if opening && primary_id.is_none() {
                surface.request_focus();
            }

            let role = match self.kind {
                DialogKind::Alert => egui::accesskit::Role::AlertDialog,
                DialogKind::Modal | DialogKind::NonModal => egui::accesskit::Role::Dialog,
            };
            ui.ctx().accesskit_node_builder(surface_id, |node| {
                node.set_role(role);
                node.set_label(title_text.clone());
                node.push_labelled_by(title_response.id.accesskit_id());
                // Focus opens on the primary button, so the title and body are announced
                // through the dialog's description, as Fluent does with aria-describedby
                // (WI-1850).
                node.push_described_by(body_node.accesskit_id());
                if let Some(description) = &self.description {
                    node.set_description(description.clone());
                }
            });

            (value, primary_id, shown_actions)
        };
        // A modal dialog uses `egui::Modal`, which blocks the page and records whether a popup
        // was already open. A non-modal dialog uses the same centred area with no backdrop, so a
        // click outside it reaches the page and does not close the dialog.
        let shown = if blocks {
            Modal::new(id)
                .backdrop_color(c(theme.color.background_overlay))
                .frame(frame)
                .area(Modal::default_area(id))
                .show(ctx, content)
        } else {
            let area = Modal::default_area(id);
            let any_popup_open = egui::Popup::is_any_open(ctx);
            let surface_frame = frame;
            let shown = area.show(ctx, move |ui| {
                ui.scope_builder(
                    egui::UiBuilder::new().sense(egui::Sense::CLICK | egui::Sense::DRAG),
                    |ui| surface_frame.show(ui, content).inner,
                )
                .inner
            });
            // No backdrop, so a click on the area is not a backdrop click. A dummy response
            // built from the area's would move the real one, so the click is ignored by the
            // `blocks` guard at the close check instead.
            ModalResponse {
                response: shown.response.clone(),
                backdrop_response: shown.response,
                inner: shown.inner,
                is_top_modal: true,
                any_popup_open,
            }
        };

        let (value, primary_id, action_state) = shown.inner;
        let mut close = None;
        if let Some(id) = action_state.clicked {
            close = Some(CloseReason::Button(id));
        }
        if action_state.close_requested {
            close = Some(CloseReason::Programmatic);
        }
        if blocks && shown.backdrop_response.clicked() && self.kind == DialogKind::Modal {
            close = Some(CloseReason::Backdrop);
        }

        // A popup open before the body owns Escape. Otherwise the dialog reads the key even when
        // a focused control's event filter would have hidden it from `key_pressed`.
        if close.is_none() && shown.is_top_modal && !shown.any_popup_open {
            let escape = escape_pressed(ctx);
            if escape && !matches!(self.kind, DialogKind::Alert) {
                close = Some(CloseReason::Escape);
            }
            let enter = ctx.input(|input| {
                input.key_pressed(Key::Enter)
                    || input.events.iter().any(|event| {
                        matches!(
                            event,
                            egui::Event::Key {
                                key: Key::Enter,
                                pressed: true,
                                ..
                            }
                        )
                    })
            });
            // Enter belongs to a text field (it may be a newline) and to another button. Egui
            // reports `wants_keyboard_input` for any text edit, so single-line fields also keep
            // Enter; see the spec's "Enter and the default button" section.
            let on_other = ctx
                .data(|data| data.get_temp::<bool>(id.with("focus_other_button")))
                .unwrap_or(false);
            if enter
                && !on_other
                && !ctx.text_edit_focused()
                && let Some(id) = primary_id
            {
                close = Some(CloseReason::Button(id));
            }
        }

        if let Some(reason) = close {
            *open = false;
            finish_close(ctx, id);
            return DialogResult::Closed(reason);
        }

        DialogResult::Open(value)
    }
}

/// Scroll the body so a control that has just gained focus is in view.
///
/// Egui only scrolls to a widget focused by its own Tab handling; the dialog's focus trap and
/// `request_focus` calls move focus without that, so the body does it here (WI-1912).
fn reveal_new_focus(ui: &Ui, key: Id) {
    let focused = ui.ctx().memory(|memory| memory.focused());
    let last = ui.ctx().data(|data| data.get_temp::<Option<Id>>(key)).flatten();
    if focused == last {
        return;
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, focused));
    let Some(response) = focused.and_then(|id| ui.ctx().read_response(id)) else {
        return;
    };
    if ui.min_rect().contains_rect(response.rect) && !ui.clip_rect().contains_rect(response.rect) {
        ui.scroll_to_rect(response.rect, None);
    }
}

struct ActionState {
    clicked: Option<String>,
    close_requested: bool,
}

/// Opener tracking shared by the modal surfaces (Dialog, Drawer, BottomSheet).
///
/// Returns `None` while `open` is `false`, after giving focus back to the opener if the surface
/// was open last frame. Otherwise returns whether this is the frame the surface opens, and on
/// that frame records the focused widget as the opener.
pub(crate) fn track_open(ctx: &Context, id: Id, open: bool) -> Option<bool> {
    let opener_key = id.with("opener");
    let seen_key = id.with("seen");

    // A close requests focus for the opener while the modal still blocks it, and egui
    // drops a focus request for a widget that was not interactive last frame. Ask again on
    // the first frame after the close (found by the WI-1586 Orca walk).
    if let Some(opener) = ctx.data_mut(|data| {
        data.remove_temp::<Option<Id>>(opener_key.with("pending"))
            .flatten()
    }) {
        ctx.memory_mut(|memory| memory.request_focus(opener));
    }

    if !open {
        // Closed by the app: forget the open state so the next opening captures the opener again.
        if ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false)) {
            finish_close(ctx, id);
        }
        return None;
    }

    let opening = !ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false));
    if opening {
        let previous = ctx.memory(|memory| memory.focused());
        ctx.data_mut(|data| {
            data.insert_temp(opener_key, previous);
            data.insert_temp(seen_key, true);
        });
    }
    Some(opening)
}

/// Mark the surface closed and give focus back to the widget that had it when it opened.
pub(crate) fn finish_close(ctx: &Context, id: Id) {
    let opener_key = id.with("opener");
    ctx.data_mut(|data| data.insert_temp(id.with("seen"), false));
    if let Some(opener) = ctx.data(|data| data.get_temp::<Option<Id>>(opener_key)).flatten() {
        ctx.memory_mut(|memory| memory.request_focus(opener));
        ctx.data_mut(|data| data.insert_temp(opener_key.with("pending"), Some(opener)));
    }
}

/// Escape was pressed this frame, even if a focused control's event filter hid it from
/// `key_pressed`.
pub(crate) fn escape_pressed(ctx: &Context) -> bool {
    ctx.input(|input| {
        input.key_pressed(Key::Escape)
            || input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    }
                )
            })
    })
}

fn show_body<R>(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui) -> R) -> (R, Id) {
    // The body gets its own container node, which describes the dialog.
    ui.scope_builder(egui::UiBuilder::new().id_salt("fluent2_dialog_body"), |ui| {
        let value = body(ui);
        reveal_new_focus(ui, id.with("body_focus"));
        (value, ui.unique_id())
    })
    .inner
}

fn show_actions(ui: &mut Ui, actions: DialogActions<'_>, stack: bool, opening: bool) -> ActionState {
    let dialog_scope = actions.dialog;
    // Recomputed every frame: `note_button` sets it again if focus is on a non-primary button.
    ui.ctx()
        .data_mut(|data| data.insert_temp(dialog_scope.with("focus_other_button"), false));
    let buttons: Vec<(DialogAction, Appearance)> = actions
        .primary
        .into_iter()
        .map(|action| (action, Appearance::Primary))
        .chain(
            actions
                .secondary
                .into_iter()
                .map(|action| (action, Appearance::Secondary)),
        )
        .collect();
    let mut clicked = None;
    let primary_id = buttons
        .first()
        .and_then(|(action, appearance)| (*appearance == Appearance::Primary).then(|| action.id.clone()));
    if stack {
        ui.vertical(|ui| {
            for (action, appearance) in &buttons {
                let response = ui.add_sized(
                    Vec2::new(ui.available_width(), 32.0),
                    Button::new(action.text.clone())
                        .appearance(*appearance)
                        .id_salt(&action.id),
                );
                note_button(
                    ui,
                    dialog_scope,
                    response.id,
                    &action.id,
                    primary_id.as_deref(),
                    opening,
                );
                if response.clicked() {
                    clicked = Some(action.id.clone());
                }
            }
        });
    } else if !buttons.is_empty() {
        // `Align::Min`, not `Center`: a centred cross axis spans all the height left in the
        // surface, which is last frame's height, so the row (and the surface) never shrank.
        // The buttons share one height, so top alignment looks the same (WI-1912).
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = spacing_horizontal::S;
            for (action, appearance) in buttons.iter().rev() {
                let response = ui.add(
                    Button::new(action.text.clone())
                        .appearance(*appearance)
                        .id_salt(&action.id),
                );
                note_button(
                    ui,
                    dialog_scope,
                    response.id,
                    &action.id,
                    primary_id.as_deref(),
                    opening,
                );
                if response.clicked() {
                    clicked = Some(action.id.clone());
                }
            }
        });
    }
    ActionState {
        clicked,
        close_requested: actions.close_requested,
    }
}

/// Remember which button is focused so Enter can tell the primary from another button.
fn note_button(
    ui: &Ui,
    dialog_scope: Id,
    button_id: Id,
    action_id: &str,
    primary: Option<&str>,
    opening: bool,
) {
    let ctx = ui.ctx();
    if Some(action_id) == primary {
        ctx.data_mut(|data| data.insert_temp(dialog_scope.with("primary_focus"), button_id));
        if opening {
            ui.memory_mut(|memory| memory.request_focus(button_id));
        }
    }
    let focused = ui.memory(|memory| memory.focused()) == Some(button_id);
    if focused && Some(action_id) != primary {
        ctx.data_mut(|data| data.insert_temp(dialog_scope.with("focus_other_button"), true));
    }
}

pub(crate) fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

/// `shadow64`, the same single-layer conversion `theme::install` uses for windows.
pub(crate) fn shadow64(theme: &Theme) -> egui::Shadow {
    let layers = theme.shadow.shadow_64;
    let Some(layer) = layers.iter().max_by(|left, right| {
        left.blur
            .total_cmp(&right.blur)
            .then(left.spread.total_cmp(&right.spread))
    }) else {
        return egui::Shadow::NONE;
    };
    if !layer.x.is_finite() || !layer.y.is_finite() || !layer.blur.is_finite() || !layer.spread.is_finite() {
        return egui::Shadow::NONE;
    }
    egui::Shadow {
        offset: [rounded_i8(layer.x), rounded_i8(layer.y)],
        blur: rounded_u8(layer.blur),
        spread: rounded_u8(layer.spread),
        color: c(layer.color),
    }
}

fn rounded_i8(value: f32) -> i8 {
    if !value.is_finite() {
        0
    } else {
        value.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8
    }
}
