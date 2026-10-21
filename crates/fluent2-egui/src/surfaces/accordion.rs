//! Fluent accordion: a stack of headers, each showing or hiding a panel.
//!
//! The Fluent reference is the React v9 Accordion at
//! <https://github.com/microsoft/fluentui/tree/990e2c65adab04fccc1f9ace3ecae5487ba94348/packages/react-components/react-accordion>.
//! Headers are buttons (Enter or Space toggles). The panel height animates with
//! `durationNormal` on `curveEasyEase` and snaps under reduced motion. Each header is an
//! AccessKit Button with its expanded state that controls its panel region.

use egui::{
    CornerRadius, FontId, FontSelection, Id, Rect, Sense, Ui, UiBuilder, Vec2, WidgetInfo, WidgetText,
    WidgetType, accesskit,
};

use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, font_size, spacing_horizontal};

use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::paint_icon;
use crate::foundation::motion::{self, Curve, Duration};
use crate::theme::{Theme, c};

/// Header size, from Fluent's `size` prop.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccordionSize {
    /// `fontSizeBase200`, 32 points tall, 16-point chevron.
    Small,
    /// `fontSizeBase300`, 44 points tall. The default.
    #[default]
    Medium,
    /// `fontSizeBase400`, 44 points tall.
    Large,
    /// `fontSizeBase500`, 44 points tall.
    ExtraLarge,
}

impl AccordionSize {
    fn font_size(self) -> f32 {
        match self {
            Self::Small => font_size::BASE_200,
            Self::Medium => font_size::BASE_300,
            Self::Large => font_size::BASE_400,
            Self::ExtraLarge => font_size::BASE_500,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::Small => 32.0,
            _ => 44.0,
        }
    }

    fn icon(self) -> IconSize {
        match self {
            Self::Small => IconSize::S16,
            _ => IconSize::S20,
        }
    }
}

/// Where the expand chevron sits in a header.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExpandIconPosition {
    /// Before the header text. The default.
    #[default]
    Start,
    /// At the far end of the header.
    End,
}

/// What [`Accordion::show`] reports.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AccordionResponse {
    /// The item whose open state changed this frame, if any.
    pub toggled: Option<usize>,
}

/// A panel body, run with the panel's `Ui`.
type PanelBody<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// One accordion item: a header and its panel.
pub struct AccordionItem<'a> {
    header: WidgetText,
    panel: PanelBody<'a>,
    disabled: bool,
}

impl<'a> AccordionItem<'a> {
    /// An item with a header and a panel body.
    pub fn new(header: impl Into<WidgetText>, panel: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            header: header.into(),
            panel: Box::new(panel),
            disabled: false,
        }
    }

    /// Disable the header so it cannot be toggled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A Fluent accordion. The app owns the open item indices.
///
/// ```
/// # use fluent2_egui::surfaces::{Accordion, AccordionItem};
/// # fn demo(ui: &mut egui::Ui, open: &mut Vec<usize>) {
/// let _ = Accordion::new("settings", open)
///     .item(AccordionItem::new("General", |ui| { ui.label("General settings"); }))
///     .item(AccordionItem::new("Advanced", |ui| { ui.label("Advanced settings"); }))
///     .show(ui);
/// # }
/// ```
pub struct Accordion<'a> {
    id: Id,
    open: &'a mut Vec<usize>,
    items: Vec<AccordionItem<'a>>,
    size: AccordionSize,
    icon_position: ExpandIconPosition,
    multiple: bool,
    collapsible: bool,
}

impl<'a> Accordion<'a> {
    /// An accordion keyed by `id_salt`, with the open item indices in `open`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, open: &'a mut Vec<usize>) -> Self {
        Self {
            id: Id::new(id_salt).with("fluent2_accordion"),
            open,
            items: Vec::new(),
            size: AccordionSize::Medium,
            icon_position: ExpandIconPosition::Start,
            multiple: false,
            collapsible: false,
        }
    }

    /// Add an item.
    #[must_use]
    pub fn item(mut self, item: AccordionItem<'a>) -> Self {
        self.items.push(item);
        self
    }

    /// Header size. Defaults to [`AccordionSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: AccordionSize) -> Self {
        self.size = size;
        self
    }

    /// Chevron position. Defaults to [`ExpandIconPosition::Start`].
    #[must_use]
    pub fn expand_icon_position(mut self, position: ExpandIconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Let several items be open at once. Off by default: opening one closes the others.
    #[must_use]
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Let the last open item be closed. Off by default, as in Fluent.
    #[must_use]
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Show the accordion.
    pub fn show(self, ui: &mut Ui) -> AccordionResponse {
        let Accordion {
            id,
            open,
            items,
            size,
            icon_position,
            multiple,
            collapsible,
        } = self;
        let count = items.len();
        open.retain(|index| *index < count);
        open.sort_unstable();
        open.dedup();
        let mut toggled = None;
        ui.vertical(|ui| {
            for (index, item) in items.into_iter().enumerate() {
                let item_id = id.with(index);
                let is_open = open.contains(&index);
                if header(ui, item_id, &item, is_open, size, icon_position)
                    && toggle(open, index, multiple, collapsible)
                {
                    toggled = Some(index);
                }
                panel(ui, item_id, item.panel, open.contains(&index));
            }
        });
        AccordionResponse { toggled }
    }
}

