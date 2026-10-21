//! Backstage: the full-window File view (spec §7, WI-1568).
//!
//! The app shows this instead of the ribbon and content while `BackstageState::open` is true.
//! A left navigation column lists the pages; the page area draws the selected page's closure.
//! The component holds no file logic. Escape and the back arrow close it and return focus to
//! the control that had it when it opened (spec §7). Focus is captured on the frame `open`
//! turns true, so the File tab needs no special API.

use egui::{
    Align2, Context, CornerRadius, Frame, Id, Key, Modifiers, Rect, Response, Sense, Ui, Vec2, WidgetInfo,
    Role, accesskit,
};

use fluent2_icons::Icon;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical};

use crate::controls::{Appearance, Button, Divider, ensure_type_ramp};
use crate::foundation::focus::paint_focus_ring;
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, ThemeKind, c};

/// Width of the navigation column, in points (spec §7, `backstage_nav_width`).
///
/// Private until the chrome metrics decision (spec D2, §11) moves it onto `Theme`.
const NAV_WIDTH_PT: f32 = 240.0;
/// Height of one page entry, in points (spec §7).
const ROW_HEIGHT_PT: f32 = 40.0;
/// Width of the brand bar on the selected entry, in points (spec §7).
const SELECTED_BAR_PT: f32 = 3.0;
/// Padding around the page area, in points (spec §7).
const PAGE_PADDING_PT: f32 = 24.0;

/// Backstage state. The app owns it and persists it if it wants to (spec D3).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BackstageState {
    /// True while the backstage is shown. The File tab sets it; Escape and the back arrow clear it.
    pub open: bool,
    /// Active page key. `None` means the first page.
    pub page: Option<&'static str>,
}

/// What the backstage did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BackstageResponse {
    /// True on the frame the backstage was closed by the back arrow or Escape.
    pub closed: bool,
    /// The page that became active this frame, if the user changed it.
    pub page_changed: Option<&'static str>,
}

/// The closure that draws a page's content.
type PageBody<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// One navigation entry and the closure that draws its page.
struct Page<'a> {
    key: &'static str,
    label: String,
    add: Option<PageBody<'a>>,
}

/// The backstage view. Build it with [`Backstage::new`], add pages, then [`Backstage::show`].
///
/// ```
/// # use fluent2_egui::office::backstage::{Backstage, BackstageState};
/// # fn demo(ui: &mut egui::Ui, state: &mut BackstageState) {
/// Backstage::new(state)
///     .page("home", "Home", |ui| {
///         ui.label("Home page");
///     })
///     .bottom_page("options", "Options", |ui| {
///         ui.label("Options page");
///     })
///     .show(ui);
/// # }
/// ```
pub struct Backstage<'a> {
    state: &'a mut BackstageState,
    pages: Vec<Page<'a>>,
    bottom: Vec<Page<'a>>,
}

impl<'a> Backstage<'a> {
    /// A backstage whose open flag and selected page live in `state`.
    pub fn new(state: &'a mut BackstageState) -> Self {
        Self {
            state,
            pages: Vec::new(),
            bottom: Vec::new(),
        }
    }

