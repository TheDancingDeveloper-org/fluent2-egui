//! The keyboard shortcut help overlay: a [`Dialog`] that lists every binding in the installed
//! [`Shortcuts`](crate::foundation::shortcuts::Shortcuts) registry, by group.
//!
//! The overlay only shows. Opening it is the app's choice, usually a `?` binding in the same
//! registry, so the help key is listed too.

use egui::{Align, Context, Grid, Layout};

use fluent2_tokens::generated::global::{spacing_horizontal, spacing_vertical};

use crate::foundation::shortcuts;
use crate::foundation::text::{self, TypeRamp};
use crate::surfaces::{Dialog, DialogResult};
use crate::theme::{Theme, c};

/// Width of the overlay surface.
const WIDTH: f32 = 480.0;

/// Help dialog listing the app's shortcuts by group, in registration order.
///
/// ```
/// use egui::{Key, KeyboardShortcut, Modifiers};
/// use fluent2_egui::foundation::shortcuts::{Shortcut, ShortcutsExt};
/// use fluent2_egui::surfaces::ShortcutOverlay;
///
/// # fn demo(ctx: &egui::Context, help_open: &mut bool) {
/// if ctx.shortcut_pressed("help.shortcuts") {
///     *help_open = !*help_open;
/// }
/// let _ = ShortcutOverlay::new("shortcut-help").show(ctx, help_open);
/// # }
/// ```
pub struct ShortcutOverlay {
    id_salt: egui::Id,
    title: String,
}

impl ShortcutOverlay {
    /// An overlay titled "Keyboard shortcuts".
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: egui::Id::new(id_salt),
            title: "Keyboard shortcuts".to_owned(),
        }
    }

    /// Replace the title, for example with a translation.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Show the overlay while `open` is set. Escape or a backdrop click clears it.
    pub fn show(self, ctx: &Context, open: &mut bool) -> DialogResult<()> {
        let registry = shortcuts::get(ctx);
        Dialog::new(self.id_salt)
            .title(self.title)
            .width(WIDTH)
            .show(ctx, open, |ui| {
                let theme = Theme::get(ui.ctx());
                let caption_color = c(theme.color.neutral_foreground_2);
                for (index, group) in registry.groups().into_iter().enumerate() {
                    if index > 0 {
                        ui.add_space(spacing_vertical::L);
                    }
                    ui.label(text::rich(group, TypeRamp::Body1Strong));
                    ui.add_space(spacing_vertical::XS);
                    Grid::new(ui.id().with(("shortcut-group", index)))
                        .num_columns(2)
                        .spacing([spacing_horizontal::XXL, spacing_vertical::S])
                        .striped(false)
                        .show(ui, |ui| {
                            for entry in registry.iter().filter(|e| e.group_name() == group) {
                                ui.label(text::rich(entry.label.as_str(), TypeRamp::Body1));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        text::rich(ui.ctx().format_shortcut(&entry.keys), TypeRamp::Caption1)
                                            .color(caption_color),
                                    );
                                });
                                ui.end_row();
                            }
                        });
                }
            })
    }
}
