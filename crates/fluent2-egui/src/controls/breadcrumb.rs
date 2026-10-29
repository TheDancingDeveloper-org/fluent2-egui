//! Fluent Breadcrumb: a path of items, each a button or a link, with the current item as plain
//! text. When the row is too narrow, the middle items fold into a "…" menu button.
//!
//! The current item is the last one unless [`Breadcrumb::current`] names another. It is not
//! interactive and is marked `aria-current="page"`. The other items are ordinary Tab stops. The
//! row is an AccessKit Navigation landmark named "Breadcrumb".

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Range;

use egui::accesskit::{AriaCurrent, Role};
use egui::{Id, Label, Response, RichText, Sense, TextStyle, TextWrapMode, Ui, Vec2, WidgetText};
use fluent2_icons::{Icon, Style as IconStyle};
use fluent2_tokens::generated::global::spacing_horizontal;

use super::button::{Appearance, Button, ButtonSize, Metrics};
use super::{Link, MenuButton, ensure_type_ramp};
use crate::foundation::icon::paint_icon;
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// Size of the breadcrumb buttons. [`Medium`](Self::Medium) is the Fluent default. Links keep the
/// body type ramp at every size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreadcrumbSize {
    /// Caption1 type in 24 px buttons.
    Small,
    /// Body1 type in 32 px buttons. The default.
    #[default]
    Medium,
    /// Subtitle2 type in 40 px buttons.
    Large,
}

impl BreadcrumbSize {
    fn button(self) -> ButtonSize {
        match self {
            Self::Small => ButtonSize::Small,
            Self::Medium => ButtonSize::Medium,
            Self::Large => ButtonSize::Large,
        }
    }

    fn chevron_px(self) -> f32 {
        match self {
            Self::Small | Self::Medium => 16.0,
            Self::Large => 20.0,
        }
    }
}

/// The glyph drawn between two items. [`Chevron`](Self::Chevron) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreadcrumbDivider {
    /// A right-pointing chevron.
    #[default]
    Chevron,
    /// A slash, as in a file path.
    Slash,
}

/// How the non-current items are drawn. [`Button`](Self::Button) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreadcrumbItemStyle {
    /// Transparent buttons that take the [`BreadcrumbSize`].
    #[default]
    Button,
    /// Text links, with no padding.
    Link,
}

/// What a Breadcrumb reported this frame.
#[derive(Debug)]
pub struct BreadcrumbResponse {
    /// The landmark around the row.
    pub response: Response,
    /// The index (in the order the items were given) of the item activated this frame. Items in
    /// the overflow menu count too.
    pub clicked: Option<usize>,
}

/// A breadcrumb trail. Build it, then call [`Breadcrumb::show`].
///
/// ```
/// # use fluent2_egui::controls::Breadcrumb;
/// # fn demo(ui: &mut egui::Ui) {
/// let trail = Breadcrumb::new("path", ["Home", "Documents", "Reports", "2026"]).show(ui);
/// if let Some(index) = trail.clicked {
///     // navigate to the item at `index`
/// }
/// # }
/// ```
pub struct Breadcrumb {
    id: Id,
    items: Vec<String>,
    current: Option<usize>,
    size: BreadcrumbSize,
    divider: BreadcrumbDivider,
    style: BreadcrumbItemStyle,
}

impl Breadcrumb {
    /// A breadcrumb over `items`, first (the root) to last. The last item is the current one.
    pub fn new<I, T>(id_salt: impl Hash + Debug, items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Self {
            id: Id::unique(id_salt),
            items: items.into_iter().map(Into::into).collect(),
            current: None,
            size: BreadcrumbSize::default(),
            divider: BreadcrumbDivider::default(),
            style: BreadcrumbItemStyle::default(),
        }
    }

    /// The current item, by index. Default: the last one. An index past the end leaves no item
    /// current.
    #[must_use]
    pub fn current(mut self, index: usize) -> Self {
        self.current = Some(index);
        self
    }

    /// Button size. Default medium.
    #[must_use]
    pub fn size(mut self, size: BreadcrumbSize) -> Self {
        self.size = size;
        self
    }

    /// The glyph between items. Default chevron.
    #[must_use]
    pub fn divider(mut self, divider: BreadcrumbDivider) -> Self {
        self.divider = divider;
        self
    }

    /// Draw the items as buttons or as links. Default buttons.
    #[must_use]
    pub fn item_style(mut self, style: BreadcrumbItemStyle) -> Self {
        self.style = style;
        self
    }

    fn current_index(&self) -> Option<usize> {
        let last = self.items.len().checked_sub(1)?;
        match self.current {
            None => Some(last),
            Some(index) => (index <= last).then_some(index),
        }
    }

