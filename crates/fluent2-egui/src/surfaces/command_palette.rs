// Adapted from thedancingdeveloper/fluent-gpui crates/fluent-layout/src/command_palette.rs @ f601e54.
// Copyright (c) 2026 sprooty. Licensed Apache-2.0.

//! Fluent command palette: a centred surface with a search box and a ranked list of commands.
//!
//! The app supplies the commands and keeps the recents. The palette ranks the commands against
//! the query ([`rank`], a pure function), highlights the matched characters of each label, and
//! returns the id of the command the user chose. It does no dispatch (ARCHITECTURE rule 5).
//!
//! The score tiers (exact, prefix, substring, then an in-order fuzzy match on the label, with
//! lower tiers for the subtitle, group and keywords) come from fluent-gpui's palette, which
//! rdpapp ships. This port adds the matched positions for highlighting and the recents order.
//!
//! Keyboard: the search box keeps focus. Up and Down move the selection over enabled rows,
//! Enter chooses the selected row, Escape closes the palette. A click on a row chooses it and a
//! click on the backdrop closes the palette.

use egui::epaint::RectShape;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, Area, Context, CornerRadius, Frame, Id, Key, Modal, Order, Rect, Sense, Stroke, TextWrapMode, Ui,
    UiKind, Vec2, WidgetInfo, WidgetType,
};

use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::controls::SearchBox;
use crate::foundation::icon::paint_icon;
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, ThemeKind, c};

const EXACT_MATCH_SCORE: i32 = 100;
const PREFIX_MATCH_SCORE: i32 = 80;
const SUBSTRING_MATCH_SCORE: i32 = 60;
const SUBTITLE_MATCH_SCORE: i32 = 50;
const KEYWORD_MATCH_SCORE: i32 = 40;
const GROUP_MATCH_SCORE: i32 = 35;
const FUZZY_MATCH_SCORE: i32 = 20;

/// Default surface width, in points.
const DEFAULT_WIDTH: f32 = 560.0;
/// Distance from the top of the screen to the surface, as in VS Code and fluent-gpui.
const TOP_OFFSET: f32 = 80.0;
/// Maximum height of the result list before it scrolls.
const LIST_MAX_HEIGHT: f32 = 360.0;
/// Row height with a subtitle, and without.
const ROW_HEIGHT_TWO_LINES: f32 = 44.0;
const ROW_HEIGHT: f32 = 32.0;
/// Width reserved for a leading icon.
const ICON_SLOT: f32 = 20.0;

/// One command offered by the palette.
///
/// ```
/// # use fluent2_egui::surfaces::PaletteCommand;
/// let open = PaletteCommand::new("file.open", "Open file")
///     .group("File")
///     .shortcut("Ctrl+O")
///     .keyword("load");
/// assert_eq!(open.id, "file.open");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteCommand {
    /// Returned by [`CommandPalette::show`] when the command is chosen. Also the row's egui id
    /// salt, so it must be unique within the list.
    pub id: String,
    /// Display text, matched and highlighted.
    pub label: String,
    /// Second line, matched at a lower score.
    pub subtitle: Option<String>,
    /// Section shown above consecutive rows with the same group, matched at a lower score.
    pub group: Option<String>,
    /// Trailing shortcut caption. Not matched.
    pub shortcut: Option<String>,
    /// Leading icon.
    pub icon: Option<Icon>,
    /// Extra words matched but not shown.
    pub keywords: Vec<String>,
    /// Disabled commands are shown but cannot be selected or chosen.
    pub disabled: bool,
}

impl PaletteCommand {
    /// A command with an id and a label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            ..Self::default()
        }
    }

    /// Second line.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Section name.
    #[must_use]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Trailing shortcut caption, for example `"Ctrl+O"`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Leading icon.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Add a matched, hidden keyword.
    #[must_use]
    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Show the command but refuse to choose it.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// One ranked row: which command, its score, and which label characters matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteMatch {
    /// Index into the command slice.
    pub index: usize,
    /// Higher is better. 0 for every row when the query is empty.
    pub score: i32,
    /// Whether the command is in the app's recents.
    pub recent: bool,
    /// Character (not byte) indices into the label that matched the query, ascending. Empty when
    /// the match came from the subtitle, group or a keyword, or the query is empty.
    pub label_matches: Vec<usize>,
}

