//! Ribbon gallery (spec §4.4): an inline row of tiles with scroll and expand buttons at its
//! right edge, and an expanded popup grid with optional category headers and footer items.
//!
//! The caller draws each tile's content with a closure; the gallery paints the tile
//! background, the hover, pressed and selected states, and the focus ring. The popup lays out
//! only the rows that are on screen, so a gallery of thousands of items stays cheap.

use std::hash::Hash;
use std::ops::Range;

use egui::{
    CornerRadius, EventFilter, Id, Key, Modifiers, Popup, PopupCloseBehavior, Rect, Response, ScrollArea,
    Sense, Stroke, StrokeKind, TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText, WidgetType,
};
use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::paint_icon;
use crate::foundation::text::{self, TypeRamp};
use crate::surfaces::{divider, menu_frame};
use crate::theme::{Theme, c};

/// Tile side at the Large step (§4.4).
const TILE_LARGE: f32 = 48.0;
/// Tile side at the Medium step (§4.4).
const TILE_MEDIUM: f32 = 32.0;
/// Gap between tiles (§10, `spacing_horizontal::XS`).
const GAP: f32 = spacing_horizontal::XS;
/// Height of a category header row in the popup.
const HEADER_HEIGHT: f32 = 24.0;
/// Tile rows the popup shows before it scrolls.
const POPUP_VISIBLE_ROWS: f32 = 6.0;

/// Tile size of a gallery, following the group's scale step (§4.4, §5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GalleryTileSize {
    /// 48 × 48 pt tiles.
    #[default]
    Large,
    /// 32 × 32 pt tiles.
    Medium,
}

impl GalleryTileSize {
    /// Side of one tile, in points.
    #[must_use]
    pub fn side(self) -> f32 {
        match self {
            Self::Large => TILE_LARGE,
            Self::Medium => TILE_MEDIUM,
        }
    }
}

/// How a tile is being drawn, passed to the caller's draw closure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GalleryItemState {
    /// The item is the bound selection.
    pub selected: bool,
    /// The pointer is over the tile.
    pub hovered: bool,
    /// The tile is held down.
    pub pressed: bool,
    /// The tile has keyboard focus (inline) or is the keyboard cursor (popup).
    pub focused: bool,
    /// The gallery is enabled.
    pub enabled: bool,
}

/// What a gallery did this frame.
#[derive(Debug)]
pub struct GalleryResponse {
    /// The inline row, including its buttons.
    pub response: Response,
    /// The selection changed this frame. The new index is in the bound `selected`.
    pub changed: bool,
    /// Index of the first item shown in the inline row.
    pub first_visible: usize,
    /// Whether the expanded popup is open after this frame.
    pub popup_open: bool,
}

/// Gallery state kept across frames in egui's memory.
#[derive(Clone, Copy, Debug, Default)]
struct Memory {
    /// First inline row.
    row: usize,
    /// Keyboard cursor in the popup.
    cursor: usize,
    /// The popup should scroll the cursor into view next frame.
    reveal: bool,
    /// The grid should take keyboard focus once it is laid out visibly. The popup's first
    /// frame is an invisible sizing pass, and focus given then would be dropped.
    focus_grid: bool,
}

struct Category {
    label: WidgetText,
    first: usize,
}

/// One line of the popup: a category header or a row of tiles.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    Header(usize),
    Row(Range<usize>),
}

