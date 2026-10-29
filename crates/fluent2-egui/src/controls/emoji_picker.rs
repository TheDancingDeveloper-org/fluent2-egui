//! Emoji picker: search, categories, recent emoji, keyboard and AccessKit.
//!
//! The catalogue is a fixed subset of Unicode emoji (names and categories only).
//! Colour comes from the bundled Noto COLR face when the text stack can draw it;
//! otherwise the same code points still layout through the missing-glyph policy
//! and never panic.

use egui::{Id, Key, Modifiers, Response, ScrollArea, Sense, Ui, Vec2, Widget};

use crate::controls::search_box::SearchBox;
use crate::foundation::text::apply_missing_glyph_policy;
use crate::theme::{Theme, c};

/// How many recent emoji the picker remembers.
pub const RECENT_LIMIT: usize = 16;

/// A Unicode emoji group, in picker order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmojiCategory {
    /// Faces and emotion.
    Smileys,
    /// Hands, people and body.
    People,
    /// Animals and nature.
    Animals,
    /// Food and drink.
    Food,
    /// Travel and places.
    Travel,
    /// Activities and sports.
    Activities,
    /// Objects.
    Objects,
    /// Symbols.
    Symbols,
}

impl EmojiCategory {
    /// Every category, in display order.
    pub const ALL: [Self; 8] = [
        Self::Smileys,
        Self::People,
        Self::Animals,
        Self::Food,
        Self::Travel,
        Self::Activities,
        Self::Objects,
        Self::Symbols,
    ];

    /// Short label drawn on the category chip and read by AccessKit.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Smileys => "Smileys",
            Self::People => "People",
            Self::Animals => "Animals",
            Self::Food => "Food",
            Self::Travel => "Travel",
            Self::Activities => "Activities",
            Self::Objects => "Objects",
            Self::Symbols => "Symbols",
        }
    }
}

/// One catalogue entry.
#[derive(Clone, Copy, Debug)]
pub struct EmojiEntry {
    /// The emoji (and any variation selector) as it is inserted.
    pub emoji: &'static str,
    /// English name, also the search text and the accessible name.
    pub name: &'static str,
    /// Category chip this entry belongs to.
    pub category: EmojiCategory,
}