    /// A page in the main group. `add` draws its content under the page heading.
    #[must_use]
    pub fn page(mut self, key: &'static str, label: &str, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.pages.push(Page {
            key,
            label: label.to_owned(),
            add: Some(Box::new(add)),
        });
        self
    }

    /// A page in the bottom group, below a divider (for example Options).
    #[must_use]
    pub fn bottom_page(mut self, key: &'static str, label: &str, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.bottom.push(Page {
            key,
            label: label.to_owned(),
            add: Some(Box::new(add)),
        });
        self
    }

    /// Show the backstage in `ui`'s area. Draws nothing and returns the default response while
    /// `open` is false.
    pub fn show(self, ui: &mut Ui) -> BackstageResponse {
        let Self {
            state,
            mut pages,
            mut bottom,
        } = self;
        let id = ui.make_persistent_id("backstage");
        let ctx = ui.ctx().clone();
        let opener_key = id.with("opener");
        let seen_key = id.with("seen");
        let text_focus_key = id.with("text_focus");
        let pending_key = id.with("pending");
        let focus_in_key = id.with("focus_in");

        if !state.open {
            // Closed by the app: forget the opening, then give focus back to the opener once it
            // is drawn again.
            if ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false)) {
                ctx.data_mut(|data| {
                    data.insert_temp(seen_key, false);
                    data.insert_temp(pending_key, true);
                });
            }
            restore_focus(&ctx, opener_key, pending_key);
            return BackstageResponse::default();
        }

        let opening = !ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false));
        if opening {
            let previous = ctx.memory(|memory| memory.focused());
            ctx.data_mut(|data| {
                data.insert_temp(opener_key, previous);
                data.insert_temp(seen_key, true);
                data.insert_temp(focus_in_key, FOCUS_IN_FRAMES);
            });
        }

        ensure_type_ramp(ui);
        let theme = Theme::get(&ctx);
        let keys: Vec<&'static str> = pages.iter().chain(bottom.iter()).map(|page| page.key).collect();
        let mut page_changed = None;

        // Escape closes unless a text field has the keyboard, which keeps Escape for itself. egui
        // releases focus on Escape before this frame's widgets run, so the field's focus is read
        // from the end of the previous frame as well.
        let text_focused = ctx.text_edit_focused()
            || ctx.data(|data| data.get_temp::<bool>(text_focus_key).unwrap_or(false));
        let escape = !text_focused && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        if escape {
            return close(state, &ctx, seen_key, pending_key);
        }

        // The entries are focusable, so egui moves focus between them with the arrow keys. The
        // page follows the focused entry, the way a tab list selects the tab it moves to.
        let focused = ctx.memory(|memory| memory.focused());
        if let Some(key) = keys.iter().copied().find(|key| Some(row_id(id, key)) == focused)
            && Some(key) != selected_key(state.page, &keys)
        {
            state.page = Some(key);
            page_changed = Some(key);
        }

        let selected = selected_key(state.page, &keys);
        let mut back_clicked = false;
        let mut clicked_key = None;

        Frame::new()
            .fill(c(theme.color.neutral_background_1))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(NAV_WIDTH_PT);
                        ui.spacing_mut().item_spacing.y = spacing_vertical::XS;
                        let back =
                            ui.add(Button::icon_only(Icon::ArrowLeft, "Back").appearance(Appearance::Subtle));
                        back_clicked = back.clicked();
                        for page in &pages {
                            let row =
                                nav_row(ui, row_id(id, page.key), &page.label, Some(page.key) == selected);
                            if row.clicked() {
                                clicked_key = Some(page.key);
                            }
                        }
                        ui.add_space(spacing_vertical::M);
                        if !bottom.is_empty() {
                            ui.add(Divider::horizontal());
                            for page in &bottom {
                                let row = nav_row(
                                    ui,
                                    row_id(id, page.key),
                                    &page.label,
                                    Some(page.key) == selected,
                                );
                                if row.clicked() {
                                    clicked_key = Some(page.key);
                                }
                            }
                        }
                    });
                    ui.vertical(|ui| {
                        Frame::new().inner_margin(PAGE_PADDING_PT).show(ui, |ui| {
                            let page = pages
                                .iter_mut()
                                .chain(bottom.iter_mut())
                                .find(|page| Some(page.key) == selected);
                            if let Some(page) = page {
                                ui.label(text::rich(page.label.clone(), TypeRamp::Title2));
                                if let Some(add) = page.add.take() {
                                    add(ui);
                                }
                            }
                        });
                    });
                });
            });

        // Focus moves into the backstage on open, to the selected entry (WI-1850). egui drops a
        // focus request for a widget that was not interactive last frame, and the entries are
        // new on the opening frame, so the request repeats until it takes, for a few frames.
        let focus_in = ctx.data(|data| data.get_temp::<u8>(focus_in_key)).unwrap_or(0);
        if focus_in > 0
            && let Some(key) = selected
        {
            let target = row_id(id, key);
            if ctx.memory(|memory| memory.has_focus(target)) {
                ctx.data_mut(|data| data.insert_temp(focus_in_key, 0_u8));
            } else {
                ctx.memory_mut(|memory| memory.request_focus(target));
                ctx.data_mut(|data| data.insert_temp(focus_in_key, focus_in - 1));
            }
        }

        // The backstage region, for assistive technology.
        let region = ui.interact(ui.min_rect(), id.with("region"), Sense::hover());
        ctx.accesskit_node_builder(region.id, |node| {
            node.set_role(accesskit::Role::Dialog);
            node.set_label("Backstage");
        });

        if back_clicked {
            return close(state, &ctx, seen_key, pending_key);
        }
        if let Some(key) = clicked_key
            && Some(key) != selected
        {
            state.page = Some(key);
            page_changed = Some(key);
        }
        let text_focused_now = ctx.text_edit_focused();
        ctx.data_mut(|data| data.insert_temp(text_focus_key, text_focused_now));
        BackstageResponse {
            closed: false,
            page_changed,
        }
    }
}