/// Accessible label of an item, by index.
type ItemLabel<'a> = Box<dyn Fn(usize) -> String + 'a>;
/// Footer contents of the popup.
type Footer<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A ribbon gallery bound to `selected` (spec §4.4).
///
/// ```
/// # use fluent2_egui::office::ribbon::Gallery;
/// # fn demo(ui: &mut egui::Ui, style: &mut Option<usize>) {
/// let names = ["Normal", "Heading 1", "Heading 2", "Title"];
/// let response = Gallery::new("styles", "Styles", names.len(), style)
///     .item_label(|index| names.get(index).copied().unwrap_or_default().to_owned())
///     .show(ui, |ui, index, rect, _state| {
///         let name = names.get(index).copied().unwrap_or_default();
///         ui.painter().text(
///             rect.center(),
///             egui::Align2::CENTER_CENTER,
///             name,
///             egui::FontId::proportional(10.0),
///             ui.visuals().text_color(),
///         );
///     });
/// if response.changed {
///     // the app applies the chosen style
/// }
/// # }
/// ```
pub struct Gallery<'a> {
    id: Id,
    label: String,
    count: usize,
    selected: &'a mut Option<usize>,
    columns: Option<usize>,
    tile: GalleryTileSize,
    item_label: Option<ItemLabel<'a>>,
    categories: Vec<Category>,
    footer: Option<Footer<'a>>,
    popup_width: Option<f32>,
}

impl<'a> Gallery<'a> {
    /// A gallery of `count` items. `label` names it for assistive technology.
    pub fn new(
        id: impl Hash + std::fmt::Debug,
        label: impl Into<String>,
        count: usize,
        selected: &'a mut Option<usize>,
    ) -> Self {
        Self {
            id: Id::new(id),
            label: label.into(),
            count,
            selected,
            columns: None,
            tile: GalleryTileSize::default(),
            item_label: None,
            categories: Vec::new(),
            footer: None,
            popup_width: None,
        }
    }