/// The catalogue the picker searches. Order inside a category is stable.
pub static EMOJI: &[EmojiEntry] = &[
    entry("😀", "grinning face", EmojiCategory::Smileys),
    entry("😃", "grinning face with big eyes", EmojiCategory::Smileys),
    entry("😄", "grinning face with smiling eyes", EmojiCategory::Smileys),
    entry("😁", "beaming face with smiling eyes", EmojiCategory::Smileys),
    entry("😅", "grinning face with sweat", EmojiCategory::Smileys),
    entry("😂", "face with tears of joy", EmojiCategory::Smileys),
    entry("🤣", "rolling on the floor laughing", EmojiCategory::Smileys),
    entry("😊", "smiling face with smiling eyes", EmojiCategory::Smileys),
    entry("😇", "smiling face with halo", EmojiCategory::Smileys),
    entry("🙂", "slightly smiling face", EmojiCategory::Smileys),
    entry("😉", "winking face", EmojiCategory::Smileys),
    entry("😍", "smiling face with heart-eyes", EmojiCategory::Smileys),
    entry("😘", "face blowing a kiss", EmojiCategory::Smileys),
    entry("😋", "face savoring food", EmojiCategory::Smileys),
    entry("😜", "winking face with tongue", EmojiCategory::Smileys),
    entry("🤔", "thinking face", EmojiCategory::Smileys),
    entry("😐", "neutral face", EmojiCategory::Smileys),
    entry("😑", "expressionless face", EmojiCategory::Smileys),
    entry("😏", "smirking face", EmojiCategory::Smileys),
    entry("😒", "unamused face", EmojiCategory::Smileys),
    entry("😔", "pensive face", EmojiCategory::Smileys),
    entry("😴", "sleeping face", EmojiCategory::Smileys),
    entry("😷", "face with medical mask", EmojiCategory::Smileys),
    entry("🤒", "face with thermometer", EmojiCategory::Smileys),
    entry("😎", "smiling face with sunglasses", EmojiCategory::Smileys),
    entry("😢", "crying face", EmojiCategory::Smileys),
    entry("😭", "loudly crying face", EmojiCategory::Smileys),
    entry("😱", "face screaming in fear", EmojiCategory::Smileys),
    entry("😡", "enraged face", EmojiCategory::Smileys),
    entry("👍", "thumbs up", EmojiCategory::People),
    entry("👎", "thumbs down", EmojiCategory::People),
    entry("👏", "clapping hands", EmojiCategory::People),
    entry("🙌", "raising hands", EmojiCategory::People),
    entry("👋", "waving hand", EmojiCategory::People),
    entry("🤝", "handshake", EmojiCategory::People),
    entry("🙏", "folded hands", EmojiCategory::People),
    entry("💪", "flexed biceps", EmojiCategory::People),
    entry("👀", "eyes", EmojiCategory::People),
    entry("🐶", "dog face", EmojiCategory::Animals),
    entry("🐱", "cat face", EmojiCategory::Animals),
    entry("🐭", "mouse face", EmojiCategory::Animals),
    entry("🐰", "rabbit face", EmojiCategory::Animals),
    entry("🦊", "fox face", EmojiCategory::Animals),
    entry("🐻", "bear face", EmojiCategory::Animals),
    entry("🐼", "panda face", EmojiCategory::Animals),
    entry("🦁", "lion face", EmojiCategory::Animals),
    entry("🐸", "frog", EmojiCategory::Animals),
    entry("🐵", "monkey face", EmojiCategory::Animals),
    entry("🍕", "pizza", EmojiCategory::Food),
    entry("🍔", "hamburger", EmojiCategory::Food),
    entry("🍟", "french fries", EmojiCategory::Food),
    entry("🌮", "taco", EmojiCategory::Food),
    entry("🍣", "sushi", EmojiCategory::Food),
    entry("🍩", "doughnut", EmojiCategory::Food),
    entry("🍪", "cookie", EmojiCategory::Food),
    entry("🍰", "shortcake", EmojiCategory::Food),
    entry("☕", "hot beverage", EmojiCategory::Food),
    entry("🍎", "red apple", EmojiCategory::Food),
    entry("🚗", "automobile", EmojiCategory::Travel),
    entry("🚕", "taxi", EmojiCategory::Travel),
    entry("🚌", "bus", EmojiCategory::Travel),
    entry("✈️", "airplane", EmojiCategory::Travel),
    entry("🚀", "rocket", EmojiCategory::Travel),
    entry("🚲", "bicycle", EmojiCategory::Travel),
    entry("⛵", "sailboat", EmojiCategory::Travel),
    entry("🏠", "house", EmojiCategory::Travel),
    entry("🌍", "globe showing Europe-Africa", EmojiCategory::Travel),
    entry("⛅", "sun behind cloud", EmojiCategory::Travel),
    entry("⚽", "soccer ball", EmojiCategory::Activities),
    entry("🏀", "basketball", EmojiCategory::Activities),
    entry("🏈", "american football", EmojiCategory::Activities),
    entry("🎾", "tennis", EmojiCategory::Activities),
    entry("🎮", "video game", EmojiCategory::Activities),
    entry("🎲", "game die", EmojiCategory::Activities),
    entry("🎯", "bullseye", EmojiCategory::Activities),
    entry("🎨", "artist palette", EmojiCategory::Activities),
    entry("🎵", "musical note", EmojiCategory::Activities),
    entry("🎸", "guitar", EmojiCategory::Activities),
    entry("⌚", "watch", EmojiCategory::Objects),
    entry("📱", "mobile phone", EmojiCategory::Objects),
    entry("💻", "laptop", EmojiCategory::Objects),
    entry("⌨️", "keyboard", EmojiCategory::Objects),
    entry("📷", "camera", EmojiCategory::Objects),
    entry("📚", "books", EmojiCategory::Objects),
    entry("✏️", "pencil", EmojiCategory::Objects),
    entry("📌", "pushpin", EmojiCategory::Objects),
    entry("🔑", "key", EmojiCategory::Objects),
    entry("💡", "light bulb", EmojiCategory::Objects),
    entry("❤️", "red heart", EmojiCategory::Symbols),
    entry("💛", "yellow heart", EmojiCategory::Symbols),
    entry("💚", "green heart", EmojiCategory::Symbols),
    entry("💙", "blue heart", EmojiCategory::Symbols),
    entry("⭐", "star", EmojiCategory::Symbols),
    entry("✅", "check mark button", EmojiCategory::Symbols),
    entry("❌", "cross mark", EmojiCategory::Symbols),
    entry("⚠️", "warning", EmojiCategory::Symbols),
    entry("❓", "question mark", EmojiCategory::Symbols),
    entry("♻️", "recycling symbol", EmojiCategory::Symbols),
];