/// Rank `commands` against `query`.
///
/// Matching ignores case and collapses runs of whitespace in the query. A command scores by its
/// best field: label exact 100, prefix 80, substring 60, in-order fuzzy 20; subtitle 70/60/50;
/// keyword 60/50/40; group 55/45/35 (no fuzzy outside the label). Commands that do not match are
/// left out. Ties go to recents (in `recents` order), then to the slice order.
///
/// An empty query keeps every command: the recents first, in `recents` order, then the rest in
/// slice order. `limit` caps the result; `None` keeps all of it.
///
/// ```
/// # use fluent2_egui::surfaces::{PaletteCommand, rank};
/// let commands = [PaletteCommand::new("a", "Open file"), PaletteCommand::new("b", "Save")];
/// let ranked = rank(&commands, "of", &[] as &[&str], None);
/// assert_eq!(ranked[0].index, 0);
/// assert_eq!(ranked[0].label_matches, vec![0, 5]);
/// ```
#[must_use]
pub fn rank<S: AsRef<str>>(
    commands: &[PaletteCommand],
    query: &str,
    recents: &[S],
    limit: Option<usize>,
) -> Vec<PaletteMatch> {
    let query = normalize(query);
    let recent_rank = |id: &str| recents.iter().position(|recent| recent.as_ref() == id);
    let mut ranked: Vec<(Option<usize>, PaletteMatch)> = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let recent = recent_rank(&command.id);
            let (score, label_matches) = if query.is_empty() {
                (0, Vec::new())
            } else {
                score_command(command, &query)?
            };
            Some((
                recent,
                PaletteMatch {
                    index,
                    score,
                    recent: recent.is_some(),
                    label_matches,
                },
            ))
        })
        .collect();
    ranked.sort_by(|(left_recent, left), (right_recent, right)| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| match (left_recent, right_recent) {
                (Some(l), Some(r)) => l.cmp(r),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left.index.cmp(&right.index))
    });
    ranked
        .into_iter()
        .map(|(_, found)| found)
        .take(limit.unwrap_or(usize::MAX))
        .collect()
}

/// Lowercase per character, so character indices stay aligned with the original text.
fn fold(text: &str) -> Vec<char> {
    text.chars()
        .map(|ch| ch.to_lowercase().next().unwrap_or(ch))
        .collect()
}