    /// Tiles shown in the inline row. Default: as many as fit the available width.
    #[must_use]
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns.max(1));
        self
    }

    /// Tile size, from the group's scale step. Default: [`GalleryTileSize::Large`].
    #[must_use]
    pub fn tile_size(mut self, tile: GalleryTileSize) -> Self {
        self.tile = tile;
        self
    }

    /// Accessible label of each item. Default: "\<gallery label\> \<n\>".
    #[must_use]
    pub fn item_label(mut self, label: impl Fn(usize) -> String + 'a) -> Self {
        self.item_label = Some(Box::new(label));
        self
    }

    /// A category header in the popup, shown above item `first`. Add them in item order.
    #[must_use]
    pub fn category(mut self, label: impl Into<WidgetText>, first: usize) -> Self {
        self.categories.push(Category {
            label: label.into(),
            first,
        });
        self
    }

    /// Menu items below the popup grid, for example "Save selection as a new style". Draw
    /// them with [`crate::surfaces::MenuItem`]. A click in the footer closes the popup.
    #[must_use]
    pub fn footer(mut self, footer: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.footer = Some(Box::new(footer));
        self
    }

    /// Popup width. Default: the width of the inline row. Inside a ribbon group this is the
    /// group's width (§4.4).
    #[must_use]
    pub fn popup_width(mut self, width: f32) -> Self {
        self.popup_width = Some(width);
        self
    }

    /// Show the gallery. `draw` paints item `index` into `rect`.
    pub fn show(
        mut self,
        ui: &mut Ui,
        draw: impl Fn(&mut Ui, usize, Rect, GalleryItemState),
    ) -> GalleryResponse {
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let side = self.tile.side();
        let button_width = theme.chrome.icon_small;
        let count = self.count;
        let memory_id = self.id.with("memory");
        let mut memory: Memory = ui.ctx().data(|data| data.get_temp(memory_id)).unwrap_or_default();
        let mut changed = false;

        let columns = self.columns.unwrap_or_else(|| {
            let free = finite(ui.available_width() - button_width - GAP);
            // Truncation is intended: only whole tiles fit.
            ((free + GAP) / (side + GAP)).floor().max(1.0) as usize
        });
        let rows = count.div_ceil(columns).max(1);
        memory.row = memory.row.min(rows - 1);

        let row_width = columns as f32 * (side + GAP) + button_width;
        let height = side + 2.0 * GAP;
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(row_width, height), Sense::hover());
        let radius = CornerRadius::from(border_radius::MEDIUM);
        if ui.is_rect_visible(rect) {
            ui.painter().rect(
                rect,
                radius,
                c(theme.color.neutral_background_1),
                Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1)),
                StrokeKind::Inside,
            );
        }

        // Inline tiles.
        let first = memory.row * columns;
        for slot in 0..columns {
            let index = first + slot;
            if index >= count {
                break;
            }
            let min = rect.min + Vec2::new(GAP + slot as f32 * (side + GAP), GAP);
            let tile = Rect::from_min_size(min, Vec2::splat(side));
            let tile_response = ui.interact(tile, self.id.with(("inline", index)), Sense::click());
            let state = item_state(
                &tile_response,
                *self.selected == Some(index),
                focus_visible(ui, &tile_response),
                enabled,
            );
            paint_tile(ui, &theme, tile, state);
            draw_tile(ui, &theme, state, |ui| draw(ui, index, tile, state));
            if state.focused {
                paint_focus_ring(ui, tile, radius);
            }
            let label = self.label_of(index);
            let selected = state.selected;
            tile_response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, &label));
            if tile_response.clicked() && *self.selected != Some(index) {
                *self.selected = Some(index);
                changed = true;
            }
            response = response.union(tile_response);
        }

        // Up, down and expand buttons, stacked at the right edge.
        let column = Rect::from_min_max(egui::pos2(rect.right() - button_width, rect.top()), rect.max);
        let third = column.height() / 3.0;
        let part = |n: f32| {
            Rect::from_min_size(
                column.min + Vec2::new(0.0, n * third),
                Vec2::new(column.width(), third),
            )
        };
        let up = ui.add_enabled_ui(memory.row > 0, |ui| {
            edge_button(
                ui,
                &theme,
                part(0.0),
                self.id.with("up"),
                Icon::ChevronUp,
                &format!("Previous {} row", self.label),
            )
        });
        if up.inner.clicked() {
            memory.row = memory.row.saturating_sub(1);
        }
        let down = ui.add_enabled_ui(memory.row + 1 < rows, |ui| {
            edge_button(
                ui,
                &theme,
                part(1.0),
                self.id.with("down"),
                Icon::ChevronDown,
                &format!("Next {} row", self.label),
            )
        });
        if down.inner.clicked() {
            memory.row = (memory.row + 1).min(rows - 1);
        }
        let expand = edge_button(
            ui,
            &theme,
            part(2.0),
            self.id.with("expand"),
            Icon::ChevronDown,
            &format!("More {}", self.label),
        );
        response = response.union(up.inner).union(down.inner);

        // The popup grid.
        let popup_id = self.id.with("popup");
        let grid_id = self.id.with("grid");
        let was_open = Popup::is_id_open(ui.ctx(), popup_id);
        let open_key = !was_open
            && expand.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(Modifiers::NONE, Key::Enter)
                    || input.consume_key(Modifiers::NONE, Key::Space)
                    || input.consume_key(Modifiers::ALT, Key::ArrowDown)
            });
        if (expand.clicked() || open_key) && enabled {
            Popup::toggle_id(ui.ctx(), popup_id);
            if !was_open {
                memory.cursor = self.selected.unwrap_or(0).min(count.saturating_sub(1));
                memory.reveal = true;
                memory.focus_grid = true;
            }
        }
        let mut open = Popup::is_id_open(ui.ctx(), popup_id) && enabled && count > 0;
        let expanded = open;
        ui.ctx()
            .accesskit_node_builder(expand.id, |node| node.set_expanded(expanded));

        if open {
            let mut close = false;
            let width = finite(self.popup_width.unwrap_or(rect.width())).max(side + 2.0 * GAP);
            let popup_columns = {
                let inner = width - 2.0 * spacing_vertical::XS;
                ((inner + GAP) / (side + GAP)).floor().max(1.0) as usize
            };

            let lines = popup_lines(count, popup_columns, &self.categories);

            // Keyboard: while the grid has focus, arrows move the cursor and Enter selects.
            // Otherwise focus is on a footer item, which handles its own keys. Escape closes
            // from anywhere in the popup.
            let grid_focused = ui.memory(|memory| memory.has_focus(grid_id));
            let (moved, choose, escape) = ui.input_mut(|input| {
                let mut moved = false;
                let mut choose = false;
                if grid_focused {
                    let last = count - 1;
                    let before = memory.cursor;
                    let mut cursor = before;
                    if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                        cursor = cursor.saturating_sub(1);
                    }
                    if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
                        cursor = (cursor + 1).min(last);
                    }
                    if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        cursor = step_row(&lines, cursor, false);
                    }
                    if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        cursor = step_row(&lines, cursor, true);
                    }
                    if input.consume_key(Modifiers::NONE, Key::Home) {
                        cursor = 0;
                    }
                    if input.consume_key(Modifiers::NONE, Key::End) {
                        cursor = last;
                    }
                    moved = cursor != before;
                    memory.cursor = cursor;
                    choose = input.consume_key(Modifiers::NONE, Key::Enter)
                        || input.consume_key(Modifiers::NONE, Key::Space);
                }
                let escape = input.consume_key(Modifiers::NONE, Key::Escape);
                (moved, choose, escape)
            });
            memory.reveal |= moved;
            if choose {
                if *self.selected != Some(memory.cursor) {
                    *self.selected = Some(memory.cursor);
                    changed = true;
                }
                close = true;
            }
            if escape {
                close = true;
            }

            let line_heights: Vec<f32> = lines
                .iter()
                .map(|line| match line {
                    Line::Header(_) => HEADER_HEIGHT,
                    Line::Row(_) => side + GAP,
                })
                .collect();
            let max_height = POPUP_VISIBLE_ROWS * (side + GAP);
            let reveal_offset = memory
                .reveal
                .then(|| cursor_offset(&lines, &line_heights, memory.cursor))
                .flatten();
            memory.reveal = false;

            let selected_now = *self.selected;
            let mut clicked: Option<usize> = None;
            let mut footer = self.footer.take();
            let mut footer_clicked = false;
            let label_of = |index: usize| self.label_of(index);
            let cursor = memory.cursor;
            let mut focus_grid = memory.focus_grid;
            let id = self.id;
            let categories = &self.categories;
            let grid_label = self.label.clone();
            let _ = Popup::menu(&expand)
                .id(popup_id)
                .open_memory(None)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .frame(menu_frame(ui))
                .width(width)
                .show(|ui| {
                    ui.set_width(width - 2.0 * spacing_vertical::XS);
                    let mut area = ScrollArea::vertical()
                        .id_salt(id.with("scroll"))
                        .max_height(max_height)
                        .auto_shrink([false, true]);
                    if let Some((top, bottom)) = reveal_offset {
                        let current = ui
                            .ctx()
                            .data(|data| data.get_temp::<f32>(id.with("offset")))
                            .unwrap_or(0.0);
                        let offset = if top < current {
                            top
                        } else if bottom > current + max_height {
                            bottom - max_height
                        } else {
                            current
                        };
                        area = area.vertical_scroll_offset(offset.max(0.0));
                    }
                    let total: f32 = line_heights.iter().sum();
                    // Not `show_rows`: it needs every row the same height, and category
                    // header lines are shorter than tile rows. This still lays out only the
                    // lines inside the viewport.
                    let output = area.show_viewport(ui, |ui, viewport| {
                        ui.set_height(total);
                        // The grid as one focus stop; the cursor tile shows where it is.
                        let grid = ui.interact(
                            viewport.translate(ui.min_rect().min.to_vec2()),
                            grid_id,
                            Sense::focusable_noninteractive(),
                        );
                        if focus_grid && ui.is_visible() {
                            grid.request_focus();
                            focus_grid = false;
                        }
                        grid.widget_info(|| WidgetInfo::labeled(WidgetType::Unknown, true, &grid_label));
                        ui.memory_mut(|memory| {
                            memory.set_focus_lock_filter(
                                grid_id,
                                EventFilter {
                                    horizontal_arrows: true,
                                    vertical_arrows: true,
                                    tab: false,
                                    escape: true,
                                },
                            );
                        });
                        let origin = ui.min_rect().min;
                        let mut y = 0.0;
                        for (line, height) in lines.iter().zip(&line_heights) {
                            let top = y;
                            y += height;
                            if top + height < viewport.min.y || top > viewport.max.y {
                                continue;
                            }
                            match line {
                                Line::Header(category) => {
                                    if let Some(category) = categories.get(*category) {
                                        let pos = origin + Vec2::new(GAP, top);
                                        let font = text::font_id(TypeRamp::Caption1);
                                        let galley = category.label.clone().into_galley(
                                            ui,
                                            Some(TextWrapMode::Truncate),
                                            width,
                                            font,
                                        );
                                        let y_pos = pos.y + (HEADER_HEIGHT - galley.size().y) * 0.5;
                                        let text = galley.text().to_owned();
                                        ui.painter().galley(
                                            egui::pos2(pos.x, y_pos),
                                            galley,
                                            c(theme.color.neutral_foreground_3),
                                        );
                                        let header_rect =
                                            Rect::from_min_size(pos, Vec2::new(width, HEADER_HEIGHT));
                                        ui.interact(
                                            header_rect,
                                            id.with(("header", category.first)),
                                            Sense::hover(),
                                        )
                                        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
                                    }
                                }
                                Line::Row(range) => {
                                    for (slot, index) in range.clone().enumerate() {
                                        let min = origin + Vec2::new(slot as f32 * (side + GAP), top);
                                        let tile = Rect::from_min_size(min, Vec2::splat(side));
                                        let tile_response =
                                            // Clickable but not a focus stop: the grid holds focus and the
                                        // keyboard cursor picks the tile, so Tab goes on to the footer.
                                        ui.interact(tile, id.with(("popup", index)), Sense::CLICK);
                                        let state = item_state(
                                            &tile_response,
                                            selected_now == Some(index),
                                            index == cursor && grid_focused,
                                            true,
                                        );
                                        paint_tile(ui, &theme, tile, state);
                                        draw_tile(ui, &theme, state, |ui| draw(ui, index, tile, state));
                                        if state.focused {
                                            paint_focus_ring(ui, tile, radius);
                                        }
                                        let label = label_of(index);
                                        let selected = state.selected;
                                        tile_response.widget_info(|| {
                                            WidgetInfo::selected(WidgetType::Button, true, selected, &label)
                                        });
                                        if tile_response.clicked() {
                                            clicked = Some(index);
                                        }
                                    }
                                }
                            }
                        }
                    });
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(id.with("offset"), output.state.offset.y));
                    if let Some(footer) = footer.take() {
                        divider(ui);
                        let footer_rect = ui.scope(footer).response.rect;
                        let footer_focused = ui
                            .memory(|memory| memory.focused())
                            .is_some_and(|focused| focused != grid_id && focused != expand.id);
                        footer_clicked = ui.input(|input| {
                            let pointer = input.pointer.any_click()
                                && input
                                    .pointer
                                    .interact_pos()
                                    .is_some_and(|pos| footer_rect.contains(pos));
                            let key = footer_focused
                                && (input.key_pressed(Key::Enter) || input.key_pressed(Key::Space));
                            pointer || key
                        });
                    }
                });

            memory.focus_grid = focus_grid;
            if let Some(index) = clicked {
                memory.cursor = index;
                if *self.selected != Some(index) {
                    *self.selected = Some(index);
                    changed = true;
                }
                close = true;
            }
            if close || footer_clicked {
                Popup::close_id(ui.ctx(), popup_id);
                expand.request_focus();
                open = false;
            }
        }

        if changed && let Some(index) = *self.selected {
            memory.row = (index / columns).min(rows - 1);
        }
        let first_visible = memory.row * columns;
        ui.ctx().data_mut(|data| data.insert_temp(memory_id, memory));

        GalleryResponse {
            response: response.union(expand),
            changed,
            first_visible,
            popup_open: open,
        }
    }

    fn label_of(&self, index: usize) -> String {
        match &self.item_label {
            Some(label) => label(index),
            None => format!("{} {}", self.label, index + 1),
        }
    }
}

