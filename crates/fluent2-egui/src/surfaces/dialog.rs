//! Fluent dialog, built on [`egui::Modal`].
//!
//! The backdrop, input blocking and topmost-layer tracking come from egui. This module adds the
//! Fluent surface, the focus trap, Escape and default-button policy, focus restoration and the
//! AccessKit role. The behaviour is specified in `docs/specs/dialog.md`.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Key, Layout, Modal, Sense, Stroke, Ui, Vec2, WidgetText,
};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::controls::{Appearance, Button};
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, c};

/// Screen width below which the action row stacks.
const NARROW_SCREEN: f32 = 480.0;
/// Default maximum surface width.
const DEFAULT_WIDTH: f32 = 600.0;

/// How the dialog reacts to the backdrop and to Escape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogKind {
    /// A backdrop click and Escape both close the dialog. The default.
    #[default]
    Modal,
    /// Escape closes the dialog. A backdrop click does not.
    NonModal,
    /// Only a button closes the dialog. Backdrop clicks and Escape are ignored.
    Alert,
}

/// Why a dialog closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseReason {
    /// Escape, on a [`DialogKind::Modal`] or [`DialogKind::NonModal`] with no popup open.
    Escape,
    /// A click on the backdrop of a [`DialogKind::Modal`].
    Backdrop,
    /// An action button. The label is the button's text.
    Button(&'static str),
    /// `open` was cleared by the app, or [`DialogActions::close`] was called.
    Programmatic,
}

/// Whether the dialog is still open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogResult<R> {
    /// The body ran. `R` is what the body closure returned.
    Open(R),
    /// The dialog closed on this frame. The body did not run.
    Closed(CloseReason),
}

/// The action declaration closure accepted by [`Dialog::actions`].
type ActionBuild<'a> = Box<dyn FnOnce(&mut DialogActions<'_>) + 'a>;

/// Footer buttons declared by [`Dialog::actions`].
pub struct DialogActions<'a> {
    primary: Option<&'static str>,
    secondary: Vec<&'static str>,
    close_requested: bool,
    dialog: Id,
    _lifetime: std::marker::PhantomData<&'a mut Ui>,
}

impl DialogActions<'_> {
    /// The primary action. Enter activates it unless focus is in a multiline field or on another
    /// button. Drawn first.
    pub fn primary(&mut self, label: &'static str) {
        self.primary = Some(label);
    }

    /// A secondary action, drawn after the primary.
    pub fn secondary(&mut self, label: &'static str) {
        self.secondary.push(label);
    }

    /// Close without a button, as [`CloseReason::Programmatic`].
    pub fn close(&mut self) {
        self.close_requested = true;
    }

    /// The egui id of a button in this dialog, for example to focus it or drive it in a test.
    #[must_use]
    pub fn button_id(&self, label: &'static str) -> egui::Id {
        self.dialog.with("action").with(label)
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
///         actions.primary("Save");
///         actions.secondary("Cancel");
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
}

