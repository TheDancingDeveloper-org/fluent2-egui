//! MenuBar: the classic File / Edit / View / Tools / Help bar (issue #115).
//!
//! A row of top-level menus whose popups hold Fluent [`MenuItem`](crate::surfaces::MenuItem)s,
//! [`submenu`](crate::surfaces::submenu)s and [`divider`](crate::surfaces::divider)s.
//! Keyboard handling follows Windows conventions:
//!
//! - **Alt** pressed and released on its own, or **F10**, moves focus to the first menu and
//!   underlines the mnemonics. Pressing it again leaves the bar.
//! - **Alt+letter** opens the menu whose mnemonic is that letter, from anywhere in the window.
//!   While the bar has focus the bare letter does the same.
//! - **Left / Right** move between menus, wrapping, and switch the open menu while one is open.
//! - **Down**, **Enter** or **Space** open the focused menu; Up and Down move through its items.
//! - **Esc** closes the open menu and leaves focus on its title; a second Esc leaves the bar.
//!
//! With a menu open, hovering another title switches to it. The bar is an ordinary widget row,
//! so it can sit in a [`TitleBar`](crate::office::TitleBar) slot on an undecorated window.
//!
//! Item shortcut captions come from [`MenuItem::shortcut`](crate::surfaces::MenuItem::shortcut);
//! once the shortcut registry lands, a registry-bound item caption is the hook for that.
//! Forwarding the menus to the macOS application menu is not done here: egui has no native
//! menu API, so an app that wants it builds the OS menu from the same model itself.
//!
//! Colours, radii and spacing come from the installed [`crate::Theme`].

use egui::epaint::RectShape;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    CornerRadius, EventFilter, FocusDirection, Id, InnerResponse, Key, Modifiers, Popup, Role, Sense, Stroke,
    Ui, UiKind, UiStackInfo, Vec2, WidgetInfo, accesskit,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::foundation::focus;
use crate::foundation::text::{self, TypeRamp};
use crate::surfaces::{cancel_focus_request, focus_first_item, menu_contents_style, menu_frame};
use crate::theme::{Theme, c};

/// Height of a menu title, in points, before density.
const TITLE_HEIGHT: f32 = 28.0;
/// Horizontal padding inside a menu title.
const H_PADDING: f32 = spacing_horizontal::S;
/// Minimum width of an open menu popup. Rows fill the popup, so this is also its width
/// unless a submenu or a wide row grows it.
const POPUP_MIN_WIDTH: f32 = 200.0;

/// A menu bar of top-level menus.
///
/// Titles mark their mnemonic with `&`, as Windows resource files do: `"&File"` shows "File"
/// and opens on Alt+F. `"&&"` is a literal ampersand. Without a marker the first letter is the
/// mnemonic.
///
/// ```
/// use fluent2_egui::office::menu_bar::MenuBar;
/// use fluent2_egui::surfaces::{MenuItem, divider};
///
/// # fn demo(ui: &mut egui::Ui) {
/// MenuBar::new("main_menu").show(ui, |bar| {
///     bar.menu("&File", |ui| {
///         if ui.add(MenuItem::new("Open…").shortcut("Ctrl+O")).clicked() {
///             // the app opens a file
///         }
///         divider(ui);
///         let _ = ui.add(MenuItem::new("Exit"));
///     });
///     bar.menu("&Help", |ui| {
///         let _ = ui.add(MenuItem::new("About"));
///     });
/// });
/// # }
/// ```
pub struct MenuBar {
    id: Id,
}

impl MenuBar {
    /// A menu bar whose state is stored under `id`.
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self { id: Id::unique(id) }
    }

    /// Lay the bar out in `ui` and add its menus with `add`. Returns what `add` returns; the
    /// response covers the whole bar.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut MenuBarUi<'_>) -> R) -> InnerResponse<R> {
        let bar_id = ui.make_persistent_id(self.id);
        let mut state = State::load(ui.ctx(), bar_id);
        handle_keys(ui, bar_id, &mut state);

        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mut bar = MenuBarUi {
                ui,
                bar_id,
                state,
                index: 0,
                mnemonics: Vec::new(),
            };
            let inner = add(&mut bar);
            let MenuBarUi { state, mnemonics, .. } = bar;
            (inner, state, mnemonics)
        });
        let (inner, mut state, mnemonics) = row.inner;
        if state.open.is_some_and(|open| open >= mnemonics.len()) {
            state.open = None;
        }
        state.mnemonics = mnemonics;
        state.store(ui.ctx(), bar_id);

        ui.ctx().accesskit_node_builder(row.response.id, |node| {
            node.set_role(accesskit::Role::MenuBar);
            node.set_label("Menu bar");
        });
        InnerResponse::new(inner, row.response)
    }
}