/// Lines of the popup: headers before the first item of each category, then rows of up to
/// `columns` items that never cross a category boundary.
fn popup_lines(count: usize, columns: usize, categories: &[Category]) -> Vec<Line> {
    let mut starts: Vec<(usize, Option<usize>)> = vec![(0, None)];
    for (n, category) in categories.iter().enumerate() {
        let first = category.first.min(count);
        if starts.last().is_some_and(|(start, _)| *start == first) {
            starts.pop();
        }
        starts.push((first, Some(n)));
    }
    let mut lines = Vec::new();
    for (i, (start, header)) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map_or(count, |(next, _)| *next).max(*start);
        if let Some(header) = header {
            lines.push(Line::Header(*header));
        }
        let mut row = *start;
        while row < end {
            let row_end = (row + columns).min(end);
            lines.push(Line::Row(row..row_end));
            row = row_end;
        }
    }
    lines
}

/// The item one tile row above or below `cursor`, following the popup's real rows, which stop
/// short at each category. Keeps the column where the target row is long enough, otherwise
/// lands on its last item. Stays put at the first or last row.
fn step_row(lines: &[Line], cursor: usize, down: bool) -> usize {
    let rows: Vec<&Range<usize>> = lines
        .iter()
        .filter_map(|line| match line {
            Line::Row(range) => Some(range),
            Line::Header(_) => None,
        })
        .collect();
    let Some(at) = rows.iter().position(|range| range.contains(&cursor)) else {
        return cursor;
    };
    let target = if down {
        rows.get(at + 1)
    } else {
        at.checked_sub(1).and_then(|at| rows.get(at))
    };
    match target {
        Some(range) => (range.start + (cursor - rows[at].start)).min(range.end - 1),
        None => cursor,
    }
}

