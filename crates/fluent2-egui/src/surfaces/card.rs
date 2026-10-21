//! Fluent Card.
//!
//! Filled, filled-alternative, outline and subtle faces. A selectable card is a checkbox-like
//! toggle. Radius is `borderRadiusMedium`, padding is 12 px (`spacingHorizontalM`), and the
//! shadow is `shadow4` (hover `shadow8` when the card is interactive).

use egui::epaint::RectShape;
use egui::{
    CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, WidgetInfo, WidgetText,
    WidgetType, accesskit,
};

use fluent2_icons::{Icon, Size as IconSize};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::controls::button;
use crate::foundation::elevation::{self, Elevation};
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::paint_icon;
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// Card face.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardAppearance {
    /// `neutral_background_1` and `shadow4`. The default.
    #[default]
    Filled,
    /// `neutral_background_2` and `shadow4`.
    FilledAlternative,
    /// Transparent fill and a thin stroke. No resting shadow.
    Outline,
    /// No fill and no stroke until hover.
    Subtle,
}

/// A Fluent card. Content is a closure.
///
/// ```
/// # use fluent2_egui::surfaces::Card;
/// # fn demo(ui: &mut egui::Ui) {
/// Card::new().show(ui, |ui| {
///     ui.label("Body");
/// });
/// # }
/// ```
pub struct Card<'a> {
    appearance: CardAppearance,
    id_salt: Option<Id>,
    selected: Option<&'a mut bool>,
    label: Option<String>,
}

impl<'a> Card<'a> {
    /// A filled card.
    #[must_use]
    pub fn new() -> Self {
        Self {
            appearance: CardAppearance::Filled,
            id_salt: None,
            selected: None,
            label: None,
        }
    }