    /// Draw the row. The middle items fold into a "…" menu when they do not fit the space left in
    /// the current `Ui`.
    pub fn show(self, ui: &mut Ui) -> BreadcrumbResponse {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let link = self.style == BreadcrumbItemStyle::Link;
        let metrics = Metrics::of(self.size.button()).at(crate::theme::density_of(ui));
        let style = if link {
            TextStyle::Name(TypeRamp::Body1.name().into())
        } else {
            metrics.text_style()
        };
        let pad = if link { 0.0 } else { metrics.h_padding };
        let current = self.current_index();

        let text_width = |ui: &Ui, text: &str| {
            WidgetText::from(text)
                .into_galley(ui, None, f32::INFINITY, style.clone())
                .size()
                .x
        };
        let widths: Vec<f32> = self
            .items
            .iter()
            .map(|text| text_width(ui, text) + 2.0 * pad)
            .collect();
        let gap = spacing_horizontal::XS;
        let glyph = match self.divider {
            BreadcrumbDivider::Chevron => self.size.chevron_px(),
            BreadcrumbDivider::Slash => text_width(ui, "/"),
        };
        let divider = glyph + 2.0 * gap;
        let overflow = text_width(ui, "…") + 2.0 * metrics.h_padding + metrics.icon + metrics.icon_gap;
        let slots = collapse(&widths, divider, overflow, ui.available_width(), current);

        let mut clicked = None;
        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (n, slot) in slots.iter().enumerate() {
                if n > 0 {
                    ui.add_space(gap);
                    self.paint_divider(ui, &theme, glyph);
                    ui.add_space(gap);
                }
                let picked = match slot {
                    Slot::Item(index) => {
                        self.item(ui, *index, current == Some(*index), style.clone(), pad, &theme)
                    }
                    Slot::Overflow(range) => self.overflow(ui, range.clone()),
                };
                clicked = picked.or(clicked);
            }
        });

        let response = inner.response;
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(Role::Navigation);
            node.set_label("Breadcrumb");
        });
        BreadcrumbResponse { response, clicked }
    }

    /// One item in the row: the current one as text, the others as a button or link.
    fn item(
        &self,
        ui: &mut Ui,
        index: usize,
        is_current: bool,
        style: TextStyle,
        pad: f32,
        theme: &Theme,
    ) -> Option<usize> {
        let label = self.items.get(index)?;
        if is_current {
            ui.add_space(pad);
            let color = c(theme.color.neutral_foreground_1);
            let text = RichText::new(label.as_str()).text_style(style).color(color);
            let response = ui.add(Label::new(text).wrap_mode(TextWrapMode::Extend));
            ui.add_space(pad);
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_aria_current(AriaCurrent::Page);
            });
            return None;
        }
        let clicked = match self.style {
            BreadcrumbItemStyle::Button => ui
                .add(
                    Button::new(label.as_str())
                        .appearance(Appearance::Transparent)
                        .size(self.size.button())
                        .min_width(0.0)
                        .id_salt((self.id, "item", index)),
                )
                .clicked(),
            BreadcrumbItemStyle::Link => ui.add(Link::new(label.as_str()).inline()).clicked(),
        };
        clicked.then_some(index)
    }

    /// The "…" menu holding the folded items in `range`. Returns the item picked this frame.
    fn overflow(&self, ui: &mut Ui, range: Range<usize>) -> Option<usize> {
        let (_, picked) = MenuButton::new("…")
            .appearance(Appearance::Transparent)
            .size(self.size.button())
            .id_salt((self.id, "overflow"))
            .show(ui, |ui| {
                let mut picked = None;
                for index in range {
                    let Some(label) = self.items.get(index) else {
                        continue;
                    };
                    let button = Button::new(label.as_str())
                        .appearance(Appearance::Transparent)
                        .size(self.size.button())
                        .min_width(0.0)
                        .id_salt((self.id, "hidden", index));
                    if ui.add(button).clicked() {
                        picked = Some(index);
                        ui.close();
                    }
                }
                picked
            });
        picked.flatten()
    }

    fn paint_divider(&self, ui: &mut Ui, theme: &Theme, glyph: f32) {
        let color = c(theme.color.neutral_foreground_3);
        match self.divider {
            BreadcrumbDivider::Chevron => {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(glyph), Sense::hover());
                let size = if glyph > 16.0 {
                    fluent2_icons::Size::S20
                } else {
                    fluent2_icons::Size::S16
                };
                paint_icon(
                    ui,
                    rect,
                    Icon::ChevronRight,
                    size,
                    IconStyle::Regular,
                    Some(color),
                );
            }
            BreadcrumbDivider::Slash => {
                let text = RichText::new("/")
                    .text_style(TextStyle::Name(TypeRamp::Body1.name().into()))
                    .color(color);
                ui.add(Label::new(text).wrap_mode(TextWrapMode::Extend));
            }
        }
    }
}