fn normalize(query: &str) -> Vec<char> {
    fold(&query.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn score_command(command: &PaletteCommand, query: &[char]) -> Option<(i32, Vec<usize>)> {
    let label = fold(&command.label);
    let mut best = tier_score(
        &label,
        query,
        [
            EXACT_MATCH_SCORE,
            PREFIX_MATCH_SCORE,
            SUBSTRING_MATCH_SCORE,
            FUZZY_MATCH_SCORE,
        ],
    );
    let others = command
        .subtitle
        .iter()
        .map(|text| (text, SUBTITLE_MATCH_SCORE))
        .chain(command.group.iter().map(|text| (text, GROUP_MATCH_SCORE)))
        .chain(command.keywords.iter().map(|text| (text, KEYWORD_MATCH_SCORE)));
    for (text, base) in others {
        if let Some((score, _)) = tier_score(&fold(text), query, [base + 20, base + 10, base, 0])
            && best.as_ref().is_none_or(|(current, _)| score > *current)
        {
            best = Some((score, Vec::new()));
        }
    }
    best
}

/// Score `text` against `query` with `[exact, prefix, substring, fuzzy]`; a 0 tier is skipped.
/// Returns the score and the matched character indices.
fn tier_score(text: &[char], query: &[char], tiers: [i32; 4]) -> Option<(i32, Vec<usize>)> {
    let [exact, prefix, substring, fuzzy] = tiers;
    if query.is_empty() {
        return None;
    }
    let span = |start: usize| (start..start + query.len()).collect::<Vec<_>>();
    if text == query {
        return Some((exact, span(0)));
    }
    if text.starts_with(query) {
        return Some((prefix, span(0)));
    }
    if let Some(start) = text.windows(query.len()).position(|window| window == query) {
        return Some((substring, span(start)));
    }
    if fuzzy > 0 {
        return fuzzy_positions(text, query).map(|positions| (fuzzy, positions));
    }
    None
}

/// Greedy in-order match of the query's non-space characters.
fn fuzzy_positions(text: &[char], query: &[char]) -> Option<Vec<usize>> {
    let mut positions = Vec::new();
    let mut from = 0;
    for &wanted in query.iter().filter(|ch| !ch.is_whitespace()) {
        let offset = text.get(from..)?.iter().position(|&ch| ch == wanted)?;
        positions.push(from + offset);
        from += offset + 1;
    }
    Some(positions)
}

/// Row index of the first enabled row at or after `from` going in `step` direction.
fn next_enabled(
    rows: &[PaletteMatch],
    commands: &[PaletteCommand],
    from: Option<usize>,
    step: isize,
) -> Option<usize> {
    let enabled = |row: usize| {
        rows.get(row)
            .and_then(|found| commands.get(found.index))
            .is_some_and(|command| !command.disabled)
    };
    let mut row = match from {
        Some(row) => row as isize + step,
        None => 0,
    };
    while row >= 0 && (row as usize) < rows.len() {
        if enabled(row as usize) {
            return Some(row as usize);
        }
        row += if step == 0 { 1 } else { step };
    }
    None
}

/// The section header drawn above each row, if any: the recents label above the first recent
/// (empty query only), then the group name wherever the group changes.
fn section_headers<'c>(
    rows: &[PaletteMatch],
    commands: &'c [PaletteCommand],
    query_empty: bool,
    recent_label: &'c str,
) -> Vec<Option<&'c str>> {
    let mut previous_group: Option<&str> = None;
    let mut in_recents = false;
    rows.iter()
        .map(|found| {
            let command = commands.get(found.index)?;
            let recent = query_empty && found.recent;
            let group = command.group.as_deref();
            let header = if recent {
                (!in_recents).then_some(recent_label)
            } else if group.is_some() && (group != previous_group || in_recents) {
                group
            } else {
                None
            };
            in_recents = recent;
            previous_group = if recent { None } else { group };
            header
        })
        .collect()
}

/// Per-palette state kept in egui memory between frames.
#[derive(Clone, Debug, Default)]
struct PaletteState {
    query: String,
    /// Id of the selected command, so the selection survives re-ranking.
    selected: Option<String>,
    seen: bool,
    opener: Option<Id>,
}

/// A command palette.
///
/// ```
/// # use fluent2_egui::surfaces::{CommandPalette, PaletteCommand};
/// # fn demo(ctx: &egui::Context, open: &mut bool, recents: &mut Vec<String>) {
/// let commands = vec![
///     PaletteCommand::new("file.open", "Open file").shortcut("Ctrl+O"),
///     PaletteCommand::new("file.save", "Save").shortcut("Ctrl+S"),
/// ];
/// if let Some(id) = CommandPalette::new("palette", &commands)
///     .recents(recents)
///     .show(ctx, open)
/// {
///     recents.retain(|recent| recent != &id);
///     recents.insert(0, id.clone());
///     // the app runs the command named `id`
/// }
/// # }
/// ```
pub struct CommandPalette<'a> {
    id: Id,
    commands: &'a [PaletteCommand],
    recents: &'a [String],
    label: String,
    placeholder: String,
    recent_label: String,
    empty_text: String,
    width: f32,
    max_results: Option<usize>,
}