/// Handle passed to the [`MenuBar::show`] closure; add menus with [`MenuBarUi::menu`].
pub struct MenuBarUi<'u> {
    ui: &'u mut Ui,
    bar_id: Id,
    state: State,
    index: usize,
    mnemonics: Vec<Option<char>>,
}

impl MenuBarUi<'_> {
    /// Add a top-level menu titled `title` (see [`MenuBar`] for the `&` mnemonic marker).
    /// `add` fills the popup while it is open. The response is the title's.
    pub fn menu<R>(&mut self, title: &str, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<R>> {
        let index = self.index;
        self.index += 1;
        let (label, mnemonic_at) = parse_mnemonic(title);
        let mnemonic = mnemonic_at
            .and_then(|at| label.chars().nth(at))
            .or_else(|| label.chars().find(|ch| ch.is_alphanumeric()))
            .map(|ch| ch.to_ascii_lowercase());
        self.mnemonics.push(mnemonic);

        let ui = &mut *self.ui;
        let theme = Theme::get(ui.ctx());
        let trigger_id = trigger_id(self.bar_id, index);
        let is_open = self.state.open == Some(index);
        let underline = self.state.active || ui.input(|input| input.modifiers.alt);

        let font = text::font_id(TypeRamp::Body1);
        let mut job = LayoutJob::default();
        let fg = if ui.is_enabled() {
            c(theme.color.neutral_foreground_1)
        } else {
            c(theme.color.neutral_foreground_disabled)
        };
        for (at, ch) in label.chars().enumerate() {
            let mut format = TextFormat::simple(font.clone(), fg);
            if underline && Some(at) == mnemonic_at {
                format.underline = Stroke::new(stroke_width::THIN, fg);
            }
            job.append(ch.encode_utf8(&mut [0; 4]), 0.0, format);
        }
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let height = crate::theme::density_of(ui).row(TITLE_HEIGHT);
        let size = Vec2::new(galley.size().x + H_PADDING * 2.0, height);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, trigger_id, Sense::click());

        if response.has_focus() {
            // The bar moves focus itself: arrows switch menus and Down opens one.
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    trigger_id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..EventFilter::default()
                    },
                );
            });
        }
        if self.state.focus_pending == Some(index) {
            response.request_focus();
            self.state.focus_pending = None;
        }

        if response.clicked() {
            if is_open {
                self.state.open = None;
            } else {
                self.state.open = Some(index);
                // A keyboard press (Enter or Space) puts focus on the first item.
                self.state.focus_first_item = response.has_focus() && self.state.active;
            }
        } else if self.state.open.is_some() && !is_open && response.hovered() {
            self.state.open = Some(index);
        }
        let is_open = self.state.open == Some(index);

        let fill = if !ui.is_enabled() {
            None
        } else if is_open || response.is_pointer_button_down_on() {
            Some(c(theme.color.subtle_background_pressed))
        } else if response.hovered() || response.has_focus() {
            Some(c(theme.color.subtle_background_hover))
        } else {
            None
        };
        if let Some(fill) = fill {
            ui.painter().add(RectShape::filled(rect, medium_radius(), fill));
        }
        let text_pos = egui::pos2(rect.left() + H_PADDING, rect.center().y - galley.size().y * 0.5);
        ui.painter().galley(text_pos, galley, fg);
        if focus::focus_visible(ui, &response) && !is_open {
            focus::paint_focus_ring(ui, rect, medium_radius());
        }

        let enabled = ui.is_enabled();
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &label));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_has_popup(accesskit::HasPopup::Menu);
            node.set_expanded(is_open);
            if let Some(key) = mnemonic {
                node.set_access_key(key.to_uppercase().collect::<String>());
            }
        });

        if !is_open {
            return InnerResponse::new(None, response);
        }

        let popup_id = popup_id(self.bar_id, index);
        let mut keep_open = true;
        // The first frame of a popup is an invisible sizing pass; move focus once it is shown.
        let focus_first = self.state.focus_first_item && ui.ctx().read_response(popup_id).is_some();
        if focus_first {
            self.state.focus_first_item = false;
        }
        let shown = Popup::menu(&response)
            .id(popup_id)
            .open_bool(&mut keep_open)
            .width(POPUP_MIN_WIDTH)
            .style(menu_contents_style)
            .frame(menu_frame(ui))
            .info(UiStackInfo::new(UiKind::Menu))
            .show(|ui| {
                ui.set_min_width(POPUP_MIN_WIDTH);
                if focus_first {
                    focus_first_item(ui.ctx());
                }
                ui.ctx().accesskit_node_builder(ui.scope_id(), |node| {
                    node.set_role(accesskit::Role::Menu);
                    node.set_label(label.clone());
                });
                let inner = add(ui);
                cancel_focus_request(ui.ctx());
                inner
            });
        if !keep_open {
            self.state.open = None;
            if ui.input(|input| input.key_pressed(Key::Escape)) && self.state.active {
                self.state.focus_pending = Some(index);
            } else {
                self.state.active = false;
                ui.memory_mut(|memory| memory.surrender_focus(trigger_id));
            }
        }
        InnerResponse::new(shown.map(|inner| inner.inner), response)
    }
}