    /// Face. Defaults to [`CardAppearance::Filled`].
    #[must_use]
    pub fn appearance(mut self, appearance: CardAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Extra salt so two cards can share a parent without colliding.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Make the card a checkbox-like toggle bound to `selected`.
    ///
    /// [`Response::changed`] is true when the toggle flipped this frame. Give it a
    /// [`label`](Self::label) so the toggle has a name for assistive technology.
    #[must_use]
    pub fn selectable(mut self, selected: &'a mut bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Accessible name of a selectable card, for example the card's title.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show the card and run `content` inside the padded area.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui)) -> Response {
        let theme = Theme::get(ui.ctx());
        button::ensure_type_ramp(ui);
        let enabled = ui.is_enabled();
        let Card {
            appearance,
            id_salt,
            mut selected,
            label,
        } = self;
        let interactive = selected.is_some();
        let id = id_salt.unwrap_or_else(|| ui.next_auto_id()).with("fluent2_card");
        let padding = spacing_horizontal::M;
        let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));

        let width = finite_pos(ui.available_width()).max(padding * 2.0);
        let cursor = ui.cursor().min;
        let layer = ui.layer_id();
        let frame_idx = ui
            .ctx()
            .graphics_mut(|gfx| gfx.entry(layer).add(ui.clip_rect(), egui::Shape::Noop));
        // The card's hit area is registered before its content. egui gives the widget added
        // last the click, so registering after the content would let the card take clicks
        // meant for a button inside it. The area is last frame's measured size (zero on the
        // first frame), so it can lag one frame behind a size change.
        let size_memory = id.with("measured_size");
        let measured: Option<Vec2> = ui.ctx().data_mut(|data| data.get_temp(size_memory));
        let sense = if interactive {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(
            Rect::from_min_size(cursor, measured.unwrap_or(Vec2::ZERO)),
            id,
            sense,
        );
        // High contrast fills a hovered, pressed or selected interactive card with the highlight
        // colour, so its content takes the matching foreground (WI-1845). The hit area was
        // registered above, so this frame's state is known before the content runs.
        let ink = card_ink(
            &theme,
            interactive && enabled,
            response.hovered(),
            response.is_pointer_button_down_on(),
            selected.as_deref().copied().unwrap_or(false),
        );
        let scope = ui.scope(|ui| {
            if let Some(ink) = ink {
                ui.visuals_mut().override_text_color = Some(ink);
            }
            ui.set_min_width(width);
            ui.add_space(padding);
            ui.horizontal(|ui| {
                ui.add_space(padding);
                ui.vertical(|ui| {
                    ui.set_max_width((width - padding * 2.0).max(0.0));
                    content(ui);
                });
            });
            ui.add_space(padding);
        });
        let rect = Rect::from_min_size(cursor, scope.response.rect.size());
        ui.ctx()
            .data_mut(|data| data.insert_temp(size_memory, rect.size()));

        let hovered = response.hovered();
        let pressed = response.is_pointer_button_down_on();
        let was = selected.as_deref().copied().unwrap_or(false);
        let activated =
            interactive && enabled && (response.clicked() || (response.has_focus() && space_or_enter(ui)));
        let selected_now = if activated {
            if let Some(slot) = selected.as_mut() {
                **slot = !**slot;
            }
            !was
        } else {
            was
        };
        if activated {
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let (fill, stroke, shadow) = card_face(
                appearance,
                &theme,
                enabled,
                hovered,
                pressed,
                selected_now,
                interactive,
            );
            if let Some(step) = shadow {
                elevation::paint_shadow(ui.painter(), rect, radius, step, &theme);
            }
            let shape = RectShape::new(
                rect,
                radius,
                c(fill),
                Stroke::new(stroke_width::THIN, c(stroke)),
                StrokeKind::Inside,
            );
            ui.ctx().graphics_mut(|gfx| {
                gfx.entry(layer).set(frame_idx, ui.clip_rect(), shape.into());
            });
            if interactive && focus::focus_visible(ui, &response) {
                paint_focus_ring(ui, rect, radius);
            }
        }

        if interactive {
            let name = label.clone().unwrap_or_default();
            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::CheckBox, enabled, selected_now, name.clone())
            });
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_role(accesskit::Role::CheckBox);
                node.set_toggled(if selected_now {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                });
            });
        }
        response
    }
}

impl Default for Card<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// The trailing action of a [`CardHeader`], run with the header's `Ui`.
type HeaderAction<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// Title row of a card: optional image, title, description and a trailing action.
///
/// ```
/// # use fluent2_egui::surfaces::CardHeader;
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// CardHeader::new("Title").description("One line").image(Icon::Calendar).show(ui);
/// # }
/// ```
pub struct CardHeader<'a> {
    title: WidgetText,
    description: Option<WidgetText>,
    image: Option<Icon>,
    action: Option<HeaderAction<'a>>,
}

impl<'a> CardHeader<'a> {
    /// A header with a title.
    pub fn new(title: impl Into<WidgetText>) -> Self {
        Self {
            title: title.into(),
            description: None,
            image: None,
            action: None,
        }
    }