const fn entry(emoji: &'static str, name: &'static str, category: EmojiCategory) -> EmojiEntry {
    EmojiEntry {
        emoji,
        name,
        category,
    }
}

/// What the picker did this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmojiPickerResponse {
    /// Set when the user picked an emoji (click or Enter / Space on the active cell).
    pub picked: Option<String>,
}

/// Recent emoji for `id`, newest first. Empty when nothing has been picked yet.
pub fn recent_emoji(ctx: &egui::Context, id: Id) -> Vec<String> {
    ctx.data_mut(|data| data.get_persisted(recent_key(id)).unwrap_or_default())
}

fn recent_key(id: Id) -> Id {
    id.with("emoji_recent")
}

fn remember(ctx: &egui::Context, id: Id, emoji: &str) {
    let key = recent_key(id);
    let mut recent: Vec<String> = ctx.data_mut(|data| data.get_persisted(key).unwrap_or_default());
    recent.retain(|item| item != emoji);
    recent.insert(0, emoji.to_owned());
    recent.truncate(RECENT_LIMIT);
    ctx.data_mut(|data| data.insert_persisted(key, recent));
}

/// Every catalogue entry matching `query`, across categories.
pub fn filter_emoji_all(query: &str) -> Vec<&'static EmojiEntry> {
    let needle = query.trim().to_lowercase();
    EMOJI
        .iter()
        .filter(|entry| {
            needle.is_empty()
                || entry.name.to_lowercase().contains(&needle)
                || entry.emoji.contains(query.trim())
        })
        .collect()
}

/// Entries matching `query` (case-insensitive substring of the name or the emoji)
/// inside `category`. An empty query keeps the whole category.
pub fn filter_emoji(category: EmojiCategory, query: &str) -> Vec<&'static EmojiEntry> {
    let needle = query.trim().to_lowercase();
    EMOJI
        .iter()
        .filter(|entry| entry.category == category)
        .filter(|entry| {
            needle.is_empty()
                || entry.name.to_lowercase().contains(&needle)
                || entry.emoji.contains(query.trim())
        })
        .collect()
}

/// A popover body: search field, category chips, an optional recent row, and a grid.
///
/// ```
/// # use fluent2_egui::controls::EmojiPicker;
/// # fn demo(ui: &mut egui::Ui) {
/// let response = EmojiPicker::new("compose").show(ui);
/// if let Some(emoji) = response.picked {
///     let _ = emoji;
/// }
/// # }
/// ```
pub struct EmojiPicker {
    id_salt: Id,
}