/// Top and bottom of the row holding `cursor`, in popup content coordinates.
fn cursor_offset(lines: &[Line], heights: &[f32], cursor: usize) -> Option<(f32, f32)> {
    let mut y = 0.0;
    for (line, height) in lines.iter().zip(heights) {
        if let Line::Row(range) = line
            && range.contains(&cursor)
        {
            return Some((y, y + height));
        }
        y += height;
    }
    None
}

fn item_state(response: &Response, selected: bool, focused: bool, enabled: bool) -> GalleryItemState {
    GalleryItemState {
        selected,
        hovered: response.hovered(),
        pressed: response.is_pointer_button_down_on(),
        focused,
        enabled,
    }
}

/// Run the caller's tile drawing with the text colour that matches the tile's fill.
///
/// High contrast fills hovered, pressed and selected tiles with the highlight colour, which
/// needs the matching `neutral_foreground_1_*` ink; the resting foreground is about 1.46:1 on
/// it (WI-1845). `ui.visuals().text_color()` returns it, and labels pick it up.
fn draw_tile(ui: &mut Ui, theme: &Theme, state: GalleryItemState, draw: impl FnOnce(&mut Ui)) {
    let saved = ui.visuals().override_text_color;
    if let Some(ink) = tile_ink(theme, state) {
        ui.visuals_mut().override_text_color = Some(ink);
    }
    draw(ui);
    ui.visuals_mut().override_text_color = saved;
}