    /// Secondary line under the title, in `neutral_foreground_2`.
    #[must_use]
    pub fn description(mut self, text: impl Into<WidgetText>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Leading icon.
    #[must_use]
    pub fn image(mut self, icon: Icon) -> Self {
        self.image = Some(icon);
        self
    }

    /// Trailing action, shown at the end of the row.
    #[must_use]
    pub fn action(mut self, action: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.action = Some(Box::new(action));
        self
    }

    /// Show the header.
    pub fn show(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        button::ensure_type_ramp(ui);
        // Inside a hovered, pressed or selected card the card sets the text colour for its fill
        // (WI-1845); the header follows it for the title, description and icon.
        let ink = ui.visuals().override_text_color;
        let title_color = ink.unwrap_or_else(|| c(theme.color.neutral_foreground_1));
        let description_color = ink.unwrap_or_else(|| c(theme.color.neutral_foreground_2));
        ui.horizontal(|ui| {
            if let Some(icon) = self.image {
                let size = 32.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
                paint_icon(
                    ui,
                    rect,
                    icon,
                    IconSize::S20,
                    fluent2_icons::Style::Regular,
                    Some(title_color),
                );
            }
            ui.vertical(|ui| {
                let galley = self.title.into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    ui.available_width(),
                    TextStyle::Name(TypeRamp::Body1Strong.name().into()),
                );
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(galley.size().x.max(1.0), galley.size().y.max(1.0)),
                    Sense::hover(),
                );
                ui.painter().galley(rect.min, galley, title_color);
                if let Some(description) = self.description {
                    let galley = description.into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        ui.available_width(),
                        TextStyle::Name(TypeRamp::Caption1.name().into()),
                    );
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(galley.size().x.max(1.0), galley.size().y.max(1.0)),
                        Sense::hover(),
                    );
                    ui.painter().galley(rect.min, galley, description_color);
                }
            });
            if let Some(action) = self.action {
                action(ui);
            }
        })
        .response
    }
}

/// The content foreground for an interactive card's fill, or `None` at rest.
fn card_ink(
    theme: &Theme,
    live: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
) -> Option<egui::Color32> {
    if !live {
        return None;
    }
    let color = if pressed {
        theme.color.neutral_foreground_1_pressed
    } else if hovered {
        theme.color.neutral_foreground_1_hover
    } else if selected {
        theme.color.neutral_foreground_1_selected
    } else {
        return None;
    };
    Some(c(color))
}

fn card_face(
    appearance: CardAppearance,
    theme: &Theme,
    enabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
    interactive: bool,
) -> (fluent2_tokens::Rgba, fluent2_tokens::Rgba, Option<Elevation>) {
    let color = &theme.color;
    match appearance {
        CardAppearance::Filled => {
            let fill = if !enabled {
                color.neutral_background_disabled
            } else if selected {
                color.neutral_background_1_selected
            } else if pressed && interactive {
                color.neutral_background_1_pressed
            } else if hovered && interactive {
                color.neutral_background_1_hover
            } else {
                color.neutral_background_1
            };
            (
                fill,
                color.transparent_stroke,
                Some(if interactive && hovered && enabled {
                    Elevation::E8
                } else {
                    Elevation::E4
                }),
            )
        }
        CardAppearance::FilledAlternative => {
            let fill = if !enabled {
                color.neutral_background_disabled
            } else if selected {
                color.neutral_background_2_selected
            } else if hovered && interactive {
                color.neutral_background_2_hover
            } else {
                color.neutral_background_2
            };
            (
                fill,
                color.transparent_stroke,
                Some(if interactive && hovered && enabled {
                    Elevation::E8
                } else {
                    Elevation::E4
                }),
            )
        }
        CardAppearance::Outline => (
            color.transparent_background,
            if selected {
                color.compound_brand_stroke
            } else {
                color.neutral_stroke_1
            },
            if interactive && hovered && enabled {
                Some(Elevation::E8)
            } else {
                None
            },
        ),
        CardAppearance::Subtle => {
            let fill = if pressed && interactive {
                color.subtle_background_pressed
            } else if hovered && interactive {
                color.subtle_background_hover
            } else if selected {
                color.subtle_background_selected
            } else {
                color.subtle_background
            };
            (
                fill,
                color.transparent_stroke,
                if interactive && hovered && enabled {
                    Some(Elevation::E8)
                } else {
                    None
                },
            )
        }
    }
}

fn space_or_enter(ui: &Ui) -> bool {
    ui.input_mut(|input| {
        input.consume_key(egui::Modifiers::NONE, egui::Key::Space)
            || input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
    })
}

fn finite_pos(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn rounded_u8(value: f32) -> u8 {
    if value.is_finite() {
        value.round().clamp(0.0, f32::from(u8::MAX)) as u8
    } else {
        0
    }
}