impl EmojiPicker {
    /// `id_salt` names the picker. It is never taken from a translated label.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
        }
    }

    /// Draw the picker and return a pick, if any.
    pub fn show(self, ui: &mut Ui) -> EmojiPickerResponse {
        apply_missing_glyph_policy(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let query_id = id.with("query");
        let category_id = id.with("category");
        let active_id = id.with("active");

        let mut query: String = ui
            .ctx()
            .data_mut(|data| data.get_persisted(query_id).unwrap_or_default());
        let mut category: EmojiCategory = ui.ctx().data_mut(|data| {
            let index: usize = data.get_persisted(category_id).unwrap_or(0);
            EmojiCategory::ALL
                .get(index)
                .copied()
                .unwrap_or(EmojiCategory::Smileys)
        });
        // A query searches every category. The chips still switch the unfiltered grid.
        let browsing = query.trim().is_empty();
        let mut active: usize = ui
            .ctx()
            .data_mut(|data| data.get_persisted(active_id).unwrap_or(0));

        let search = ui.add(
            SearchBox::new(&mut query)
                .placeholder("Search emoji")
                .label("Search emoji")
                .id_salt(id.with("search")),
        );

        ui.horizontal_wrapped(|ui| {
            for item in EmojiCategory::ALL {
                let selected = item == category;
                let button = egui::Button::new(item.label()).selected(selected);
                let response = ui.add(button);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), item.label())
                });
                if response.clicked() {
                    category = item;
                    active = 0;
                }
            }
        });

        let recent = recent_emoji(ui.ctx(), id);
        if !recent.is_empty() && query.trim().is_empty() {
            ui.label("Recent");
            ui.horizontal_wrapped(|ui| {
                for emoji in &recent {
                    if emoji_button(ui, emoji, emoji).clicked() {
                        remember(ui.ctx(), id, emoji);
                        // A recent pick is reported through the same path as the grid: stash it.
                        ui.ctx()
                            .data_mut(|data| data.insert_temp(id.with("picked"), emoji.clone()));
                    }
                }
            });
        }

        let entries = if browsing {
            filter_emoji(category, &query)
        } else {
            filter_emoji_all(&query)
        };
        if active >= entries.len() {
            active = 0;
        }
        let columns = 8usize;
        let mut picked = ui.ctx().data(|data| data.get_temp::<String>(id.with("picked")));
        // The search field owns keys while it is focused. Otherwise arrows, Home, End,
        // Enter and Space move the active cell and insert it.
        let grid_keys = !search.has_focus();
        if grid_keys && !entries.is_empty() {
            let input = ui.input(|input| input.clone());
            let count = entries.len();
            if input.key_pressed(Key::ArrowRight) {
                active = (active + 1) % count;
            }
            if input.key_pressed(Key::ArrowLeft) {
                active = (active + count - 1) % count;
            }
            if input.key_pressed(Key::ArrowDown) {
                active = (active + columns).min(count - 1);
            }
            if input.key_pressed(Key::ArrowUp) {
                active = active.saturating_sub(columns);
            }
            if input.key_pressed(Key::Home) {
                active = 0;
            }
            if input.key_pressed(Key::End) {
                active = count - 1;
            }
            if input.key_pressed(Key::Enter) || input.key_pressed(Key::Space) {
                picked = Some(entries[active].emoji.to_owned());
                remember(ui.ctx(), id, entries[active].emoji);
                ui.input_mut(|input| {
                    input.consume_key(Modifiers::NONE, Key::Enter);
                    input.consume_key(Modifiers::NONE, Key::Space);
                });
            }
        }

        ScrollArea::vertical()
            .id_salt(id.with("grid"))
            .max_height(220.0)
            .show(ui, |ui| {
                if entries.is_empty() {
                    ui.label("No emoji");
                    return;
                }
                let theme = Theme::get(ui.ctx());
                ui.horizontal_wrapped(|ui| {
                    for (index, entry) in entries.iter().enumerate() {
                        let response = emoji_button(ui, entry.emoji, entry.name);
                        if index == active {
                            ui.painter().rect_stroke(
                                response.rect.expand(1.0),
                                4.0,
                                egui::Stroke::new(1.0, c(theme.color.stroke_focus_2)),
                                egui::StrokeKind::Outside,
                            );
                        }
                        if response.clicked() {
                            picked = Some(entry.emoji.to_owned());
                            active = index;
                            remember(ui.ctx(), id, entry.emoji);
                        }
                        if response.has_focus() {
                            active = index;
                        }
                    }
                });
            });

        ui.ctx().data_mut(|data| {
            data.insert_persisted(query_id, query);
            data.insert_persisted(
                category_id,
                EmojiCategory::ALL
                    .iter()
                    .position(|item| *item == category)
                    .unwrap_or(0),
            );
            data.insert_persisted(active_id, active);
            data.remove::<String>(id.with("picked"));
        });

        EmojiPickerResponse { picked }
    }
}

impl Widget for EmojiPicker {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = ui.scope_id().with(self.id_salt);
        let response = self.show(ui);
        // The widget response is the search field's neighbour: report a pick as a change
        // on a focusable strip so callers using `ui.add` still see `changed`.
        let width = ui.available_width().max(1.0);
        let (_, mut strip) = ui.allocate_exact_size(Vec2::new(width, 0.0), Sense::hover());
        if response.picked.is_some() {
            strip.mark_changed();
        }
        ui.ctx().data_mut(|data| {
            if let Some(emoji) = response.picked.clone() {
                data.insert_temp(id.with("last_pick"), emoji);
            }
        });
        strip
    }
}

fn emoji_button(ui: &mut Ui, emoji: &str, name: &str) -> Response {
    let button = egui::Button::new(emoji).min_size(Vec2::splat(32.0));
    let response = ui.add(button);
    let accessible = name.to_owned();
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), accessible.clone()));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_entries_and_search_is_case_insensitive() {
        for category in EmojiCategory::ALL {
            assert!(!filter_emoji(category, "").is_empty(), "{category:?}");
        }
        let hits = filter_emoji(EmojiCategory::Smileys, "GRIN");
        assert!(hits.iter().any(|entry| entry.emoji == "😀"));
        assert!(filter_emoji(EmojiCategory::Food, "no-such-emoji").is_empty());
        assert!(
            filter_emoji(EmojiCategory::Symbols, "⚠")
                .iter()
                .any(|e| e.name == "warning")
        );
    }

    #[test]
    fn catalogue_names_are_unique() {
        let mut names: Vec<_> = EMOJI.iter().map(|entry| entry.name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before);
    }
}