impl<'a> CommandPalette<'a> {
    /// A palette over `commands`, with an id from `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, commands: &'a [PaletteCommand]) -> Self {
        Self {
            id: Id::new(id_salt),
            commands,
            recents: &[],
            label: "Command palette".to_owned(),
            placeholder: "Type a command".to_owned(),
            recent_label: "Recent".to_owned(),
            empty_text: "No results".to_owned(),
            width: DEFAULT_WIDTH,
            max_results: None,
        }
    }

    /// Ids of recently chosen commands, most recent first. The app keeps this list.
    #[must_use]
    pub fn recents(mut self, recents: &'a [String]) -> Self {
        self.recents = recents;
        self
    }

    /// Accessible name of the palette and its search box. Defaults to "Command palette".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Search box hint. Defaults to "Type a command".
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Header above the recents when the query is empty. Defaults to "Recent".
    #[must_use]
    pub fn recent_label(mut self, label: impl Into<String>) -> Self {
        self.recent_label = label.into();
        self
    }

    /// Text shown when nothing matches. Defaults to "No results".
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// Maximum surface width, in points. Defaults to 560. Non-finite values use the default.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Cap the number of rows. Defaults to no cap.
    #[must_use]
    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max.max(1));
        self
    }

    /// The egui id of a command's row, for example to drive it in a test.
    #[must_use]
    pub fn row_id(&self, command_id: &str) -> Id {
        self.id.with("row").with(command_id)
    }

    /// Show the palette while `*open` is `true`.
    ///
    /// Returns the id of the command chosen on this frame and clears `*open`. Escape and a
    /// backdrop click clear `*open` and return `None`. Each opening starts with an empty query.
    pub fn show(self, ctx: &Context, open: &mut bool) -> Option<String> {
        let state_id = self.id.with("state");
        let mut state: PaletteState = ctx.data(|data| data.get_temp(state_id)).unwrap_or_default();

        if !*open {
            if state.seen {
                if let Some(opener) = state.opener {
                    ctx.memory_mut(|memory| memory.request_focus(opener));
                }
                ctx.data_mut(|data| data.remove_temp::<PaletteState>(state_id));
            }
            return None;
        }

        let opening = !state.seen;
        if opening {
            state = PaletteState {
                seen: true,
                opener: ctx.memory(|memory| memory.focused()),
                ..PaletteState::default()
            };
        }

        // Keys are taken before the search box runs, so the text edit never sees them.
        let (up, down, enter, escape) = ctx.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                input.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                input.consume_key(egui::Modifiers::NONE, Key::Enter),
                input.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });

        let theme = Theme::get(ctx);
        let width = if self.width.is_finite() && self.width > 0.0 {
            self.width.min(10_000.0)
        } else {
            DEFAULT_WIDTH
        };
        let stroke_color = if theme.kind == ThemeKind::HighContrast {
            c(theme.color.neutral_stroke_2)
        } else {
            c(theme.color.transparent_stroke)
        };
        let frame = Frame::new()
            .fill(c(theme.color.neutral_background_1))
            .stroke(Stroke::new(stroke_width::THIN, stroke_color))
            .corner_radius(CornerRadius::same(radius_u8(border_radius::X_LARGE)))
            .inner_margin(Vec2::new(spacing_horizontal::S, spacing_vertical::S))
            .shadow(super::dialog::shadow64(&theme));
        let area = Area::new(self.id)
            .kind(UiKind::Modal)
            .sense(Sense::hover())
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, TOP_OFFSET))
            .order(Order::Foreground)
            .interactable(true);

        let surface_id = self.id.with("surface");
        let list_id = self.id.with("list");
        let search_salt = self.id.with("search");
        let shown = Modal::new(self.id)
            .backdrop_color(c(theme.color.background_overlay))
            .frame(frame)
            .area(area)
            .show(ctx, |ui| {
                ui.set_width(
                    width
                        .min(ui.ctx().content_rect().width() - 2.0 * spacing_horizontal::L)
                        .max(1.0),
                );
                crate::controls::ensure_type_ramp(ui);
                let before = state.query.clone();
                let search = ui.add(
                    SearchBox::new(&mut state.query)
                        .placeholder(self.placeholder.clone())
                        .label(self.label.clone())
                        .id_salt(search_salt),
                );
                if opening || !search.has_focus() {
                    search.request_focus();
                }

                let rows = rank(self.commands, &state.query, self.recents, self.max_results);
                let row_of = |id: &Option<String>| {
                    id.as_ref().and_then(|id| {
                        rows.iter().position(|found| {
                            self.commands
                                .get(found.index)
                                .is_some_and(|command| &command.id == id)
                        })
                    })
                };
                let mut selected = row_of(&state.selected);
                if before != state.query
                    || selected
                        .is_none_or(|row| next_enabled(&rows, self.commands, Some(row), 0) != Some(row))
                {
                    selected = next_enabled(&rows, self.commands, None, 1);
                }
                let mut moved = false;
                if down && let Some(row) = next_enabled(&rows, self.commands, selected, 1) {
                    selected = Some(row);
                    moved = true;
                }
                if up
                    && let Some(row) =
                        selected.and_then(|row| next_enabled(&rows, self.commands, Some(row), -1))
                {
                    selected = Some(row);
                    moved = true;
                }

                ui.add_space(spacing_vertical::S);
                let headers =
                    section_headers(&rows, self.commands, state.query.is_empty(), &self.recent_label);
                // The area keeps last frame's size as its limit, so the list asks for its own
                // height; otherwise it could never grow past the first frame's content.
                let header_height = TypeRamp::Caption1.line_height() + spacing_vertical::XS;
                let content_height: f32 = rows
                    .iter()
                    .filter_map(|found| self.commands.get(found.index))
                    .map(|command| {
                        if command.subtitle.is_some() {
                            ROW_HEIGHT_TWO_LINES
                        } else {
                            ROW_HEIGHT
                        }
                    })
                    .sum::<f32>()
                    + headers.iter().flatten().count() as f32 * header_height;
                let list_height = content_height.clamp(ROW_HEIGHT, LIST_MAX_HEIGHT);
                let mut clicked = None;
                let list = egui::ScrollArea::vertical()
                    .max_height(list_height)
                    .min_scrolled_height(list_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if rows.is_empty() {
                            ui.label(
                                text::rich(self.empty_text.clone(), TypeRamp::Body1)
                                    .color(c(theme.color.neutral_foreground_3)),
                            );
                        }
                        for ((row, found), header) in rows.iter().enumerate().zip(&headers) {
                            let Some(command) = self.commands.get(found.index) else {
                                continue;
                            };
                            if let Some(header) = header {
                                ui.add_space(spacing_vertical::XS);
                                ui.label(
                                    text::rich(header.to_owned(), TypeRamp::Caption1)
                                        .color(c(theme.color.neutral_foreground_3)),
                                );
                            }
                            let is_selected = selected == Some(row);
                            let response =
                                paint_row(ui, &theme, self.row_id(&command.id), command, found, is_selected);
                            if is_selected && moved {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() && !command.disabled {
                                clicked = Some(command.id.clone());
                            }
                        }
                    });
                let list_rect = list.inner_rect;
                let _ = ui.interact(list_rect, list_id, Sense::hover());
                let label = self.label.clone();
                let active = selected
                    .and_then(|row| rows.get(row))
                    .and_then(|found| self.commands.get(found.index))
                    .map(|command| self.row_id(&command.id));
                ui.ctx().accesskit_node_builder(list_id, |node| {
                    node.set_role(egui::accesskit::Role::ListBox);
                    node.set_label(label.clone());
                });
                if let Some(active) = active {
                    ui.ctx().accesskit_node_builder(search.id, |node| {
                        node.set_active_descendant(active.accesskit_id());
                        node.push_controlled(list_id.accesskit_id());
                    });
                }
                let _ = ui.interact(ui.min_rect(), surface_id, Sense::hover());
                ui.ctx().accesskit_node_builder(surface_id, |node| {
                    node.set_role(egui::accesskit::Role::Dialog);
                    node.set_label(label);
                });

                let chosen_by_key = if enter {
                    selected
                        .and_then(|row| rows.get(row))
                        .and_then(|found| self.commands.get(found.index))
                        .filter(|command| !command.disabled)
                        .map(|command| command.id.clone())
                } else {
                    None
                };
                state.selected = selected
                    .and_then(|row| rows.get(row))
                    .and_then(|found| self.commands.get(found.index))
                    .map(|command| command.id.clone());
                clicked.or(chosen_by_key)
            });

        let chosen = shown.inner;
        let close = chosen.is_some() || escape || shown.backdrop_response.clicked();
        if close {
            *open = false;
            if let Some(opener) = state.opener {
                ctx.memory_mut(|memory| memory.request_focus(opener));
            }
            ctx.data_mut(|data| data.remove_temp::<PaletteState>(state_id));
        } else {
            ctx.data_mut(|data| data.insert_temp(state_id, state));
        }
        chosen
    }
}