/// The foreground for a tile's fill, or `None` at rest.
fn tile_ink(theme: &Theme, state: GalleryItemState) -> Option<egui::Color32> {
    let color = if !state.enabled {
        return None;
    } else if state.pressed {
        theme.color.neutral_foreground_1_pressed
    } else if state.hovered {
        theme.color.neutral_foreground_1_hover
    } else if state.selected {
        theme.color.neutral_foreground_1_selected
    } else {
        return None;
    };
    Some(c(color))
}

/// Tile background: subtle hover and pressed fills, and the selected fill with a brand stroke.
fn paint_tile(ui: &Ui, theme: &Theme, rect: Rect, state: GalleryItemState) {
    let radius = CornerRadius::from(border_radius::MEDIUM);
    let fill = if !state.enabled {
        egui::Color32::TRANSPARENT
    } else if state.pressed {
        c(theme.color.subtle_background_pressed)
    } else if state.hovered {
        c(theme.color.subtle_background_hover)
    } else if state.selected {
        c(theme.color.subtle_background_selected)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, radius, fill);
    if state.selected {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(stroke_width::THIN, c(theme.color.brand_stroke_1)),
            StrokeKind::Inside,
        );
    }
}

/// One of the scroll or expand buttons at the inline row's right edge.
fn edge_button(ui: &mut Ui, theme: &Theme, rect: Rect, id: Id, icon: Icon, label: &str) -> Response {
    let enabled = ui.is_enabled();
    let response = ui.interact(rect, id, Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            egui::Color32::TRANSPARENT
        } else if response.is_pointer_button_down_on() {
            c(theme.color.subtle_background_pressed)
        } else if response.hovered() {
            c(theme.color.subtle_background_hover)
        } else {
            egui::Color32::TRANSPARENT
        };
        let radius = CornerRadius::from(border_radius::SMALL);
        ui.painter().rect_filled(rect, radius, fill);
        let tint = if !enabled {
            c(theme.color.neutral_foreground_disabled)
        } else if response.is_pointer_button_down_on() {
            c(theme.color.neutral_foreground_2_pressed)
        } else if response.hovered() {
            c(theme.color.neutral_foreground_2_hover)
        } else {
            c(theme.color.neutral_foreground_2)
        };
        let side = rect.width().min(rect.height());
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(side));
        paint_icon(ui, icon_rect, icon, IconSize::S16, IconStyle::Regular, Some(tint));
        if focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, radius);
        }
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    response
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(first: usize) -> Category {
        Category {
            label: "c".into(),
            first,
        }
    }

    #[test]
    fn rows_do_not_cross_categories() {
        let lines = popup_lines(7, 3, &[category(0), category(4)]);
        assert_eq!(
            lines,
            vec![
                Line::Header(0),
                Line::Row(0..3),
                Line::Row(3..4),
                Line::Header(1),
                Line::Row(4..7),
            ]
        );
    }

    #[test]
    fn up_and_down_follow_rows_that_stop_at_categories() {
        // 4 columns, categories at 0 and 6: rows 0..4, 4..6, 6..10, 10..12.
        let lines = popup_lines(12, 4, &[category(0), category(6)]);
        assert_eq!(step_row(&lines, 4, true), 6);
        assert_eq!(step_row(&lines, 1, true), 5);
        assert_eq!(step_row(&lines, 3, true), 5);
        assert_eq!(step_row(&lines, 9, false), 5);
        assert_eq!(step_row(&lines, 6, false), 4);
        assert_eq!(step_row(&lines, 5, false), 1);
        assert_eq!(step_row(&lines, 2, false), 2);
        assert_eq!(step_row(&lines, 11, true), 11);
    }

    #[test]
    fn cursor_offset_finds_the_row() {
        let lines = popup_lines(7, 3, &[category(4)]);
        let heights = vec![10.0; lines.len()];
        assert_eq!(cursor_offset(&lines, &heights, 5), Some((30.0, 40.0)));
    }
}