/// Flip `index` in `open` under the accordion's rules. Returns whether anything changed.
fn toggle(open: &mut Vec<usize>, index: usize, multiple: bool, collapsible: bool) -> bool {
    if let Some(at) = open.iter().position(|i| *i == index) {
        if !collapsible && open.len() == 1 {
            return false;
        }
        open.remove(at);
    } else {
        if !multiple {
            open.clear();
        }
        open.push(index);
        open.sort_unstable();
    }
    true
}

/// Paint one header. Returns whether it was activated this frame.
fn header(
    ui: &mut Ui,
    id: Id,
    item: &AccordionItem<'_>,
    is_open: bool,
    size: AccordionSize,
    icon_position: ExpandIconPosition,
) -> bool {
    let theme = Theme::get(ui.ctx());
    let enabled = ui.is_enabled() && !item.disabled;
    let header_id = id.with("header");
    let width = finite_pos(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, size.height()), Sense::hover());
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let mut response = ui.interact(rect, header_id, sense);
    let activated = enabled && (response.clicked() || (response.has_focus() && space_or_enter(ui)));
    let expanded = is_open != activated;
    if activated {
        response.mark_changed();
    }

    let text = item.header.text().to_owned();
    if ui.is_rect_visible(rect) {
        let color = c(if enabled {
            theme.color.neutral_foreground_1
        } else {
            theme.color.neutral_foreground_disabled
        });
        let icon_points = match size.icon() {
            IconSize::S16 => 16.0,
            _ => 20.0,
        };
        let pad = spacing_horizontal::M;
        let gap = spacing_horizontal::S;
        let icon_rect = match icon_position {
            ExpandIconPosition::Start => Rect::from_center_size(
                egui::pos2(rect.left() + pad + icon_points / 2.0, rect.center().y),
                Vec2::splat(icon_points),
            ),
            ExpandIconPosition::End => Rect::from_center_size(
                egui::pos2(rect.right() - pad - icon_points / 2.0, rect.center().y),
                Vec2::splat(icon_points),
            ),
        };
        let chevron = if expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        paint_icon(
            ui,
            icon_rect,
            chevron,
            size.icon(),
            IconStyle::Regular,
            Some(color),
        );
        let text_left = match icon_position {
            ExpandIconPosition::Start => icon_rect.right() + gap,
            ExpandIconPosition::End => rect.left() + pad,
        };
        let text_right = match icon_position {
            ExpandIconPosition::Start => rect.right() - pad,
            ExpandIconPosition::End => icon_rect.left() - gap,
        };
        let galley = item.header.clone().into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (text_right - text_left).max(0.0),
            FontSelection::FontId(FontId::proportional(size.font_size())),
        );
        let pos = egui::pos2(text_left, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
        if focus::focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }
    }

    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, text.clone()));
    let panel_node = panel_ui_id(id);
    ui.ctx().accesskit_node_builder(header_id, |node| {
        node.set_role(accesskit::Role::Button);
        node.set_expanded(expanded);
        if expanded {
            node.push_controlled(panel_node.accesskit_id());
        }
    });
    activated
}

fn panel_ui_id(item_id: Id) -> Id {
    item_id.with("panel")
}

/// Show the panel, clipped to its animated height.
fn panel(ui: &mut Ui, item_id: Id, body: PanelBody<'_>, is_open: bool) {
    let ctx = ui.ctx().clone();
    let progress = motion::animate(
        &ctx,
        item_id.with("height"),
        if is_open { 1.0 } else { 0.0 },
        Duration::Normal,
        Curve::EasyEase,
    )
    .clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    let measured_id = item_id.with("measured");
    let width = finite_pos(ui.available_width());
    let top = ui.cursor().min;
    let pad = spacing_horizontal::M;
    let mut child = ui.new_child(
        UiBuilder::new()
            .id(panel_ui_id(item_id))
            .max_rect(Rect::from_min_size(
                egui::pos2(top.x + pad, top.y),
                Vec2::new((width - pad * 2.0).max(0.0), f32::INFINITY),
            )),
    );
    let measured: f32 = ctx.data(|data| data.get_temp(measured_id)).unwrap_or(0.0);
    let visible = if progress >= 1.0 {
        None
    } else {
        Some(measured * progress)
    };
    if let Some(height) = visible {
        let clip = Rect::from_min_size(top, Vec2::new(width, height));
        child.set_clip_rect(child.clip_rect().intersect(clip));
    }
    body(&mut child);
    let full = child.min_rect().height().max(0.0);
    ctx.data_mut(|data| data.insert_temp(measured_id, full));
    ctx.accesskit_node_builder(child.id(), |node| {
        node.set_role(accesskit::Role::Region);
    });
    let height = visible.unwrap_or(full).min(full);
    ui.advance_cursor_after_rect(Rect::from_min_size(top, Vec2::new(width, height)));
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

#[cfg(test)]
mod tests {
    use super::toggle;

    #[test]
    fn single_mode_closes_the_others() {
        let mut open = vec![0];
        assert!(toggle(&mut open, 2, false, false));
        assert_eq!(open, vec![2]);
    }

    #[test]
    fn multiple_mode_keeps_the_others() {
        let mut open = vec![0];
        assert!(toggle(&mut open, 2, true, false));
        assert_eq!(open, vec![0, 2]);
        assert!(toggle(&mut open, 0, true, false));
        assert_eq!(open, vec![2]);
    }

    #[test]
    fn the_last_item_stays_open_unless_collapsible() {
        let mut open = vec![1];
        assert!(!toggle(&mut open, 1, false, false));
        assert_eq!(open, vec![1]);
        assert!(toggle(&mut open, 1, false, true));
        assert!(open.is_empty());
    }
}