/// Paint one row and report it to AccessKit as a list box option.
fn paint_row(
    ui: &mut Ui,
    theme: &Theme,
    id: Id,
    command: &PaletteCommand,
    found: &PaletteMatch,
    selected: bool,
) -> egui::Response {
    let height = if command.subtitle.is_some() {
        ROW_HEIGHT_TWO_LINES
    } else {
        ROW_HEIGHT
    };
    let width = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let sense = if command.disabled {
        Sense::hover()
    } else {
        Sense::click()
    };
    let response = ui.interact(rect, id, sense);
    let radius = CornerRadius::same(radius_u8(border_radius::MEDIUM));

    let fill = if command.disabled {
        None
    } else if selected {
        Some(c(theme.color.neutral_background_1_selected))
    } else if response.hovered() {
        Some(c(theme.color.neutral_background_1_hover))
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().add(RectShape::filled(rect, radius, fill));
    }
    if selected && theme.kind == ThemeKind::HighContrast {
        // The selected fill alone is not visible in high contrast; outline the row too.
        ui.painter().rect_stroke(
            rect.shrink(stroke_width::THIN),
            radius,
            Stroke::new(
                stroke_width::THICK,
                c(theme.color.neutral_stroke_accessible_selected),
            ),
            egui::StrokeKind::Inside,
        );
    }

    let fg = if command.disabled {
        c(theme.color.neutral_foreground_disabled)
    } else {
        c(theme.color.neutral_foreground_1)
    };
    let secondary = if command.disabled {
        fg
    } else {
        c(theme.color.neutral_foreground_3)
    };
    let mut x = rect.left() + spacing_horizontal::S;
    if let Some(icon) = command.icon {
        let slot = Rect::from_min_size(
            egui::pos2(x, rect.center().y - ICON_SLOT * 0.5),
            Vec2::splat(ICON_SLOT),
        );
        paint_icon(ui, slot, icon, IconSize::S20, IconStyle::Regular, Some(fg));
    }
    x += ICON_SLOT + spacing_horizontal::S;

    let shortcut = command.shortcut.as_ref().map(|shortcut| {
        ui.fonts_mut(|fonts| {
            fonts.layout_no_wrap(shortcut.clone(), text::font_id(TypeRamp::Caption1), secondary)
        })
    });
    let shortcut_w = shortcut
        .as_ref()
        .map_or(0.0, |galley| galley.size().x + spacing_horizontal::S);
    let text_w = (rect.right() - spacing_horizontal::S - shortcut_w - x).max(1.0);

    let mut job = highlighted_job(
        &command.label,
        &found.label_matches,
        text::font_id(TypeRamp::Body1),
        fg,
        if command.disabled {
            fg
        } else {
            c(theme.color.brand_foreground_1)
        },
    );
    job.wrap.max_width = text_w;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let label_galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let sub_galley = command.subtitle.as_ref().map(|subtitle| {
        egui::WidgetText::from(text::rich(subtitle.clone(), TypeRamp::Caption1).color(secondary)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            text_w,
            text::font_id(TypeRamp::Caption1),
        )
    });
    let total = label_galley.size().y + sub_galley.as_ref().map_or(0.0, |galley| galley.size().y);
    let mut y = rect.center().y - total * 0.5;
    let label_h = label_galley.size().y;
    ui.painter().galley(egui::pos2(x, y), label_galley, fg);
    y += label_h;
    if let Some(galley) = sub_galley {
        ui.painter().galley(egui::pos2(x, y), galley, secondary);
    }
    if let Some(galley) = shortcut {
        let pos = egui::pos2(
            rect.right() - spacing_horizontal::S - galley.size().x,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(pos, galley, secondary);
    }

    let enabled = !command.disabled;
    let label = command.label.clone();
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::SelectableLabel, enabled, &label);
        info.selected = Some(selected);
        info
    });
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(egui::accesskit::Role::ListBoxOption);
        node.set_selected(selected);
        if let Some(subtitle) = &command.subtitle {
            node.set_description(subtitle.clone());
        }
        if !enabled {
            node.set_disabled();
        }
    });
    response
}