#[derive(Clone, Default)]
struct State {
    /// The open menu.
    open: Option<usize>,
    /// The bar has keyboard focus (after Alt, F10 or a mnemonic); mnemonics are underlined.
    active: bool,
    /// Alt is held.
    alt_down: bool,
    /// Nothing else was pressed while Alt was held, so its release toggles the bar.
    alt_clean: bool,
    /// Give this title focus when it is next drawn.
    focus_pending: Option<usize>,
    /// Move focus into the menu once it is shown.
    focus_first_item: bool,
    /// Lower-case mnemonic per menu, from the last frame.
    mnemonics: Vec<Option<char>>,
}

impl State {
    fn load(ctx: &egui::Context, id: Id) -> Self {
        ctx.data(|data| data.get_temp(id)).unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|data| data.insert_temp(id, self));
    }

    fn open_menu(&mut self, index: usize) {
        self.open = Some(index);
        self.active = true;
        self.focus_pending = Some(index);
        self.focus_first_item = true;
    }
}

fn trigger_id(bar_id: Id, index: usize) -> Id {
    bar_id.with(("title", index))
}

fn popup_id(bar_id: Id, index: usize) -> Id {
    bar_id.with(("popup", index))
}

/// Keyboard handling that does not belong to one title: Alt and F10, mnemonics, and arrows.
/// Runs before the titles are drawn, against last frame's menu list.
fn handle_keys(ui: &Ui, bar_id: Id, state: &mut State) {
    let count = state.mnemonics.len();
    if count == 0 || !ui.is_enabled() {
        return;
    }
    let focused = ui.memory(|memory| memory.focused());
    let focused_title = (0..count).find(|&index| focused == Some(trigger_id(bar_id, index)));
    if state.active && focused_title.is_none() && state.focus_pending.is_none() && state.open.is_none() {
        // Focus left the bar (a click elsewhere, Tab): leave keyboard mode.
        state.active = false;
    }
    let submenu_open = state.open.is_some_and(|open| {
        !egui::containers::menu::MenuState::is_deepest_open_sub_menu(ui.ctx(), popup_id(bar_id, open))
    });

    let mut toggle_bar = false;
    let mut took_arrow = false;
    ui.input_mut(|input| {
        if input.pointer.any_pressed() {
            state.alt_clean = false;
        }
        // Walk modifier changes in order, so a press and release inside one frame still count.
        let mut pressed_keys = Vec::new();
        let mut track_alt = |state: &mut State, modifiers: Modifiers| {
            let alt = modifiers.alt && !modifiers.ctrl;
            if alt && !state.alt_down {
                state.alt_clean = true;
            } else if !alt && state.alt_down && state.alt_clean {
                toggle_bar = true;
            }
            state.alt_down = alt;
        };
        for event in &input.events {
            match event {
                egui::Event::ModifiersChanged(modifiers) => track_alt(state, *modifiers),
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    state.alt_clean = false;
                    pressed_keys.push((*key, *modifiers));
                }
                _ => {}
            }
        }
        track_alt(state, input.modifiers);

        for (key, modifiers) in pressed_keys {
            let letter = key_letter(key);
            if modifiers.alt && !modifiers.ctrl && !modifiers.command {
                if let Some(index) = letter.and_then(|ch| find_mnemonic(&state.mnemonics, ch)) {
                    input.consume_key(modifiers, key);
                    state.open_menu(index);
                }
                continue;
            }
            if key == Key::F10 && modifiers.is_none() {
                input.consume_key(Modifiers::NONE, key);
                toggle_bar = true;
                continue;
            }
            if !modifiers.is_none() || !state.active {
                continue;
            }
            match (state.open, key) {
                (Some(open), Key::ArrowLeft | Key::ArrowRight) if !submenu_open => {
                    input.consume_key(Modifiers::NONE, key);
                    took_arrow = true;
                    state.open_menu(step(open, count, key == Key::ArrowRight));
                }
                (None, Key::ArrowLeft | Key::ArrowRight) => {
                    if let Some(at) = focused_title {
                        input.consume_key(Modifiers::NONE, key);
                        state.focus_pending = Some(step(at, count, key == Key::ArrowRight));
                    }
                }
                (None, Key::ArrowDown) => {
                    if let Some(at) = focused_title {
                        input.consume_key(Modifiers::NONE, key);
                        state.open_menu(at);
                    }
                }
                (None, Key::Escape) => {
                    input.consume_key(Modifiers::NONE, key);
                    state.active = false;
                }
                (None, _) => {
                    if let Some(index) = letter.and_then(|ch| find_mnemonic(&state.mnemonics, ch)) {
                        input.consume_key(Modifiers::NONE, key);
                        state.open_menu(index);
                    }
                }
                _ => {}
            }
        }
    });

    if took_arrow {
        // egui would also move focus sideways out of the item; the bar places it instead.
        ui.memory_mut(|memory| memory.move_focus(FocusDirection::None));
    }
    if toggle_bar {
        if state.active || state.open.is_some() {
            state.open = None;
            state.active = false;
            if let Some(at) = focused_title {
                ui.memory_mut(|memory| memory.surrender_focus(trigger_id(bar_id, at)));
            }
        } else {
            state.active = true;
            state.focus_pending = Some(0);
        }
    }
    if !state.active
        && let Some(at) = focused_title
    {
        ui.memory_mut(|memory| memory.surrender_focus(trigger_id(bar_id, at)));
    }
}