/// How many frames the backstage keeps asking for focus on its selected entry after opening.
const FOCUS_IN_FRAMES: u8 = 3;

/// Close the backstage: clear `open`, give focus back to the opener, and report it.
fn close(state: &mut BackstageState, ctx: &Context, seen_key: Id, pending_key: Id) -> BackstageResponse {
    state.open = false;
    ctx.data_mut(|data| {
        data.insert_temp(seen_key, false);
        data.insert_temp(pending_key, true);
    });
    BackstageResponse {
        closed: true,
        page_changed: None,
    }
}

/// The key of the page to show: `wanted` when it names a page, otherwise the first page.
fn selected_key(wanted: Option<&'static str>, keys: &[&'static str]) -> Option<&'static str> {
    wanted
        .filter(|key| keys.contains(key))
        .or_else(|| keys.first().copied())
}

/// Stable widget id of one navigation entry, from its page key (never from its label).
fn row_id(backstage: Id, key: &'static str) -> Id {
    backstage.with(("page", key))
}

/// One navigation entry: 40 pt, with the selected fill and brand bar when selected.
fn nav_row(ui: &mut Ui, id: Id, label: &str, selected: bool) -> Response {
    let theme = Theme::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT_PT), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, radius, c(theme.color.subtle_background_selected));
        let bar = Rect::from_min_size(rect.left_top(), Vec2::new(SELECTED_BAR_PT, rect.height()));
        // High contrast fills the row with the highlight colour, which the brand foreground
        // matches; the bar takes the highlight's own foreground there.
        let bar_color = if theme.kind == ThemeKind::HighContrast {
            theme.color.neutral_foreground_1_selected
        } else {
            theme.color.brand_foreground_1
        };
        painter.rect_filled(bar, CornerRadius::ZERO, c(bar_color));
    } else if response.hovered() {
        painter.rect_filled(rect, radius, c(theme.color.subtle_background_hover));
    }
    let text_color = if selected {
        theme.color.neutral_foreground_1_selected
    } else if response.hovered() {
        theme.color.neutral_foreground_1_hover
    } else {
        theme.color.neutral_foreground_1
    };
    let text_pos = rect.left_center() + Vec2::new(spacing_horizontal::L, 0.0);
    painter.text(
        text_pos,
        Align2::LEFT_CENTER,
        label,
        text::font_id(TypeRamp::Body1),
        c(text_color),
    );
    if response.has_focus() {
        paint_focus_ring(ui, rect, radius);
    }
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, label));
    response
}

/// Give focus back to the widget that had it when the backstage opened, once that widget is drawn.
///
/// The opener is usually hidden while the backstage is open, so the request waits until the
/// opener is drawn this pass. Requesting focus for a widget that is not in this pass's tree
/// would leave focus on a node that does not exist. The opener must therefore be drawn before
/// [`Backstage::show`] in the frame that restores it.
fn restore_focus(ctx: &Context, opener_key: Id, pending_key: Id) {
    if !ctx.data(|data| data.get_temp::<bool>(pending_key).unwrap_or(false)) {
        return;
    }
    let opener = ctx.data(|data| data.get_temp::<Option<Id>>(opener_key)).flatten();
    match opener {
        None => {
            ctx.data_mut(|data| data.insert_temp(pending_key, false));
        }
        Some(opener) if ctx.read_response(opener).is_some() => {
            ctx.memory_mut(|memory| memory.request_focus(opener));
            ctx.data_mut(|data| data.insert_temp(pending_key, false));
        }
        Some(_) => {}
    }
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}