/// A one-line job with the characters at `matches` drawn in `accent` and underlined, so the
/// highlight does not rely on colour alone.
fn highlighted_job(
    label: &str,
    matches: &[usize],
    font: egui::FontId,
    color: egui::Color32,
    accent: egui::Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(font.clone(), color);
    let marked = TextFormat {
        color: accent,
        underline: Stroke::new(stroke_width::THIN, accent),
        ..TextFormat::simple(font, accent)
    };
    let mut run = String::new();
    let mut run_marked = false;
    for (index, ch) in label.chars().enumerate() {
        let is_marked = matches.binary_search(&index).is_ok();
        if is_marked != run_marked && !run.is_empty() {
            job.append(&run, 0.0, if run_marked { marked.clone() } else { plain.clone() });
            run.clear();
        }
        run_marked = is_marked;
        run.push(ch);
    }
    if !run.is_empty() {
        job.append(&run, 0.0, if run_marked { marked } else { plain });
    }
    job
}

fn radius_u8(value: f32) -> u8 {
    if value.is_finite() {
        value.round().clamp(0.0, f32::from(u8::MAX)) as u8
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{PaletteCommand, next_enabled, rank, section_headers};

    const NO_RECENTS: &[&str] = &[];

    fn order(commands: &[PaletteCommand], query: &str, recents: &[&str]) -> Vec<String> {
        rank(commands, query, recents, None)
            .into_iter()
            .map(|found| commands[found.index].id.clone())
            .collect()
    }

    #[test]
    fn empty_query_keeps_order_and_limit() {
        let commands = [
            PaletteCommand::new("a", "Archive"),
            PaletteCommand::new("b", "Delete"),
            PaletteCommand::new("c", "Reply"),
        ];
        assert_eq!(order(&commands, "", NO_RECENTS), ["a", "b", "c"]);
        assert_eq!(rank(&commands, "  ", NO_RECENTS, Some(2)).len(), 2);
    }

    #[test]
    fn empty_query_puts_recents_first_in_recents_order() {
        let commands = [
            PaletteCommand::new("a", "Archive"),
            PaletteCommand::new("b", "Delete"),
            PaletteCommand::new("c", "Reply"),
        ];
        assert_eq!(order(&commands, "", &["c", "missing", "b"]), ["c", "b", "a"]);
    }

    #[test]
    fn tiers_order_exact_prefix_substring_fuzzy() {
        let commands = [
            PaletteCommand::new("fuzzy", "Message Archive List"),
            PaletteCommand::new("substring", "Monthly Mail"),
            PaletteCommand::new("prefix", "Mail Settings"),
            PaletteCommand::new("exact", "Mail"),
        ];
        assert_eq!(
            order(&commands, "mail", NO_RECENTS),
            ["exact", "prefix", "substring", "fuzzy"]
        );
    }

    #[test]
    fn subtitle_group_and_keywords_match_without_label_highlight() {
        let commands = [
            PaletteCommand::new("calendar", "Calendar").group("Navigate"),
            PaletteCommand::new("invoice", "Invoice").subtitle("Finance folder"),
            PaletteCommand::new("compose", "New Message").keyword("write"),
        ];
        assert_eq!(order(&commands, "navigate", NO_RECENTS), ["calendar"]);
        assert_eq!(order(&commands, "finance", NO_RECENTS), ["invoice"]);
        let write = rank(&commands, "write", NO_RECENTS, None);
        assert_eq!(write.len(), 1);
        assert_eq!(write[0].score, 60);
        assert!(write[0].label_matches.is_empty());
    }

    #[test]
    fn only_the_label_matches_fuzzily() {
        let commands = [PaletteCommand::new("a", "Open").subtitle("xyz project")];
        assert!(rank(&commands, "xp", NO_RECENTS, None).is_empty());
        assert!(rank(&commands, "on", NO_RECENTS, None).len() == 1);
    }

    #[test]
    fn label_matches_are_char_indices() {
        let commands = [PaletteCommand::new("a", "Ünïcode Search")];
        let found = rank(&commands, "ÜS", NO_RECENTS, None);
        assert_eq!(found[0].label_matches, vec![0, 8]);
        let found = rank(&commands, "code", NO_RECENTS, None);
        assert_eq!(found[0].label_matches, vec![3, 4, 5, 6]);
        assert_eq!(found[0].score, 60);
    }

    #[test]
    fn query_case_and_whitespace_are_normalised() {
        let commands = [PaletteCommand::new("a", "Advanced Search")];
        let found = rank(&commands, "  ADVANCED   SEARCH ", NO_RECENTS, None);
        assert_eq!(found[0].score, 100);
        assert_eq!(found[0].label_matches.len(), "advanced search".len());
    }

    #[test]
    fn recents_break_ties_only() {
        let commands = [
            PaletteCommand::new("a", "Open file"),
            PaletteCommand::new("b", "Open folder"),
            PaletteCommand::new("c", "Open"),
        ];
        assert_eq!(order(&commands, "open", &["b"]), ["c", "b", "a"]);
        assert!(rank(&commands, "open", &["b"], None)[1].recent);
    }

    #[test]
    fn non_matches_are_dropped() {
        let commands = [PaletteCommand::new("a", "Open")];
        assert!(rank(&commands, "zz", NO_RECENTS, None).is_empty());
    }

    #[test]
    fn selection_steps_skip_disabled_rows() {
        let commands = [
            PaletteCommand::new("a", "A").disabled(true),
            PaletteCommand::new("b", "B"),
            PaletteCommand::new("c", "C").disabled(true),
            PaletteCommand::new("d", "D"),
        ];
        let rows = rank(&commands, "", NO_RECENTS, None);
        assert_eq!(next_enabled(&rows, &commands, None, 1), Some(1));
        assert_eq!(next_enabled(&rows, &commands, Some(1), 1), Some(3));
        assert_eq!(next_enabled(&rows, &commands, Some(3), 1), None);
        assert_eq!(next_enabled(&rows, &commands, Some(3), -1), Some(1));
        assert_eq!(next_enabled(&rows, &commands, Some(1), -1), None);
        assert_eq!(next_enabled(&rows, &commands, Some(1), 0), Some(1));
        assert_eq!(next_enabled(&rows, &commands, Some(0), 0), Some(1));
    }

    #[test]
    fn headers_mark_recents_then_group_changes() {
        let commands = [
            PaletteCommand::new("a", "A").group("File"),
            PaletteCommand::new("b", "B").group("File"),
            PaletteCommand::new("c", "C").group("View"),
        ];
        let rows = rank(&commands, "", &["c"], None);
        assert_eq!(
            section_headers(&rows, &commands, true, "Recent"),
            [Some("Recent"), Some("File"), None]
        );
        let rows = rank(&commands, "", NO_RECENTS, None);
        assert_eq!(
            section_headers(&rows, &commands, true, "Recent"),
            [Some("File"), None, Some("View")]
        );
    }
}
