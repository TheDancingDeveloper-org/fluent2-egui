//! Fluent bottom sheet: a modal surface that slides up from the bottom edge, for narrow windows.
//!
//! It is the Fluent Drawer at position `bottom` (`@fluentui/react-drawer` 9.9.4, see
//! [`super::Drawer`]) with a grab handle and top corners rounded. It shares the drawer's and
//! [`super::Dialog`]'s focus trap, Escape policy and focus return. The behaviour is specified in
//! `docs/specs/drawer.md`.

use egui::{Context, Id, Ui, WidgetText};

use super::drawer::{Edge, Overlay, show_overlay};
use super::{DialogKind, DialogResult};

/// Default height, as a fraction of the window height.
const DEFAULT_FRACTION: f32 = 0.5;

/// A Fluent bottom sheet.
///
/// ```
/// # use fluent2_egui::surfaces::BottomSheet;
/// # fn demo(ctx: &egui::Context, open: &mut bool) {
/// BottomSheet::new("more")
///     .title("More")
///     .height_fraction(0.4)
///     .show(ctx, open, |ui| {
///         ui.label("Settings");
///     });
/// # }
/// ```
pub struct BottomSheet {
    id_salt: Id,
    title: WidgetText,
    fraction: f32,
    kind: DialogKind,
}

impl BottomSheet {
    /// A sheet whose id comes from `id_salt`, never from the title.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            title: WidgetText::default(),
            fraction: DEFAULT_FRACTION,
            kind: DialogKind::Modal,
        }
    }

    /// Title in the header. It is also the accessible name.
    #[must_use]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = title.into();
        self
    }

    /// Height as a fraction of the window height, clamped to `0.1..=1.0`. The default is 0.5.
    /// Non-finite values use the default.
    #[must_use]
    pub fn height_fraction(mut self, fraction: f32) -> Self {
        self.fraction = if fraction.is_finite() {
            fraction.clamp(0.1, 1.0)
        } else {
            DEFAULT_FRACTION
        };
        self
    }

    /// How the sheet reacts to the backdrop and Escape, as for [`super::Dialog`]. The default is
    /// [`DialogKind::Modal`].
    #[must_use]
    pub fn kind(mut self, kind: DialogKind) -> Self {
        self.kind = kind;
        self
    }

    /// Show the sheet while `*open` is `true`.
    ///
    /// On the frame it closes, the body does not run and the result is [`DialogResult::Closed`].
    pub fn show<R>(self, ctx: &Context, open: &mut bool, body: impl FnOnce(&mut Ui) -> R) -> DialogResult<R> {
        let extent = ctx.content_rect().height() * self.fraction;
        show_overlay(
            ctx,
            open,
            Overlay {
                id: self.id_salt,
                title: self.title,
                kind: self.kind,
                edge: Edge::Bottom,
                extent,
            },
            body,
        )
    }
}