fn step(from: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        0
    } else if forward {
        (from + 1) % count
    } else {
        (from + count - 1) % count
    }
}

fn find_mnemonic(mnemonics: &[Option<char>], ch: char) -> Option<usize> {
    mnemonics.iter().position(|m| *m == Some(ch))
}

/// The lower-case letter or digit a key types, for mnemonic matching.
fn key_letter(key: Key) -> Option<char> {
    let name = key.symbol_or_name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_lowercase()),
        _ => None,
    }
}

/// Strip `&` markers from `title`. Returns the display label and the char index of the
/// mnemonic, if one was marked. `&&` is a literal `&`.
fn parse_mnemonic(title: &str) -> (String, Option<usize>) {
    let mut label = String::with_capacity(title.len());
    let mut mnemonic = None;
    let mut count = 0;
    let mut chars = title.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            match chars.peek() {
                Some('&') => {
                    chars.next();
                }
                Some(_) => {
                    if mnemonic.is_none() {
                        mnemonic = Some(count);
                    }
                    continue;
                }
                None => continue,
            }
        }
        label.push(ch);
        count += 1;
    }
    (label, mnemonic)
}

fn medium_radius() -> CornerRadius {
    CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::{parse_mnemonic, step};

    #[test]
    fn ampersand_marks_the_mnemonic() {
        assert_eq!(parse_mnemonic("&File"), ("File".to_owned(), Some(0)));
        assert_eq!(parse_mnemonic("E&xit"), ("Exit".to_owned(), Some(1)));
        assert_eq!(
            parse_mnemonic("Save && &Close"),
            ("Save & Close".to_owned(), Some(7))
        );
        assert_eq!(parse_mnemonic("Help"), ("Help".to_owned(), None));
        assert_eq!(parse_mnemonic("Trailing&"), ("Trailing".to_owned(), None));
    }

    #[test]
    fn arrows_wrap() {
        assert_eq!(step(0, 3, false), 2);
        assert_eq!(step(2, 3, true), 0);
        assert_eq!(step(0, 0, true), 0);
    }
}