/// One slot of a laid-out breadcrumb row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// The item at this index.
    Item(usize),
    /// The items in this range, folded into the "…" menu.
    Overflow(Range<usize>),
}

/// Fold the middle items into one overflow slot when the row is wider than `available`.
///
/// The first item and the tail stay visible. The overflow takes as few items as fit, and it never
/// hides `current`. Widths are in points: `divider` is the width of one divider with its gaps, and
/// `overflow` the width of the "…" button.
pub(crate) fn collapse(
    widths: &[f32],
    divider: f32,
    overflow: f32,
    available: f32,
    current: Option<usize>,
) -> Vec<Slot> {
    let n = widths.len();
    let all = || (0..n).map(Slot::Item).collect::<Vec<_>>();
    if n <= 2 {
        return all();
    }
    let total: f32 = widths.iter().sum::<f32>() + divider * (n - 1) as f32;
    if total <= available {
        return all();
    }
    // Folding `1..fold` hides `current` when it sits in that range, so `fold` may not pass it.
    let limit = match current {
        Some(c) if (1..n).contains(&c) => c,
        _ => n - 1,
    };
    let width = |fold: usize| {
        widths[0]
            + 2.0 * divider
            + overflow
            + widths[fold..].iter().sum::<f32>()
            + divider * (n - fold - 1) as f32
    };
    let fold = (2..=limit).find(|&fold| width(fold) <= available);
    let fold = match fold {
        Some(fold) => fold,
        None if limit >= 2 => limit,
        None => return all(),
    };
    let mut slots = vec![Slot::Item(0), Slot::Overflow(1..fold)];
    slots.extend((fold..n).map(Slot::Item));
    slots
}

#[cfg(test)]
mod tests {
    use super::{Slot, collapse};

    fn visible(slots: &[Slot]) -> Vec<usize> {
        let mut out = Vec::new();
        for slot in slots {
            match slot {
                Slot::Item(i) => out.push(*i),
                Slot::Overflow(range) => out.extend(range.clone()),
            }
        }
        out
    }

    #[test]
    fn fits_or_too_short_to_fold_shows_everything() {
        let widths = [10.0; 5];
        assert_eq!(collapse(&widths, 5.0, 20.0, 1000.0, Some(4)).len(), 5);
        assert_eq!(collapse(&[10.0, 10.0], 5.0, 20.0, 1.0, Some(1)).len(), 2);
        assert!(collapse(&[], 5.0, 20.0, 1.0, None).is_empty());
    }

    #[test]
    fn folds_the_fewest_middle_items_that_fit() {
        // Full row: 50 + 4 * 5 = 70. Folding 1..4 leaves 10 + 10 + 20 + 10 = 50 <= 60.
        let widths = [10.0; 5];
        assert_eq!(
            collapse(&widths, 5.0, 20.0, 60.0, Some(4)),
            vec![Slot::Item(0), Slot::Overflow(1..4), Slot::Item(4)]
        );
    }

    #[test]
    fn never_folds_the_current_item() {
        let widths = [10.0; 5];
        // Folding 1..2 keeps item 2 visible, but a fold to 3 would hide it.
        assert_eq!(
            collapse(&widths, 5.0, 20.0, 60.0, Some(2)),
            vec![
                Slot::Item(0),
                Slot::Overflow(1..2),
                Slot::Item(2),
                Slot::Item(3),
                Slot::Item(4)
            ]
        );
        // Current at index 1 cannot be folded at all, so nothing folds.
        assert_eq!(collapse(&widths, 5.0, 20.0, 60.0, Some(1)).len(), 5);
    }

    #[test]
    fn every_item_is_shown_or_folded_exactly_once() {
        for n in 0usize..9 {
            let widths: Vec<f32> = (0..n).map(|i| 8.0 + i as f32 * 3.0).collect();
            for available in [0.0, 30.0, 60.0, 120.0, 400.0, f32::INFINITY, f32::NAN] {
                for current in [None, Some(0), Some(n / 2), Some(n.saturating_sub(1)), Some(99)] {
                    let slots = collapse(&widths, 9.0, 24.0, available, current);
                    let seen = visible(&slots);
                    assert_eq!(seen, (0..n).collect::<Vec<_>>(), "n {n} available {available}");
                    if n > 0 {
                        assert_eq!(slots.first(), Some(&Slot::Item(0)));
                        assert_eq!(slots.last(), Some(&Slot::Item(n - 1)));
                    }
                    if let Some(c) = current.filter(|&c| c < n) {
                        assert!(seen.contains(&c));
                    }
                }
            }
        }
    }
}