impl<'a> Dialog<'a> {
    /// A dialog whose id comes from `id_salt`, never from the title.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            title: WidgetText::default(),
            kind: DialogKind::Modal,
            width: DEFAULT_WIDTH,
            actions: None,
        }
    }

    /// Title. It is also the accessible name.
    #[must_use]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = title.into();
        self
    }

    /// Modality. The default is [`DialogKind::Modal`].
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
        let opener_key = id.with("opener");
        let seen_key = id.with("seen");

        if !*open {
            // Closed by the app: forget the open state so the next opening captures the opener again.
            if ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false)) {
                restore_focus(ctx, opener_key);
                ctx.data_mut(|data| data.insert_temp(seen_key, false));
            }
            return DialogResult::Closed(CloseReason::Programmatic);
        }

        let theme = Theme::get(ctx);
        let opening = !ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false));
        if opening {
            let previous = ctx.memory(|memory| memory.focused());
            ctx.data_mut(|data| {
                data.insert_temp(opener_key, previous);
                data.insert_temp(seen_key, true);
            });
        }

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

        // `egui::Modal` blocks the page and records whether a popup was already open.
        let shown = Modal::new(id)
            .backdrop_color(c(theme.color.background_overlay))
            .frame(frame)
            .area(Modal::default_area(id))
            .show(ctx, |ui| {
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

                let value = body(ui);

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
                let primary = declared.primary;
                let shown_actions = show_actions(ui, declared, stack_actions, opening);

                // A focusable surface, so Tab has a stop even when the body has no widgets.
                // It is allocated last, which makes it the last stop; the title is not a stop.
                let surface = ui.interact(ui.min_rect(), surface_id, Sense::focusable_noninteractive());
                if opening && primary.is_none() {
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
                });

                (value, primary, shown_actions)
            });

        let (value, primary, action_state) = shown.inner;
        let mut close = None;
        if let Some(label) = action_state.clicked {
            close = Some(CloseReason::Button(label));
        }
        if action_state.close_requested {
            close = Some(CloseReason::Programmatic);
        }
        if shown.backdrop_response.clicked() && self.kind == DialogKind::Modal {
            close = Some(CloseReason::Backdrop);
        }

        // A popup open before the body owns Escape. Otherwise the dialog reads the key even when
        // a focused control's event filter would have hidden it from `key_pressed`.
        if close.is_none() && shown.is_top_modal && !shown.any_popup_open {
            let escape = ctx.input(|input| {
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
            });
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
                && let Some(label) = primary
            {
                close = Some(CloseReason::Button(label));
            }
        }

        if let Some(reason) = close {
            *open = false;
            ctx.data_mut(|data| data.insert_temp(seen_key, false));
            restore_focus(ctx, opener_key);
            return DialogResult::Closed(reason);
        }

        DialogResult::Open(value)
    }
}

struct ActionState {
    clicked: Option<&'static str>,
    close_requested: bool,
}

/// Give focus back to the widget that had it when the dialog opened.
fn restore_focus(ctx: &Context, opener_key: Id) {
    if let Some(opener) = ctx.data(|data| data.get_temp::<Option<Id>>(opener_key)).flatten() {
        ctx.memory_mut(|memory| memory.request_focus(opener));
    }
}

fn show_actions(ui: &mut Ui, actions: DialogActions<'_>, stack: bool, opening: bool) -> ActionState {
    let dialog_scope = actions.dialog;
    // Recomputed every frame: `note_button` sets it again if focus is on a non-primary button.
    ui.ctx()
        .data_mut(|data| data.insert_temp(dialog_scope.with("focus_other_button"), false));
    let labels: Vec<(&'static str, Appearance)> = actions
        .primary
        .into_iter()
        .map(|label| (label, Appearance::Primary))
        .chain(
            actions
                .secondary
                .iter()
                .copied()
                .map(|label| (label, Appearance::Secondary)),
        )
        .collect();
    let mut clicked = None;
    let primary_label = actions.primary;
    if stack {
        ui.vertical(|ui| {
            for (label, appearance) in &labels {
                let response = ui.add_sized(
                    Vec2::new(ui.available_width(), 32.0),
                    Button::new(*label).appearance(*appearance).id_salt(*label),
                );
                note_button(ui, dialog_scope, response.id, label, primary_label, opening);
                if response.clicked() {
                    clicked = Some(*label);
                }
            }
        });
    } else if !labels.is_empty() {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = spacing_horizontal::S;
            for (label, appearance) in labels.iter().rev() {
                let response = ui.add(Button::new(*label).appearance(*appearance).id_salt(*label));
                note_button(ui, dialog_scope, response.id, label, primary_label, opening);
                if response.clicked() {
                    clicked = Some(*label);
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
fn note_button(ui: &Ui, dialog_scope: Id, button_id: Id, label: &str, primary: Option<&str>, opening: bool) {
    let ctx = ui.ctx();
    if Some(label) == primary {
        ctx.data_mut(|data| data.insert_temp(dialog_scope.with("primary_focus"), button_id));
        if opening {
            ui.memory_mut(|memory| memory.request_focus(button_id));
        }
    }
    let focused = ui.memory(|memory| memory.focused()) == Some(button_id);
    if focused && Some(label) != primary {
        ctx.data_mut(|data| data.insert_temp(dialog_scope.with("focus_other_button"), true));
    }
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

/// `shadow64`, the same single-layer conversion `theme::install` uses for windows.
fn shadow64(theme: &Theme) -> egui::Shadow {
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
