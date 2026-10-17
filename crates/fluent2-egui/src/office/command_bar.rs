//! Command bar with priority overflow (spec §8.2, WI-1569).
//!
//! The fitting is a pure function over item widths and priorities, so it can be unit-tested
//! without a window. [`CommandBar`] measures the labels, calls [`fit`] and draws the visible
//! items plus a trailing `More` menu.

use egui::{Id, Ui, WidgetText, accesskit};
use fluent2_icons::Icon;

use crate::controls::button::Metrics as ButtonMetrics;
use crate::controls::split_button::{self, ensure_type_ramp};
use crate::controls::{Button, ButtonSize, MenuButton};
use crate::surfaces::MenuItem;
use crate::theme::Theme;

/// One command on a [`CommandBar`].
#[derive(Debug, Clone, Copy)]
pub struct CommandItem<'a> {
    /// Names the command in responses and ids. Duplicate keys within one bar: the first wins.
    pub key: &'static str,
    /// Icon shown before the label.
    pub icon: Icon,
    /// Visible text, also used for the `More` menu entry.
    pub label: &'a str,
    /// Higher stays visible longer. 0 to 255.
    pub priority: u8,
}

/// What a [`CommandBar`] reports for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandBarResponse {
    /// The key of the command clicked this frame, visible or in the `More` menu.
    pub activated: Option<&'static str>,
    /// Whether the `More` menu is open this frame.
    pub more_open: bool,
}

/// A horizontal bar of commands. Items that do not fit move into a trailing `More` menu.
///
/// ```
/// # use fluent2_egui::office::command_bar::{CommandBar, CommandItem};
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// let items = [
///     CommandItem { key: "save", icon: Icon::Save, label: "Save", priority: 90 },
///     CommandItem { key: "print", icon: Icon::Print, label: "Print", priority: 10 },
/// ];
/// let response = CommandBar::new("commands", &items).show(ui);
/// if response.activated == Some("save") {
///     // save
/// }
/// # }
/// ```
pub struct CommandBar<'a> {
    id: Id,
    items: &'a [CommandItem<'a>],
}

impl<'a> CommandBar<'a> {
    /// A bar over `items`. `id` keeps the bar's remembered visible count apart from other bars.
    #[must_use]
    pub fn new(id: impl std::hash::Hash, items: &'a [CommandItem<'a>]) -> Self {
        Self {
            id: super::chrome_id(id),
            items,
        }
    }

    /// Draws the bar and, when something overflows, the `More` menu.
    pub fn show(self, ui: &mut Ui) -> CommandBarResponse {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let chrome = theme.chrome;

        // The first item with each key wins, so ids stay unique.
        let mut seen: Vec<&'static str> = Vec::new();
        let mut unique: Vec<&CommandItem<'a>> = Vec::new();
        for item in self.items {
            if !seen.contains(&item.key) {
                seen.push(item.key);
                unique.push(item);
            }
        }

        let widths: Vec<(f32, u8)> = unique
            .iter()
            .map(|item| (button_width(ui, item.label), item.priority))
            .collect();
        let more_width = more_menu_width(ui);

        let remembered = self.id.with("visible_count");
        let previous = ui
            .ctx()
            .data(|data| data.get_temp::<usize>(remembered))
            .unwrap_or(unique.len());
        let layout = fit(
            &widths,
            ui.available_width(),
            chrome.command_bar_gap,
            more_width,
            chrome.hysteresis,
            previous,
        );
        ui.ctx()
            .data_mut(|data| data.insert_temp(remembered, layout.visible.len()));

        let mut activated = None;
        let mut more_open = false;
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = chrome.command_bar_gap;
            for &index in &layout.visible {
                let Some(item) = unique.get(index) else { continue };
                let response = ui.add(Button::new(item.label).icon(item.icon).id_salt(item.key));
                if response.clicked() {
                    activated = Some(item.key);
                }
            }
            if !layout.overflow.is_empty() {
                let (_, picked) = MenuButton::new("More")
                    .icon(Icon::MoreHorizontal)
                    .id_salt(self.id.with("more"))
                    .show(ui, |ui| {
                        for &index in &layout.overflow {
                            let Some(item) = unique.get(index) else { continue };
                            let response =
                                ui.add(MenuItem::new(item.label).icon(item.icon).id_salt(item.key));
                            if response.clicked() {
                                activated = Some(item.key);
                            }
                        }
                    });
                more_open = picked.is_some();
            }
        });
        ui.ctx().accesskit_node_builder(row.response.id, |node| {
            node.set_role(accesskit::Role::Toolbar);
            node.set_label("Command bar");
        });

        CommandBarResponse { activated, more_open }
    }
}

/// Width `Button` gives a command with an icon and a label (spec §5.4). It uses the same
/// metrics and galley as the button, so the fit matches what is drawn.
fn button_width(ui: &Ui, label: &str) -> f32 {
    let metrics = ButtonMetrics::of(ButtonSize::default());
    let text = WidgetText::from(label)
        .into_galley(ui, None, f32::INFINITY, metrics.text_style())
        .size()
        .x;
    (text + metrics.icon + metrics.icon_gap + metrics.h_padding * 2.0).max(metrics.min_width)
}

/// Width of the trailing `More` `MenuButton`: its label, icon and chevron (spec §8.2).
fn more_menu_width(ui: &Ui) -> f32 {
    let metrics = split_button::Metrics::of(ButtonSize::default());
    split_button::natural_width(ui, &WidgetText::from("More"), true, metrics)
        + metrics.icon
        + metrics.icon_gap
}

/// Which items stay on the bar and which move to the `More` menu.
///
/// Both lists hold indices into the slice given to [`fit`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fit {
    /// Items that stay on the bar, in their input order.
    pub visible: Vec<usize>,
    /// Items that move to the `More` menu, highest priority first. Ties keep input order.
    pub overflow: Vec<usize>,
}

/// Decides which items fit on a command bar (spec §8.2).
///
/// `items` holds `(width, priority)` pairs in input order, in points. A higher priority stays
/// visible longer. Rules:
///
/// 1. Items are ordered by priority, highest first. Ties keep their input order.
/// 2. The highest-priority items are kept while their total width, plus `gap` between items,
///    fits in `available`. The first item that does not fit ends the visible run, so a
///    lower-priority item never jumps ahead of a higher one.
/// 3. The `More` button (`more_width`) is reserved whenever at least one item overflows.
/// 4. An item that was overflowed last frame comes back only when `hysteresis` points of spare
///    room remain beyond its width. `previous_visible` is the visible count last frame; pass
///    `items.len()` when there is no previous frame.
///
/// Non-finite or negative widths, gaps and `available` are treated as zero. A zero or negative
/// `available` moves every item to overflow.
///
/// ```
/// use fluent2_egui::office::command_bar::fit;
///
/// let items = [(40.0, 10), (40.0, 90), (40.0, 50)];
/// let fit = fit(&items, 110.0, 4.0, 20.0, 8.0, items.len());
/// assert_eq!(fit.visible, vec![1, 2]);
/// assert_eq!(fit.overflow, vec![0]);
/// ```
pub fn fit(
    items: &[(f32, u8)],
    available: f32,
    gap: f32,
    more_width: f32,
    hysteresis: f32,
    previous_visible: usize,
) -> Fit {
    let count = items.len();
    if count == 0 {
        return Fit::default();
    }
    let available = clean(available);
    let gap = clean(gap);
    let more_width = clean(more_width);
    let hysteresis = clean(hysteresis);

    let mut order: Vec<usize> = (0..count).collect();
    // `sort_by_key` is stable, so equal priorities keep their input order.
    order.sort_by_key(|&index| std::cmp::Reverse(items.get(index).map_or(0, |item| item.1)));

    // `prefix[k]` is the width of the k highest-priority items, without gaps.
    let mut prefix = Vec::with_capacity(count + 1);
    prefix.push(0.0_f32);
    for &index in &order {
        let width = items.get(index).map_or(0.0, |item| clean(item.0));
        prefix.push(prefix.last().copied().unwrap_or(0.0) + width);
    }
    let row = |k: usize| prefix.get(k).copied().unwrap_or(0.0) + gap * k.saturating_sub(1) as f32;
    // The row with `More` after the k highest-priority items.
    let row_with_more = |k: usize| row(k) + if k > 0 { gap } else { 0.0 } + more_width;

    let all_fit_slack = if previous_visible >= count {
        0.0
    } else {
        hysteresis
    };
    let visible_count = if row(count) + all_fit_slack <= available {
        count
    } else {
        // `row_with_more` grows with k, so the k that fit form a prefix of 1..count.
        let fits_plain = (1..count).filter(|&k| row_with_more(k) <= available).count();
        let fits_with_spare = (1..count)
            .filter(|&k| row_with_more(k) + hysteresis <= available)
            .count();
        if previous_visible >= fits_plain {
            fits_plain
        } else {
            previous_visible.max(fits_with_spare)
        }
    };

    let mut visible: Vec<usize> = order.iter().take(visible_count).copied().collect();
    visible.sort_unstable();
    let overflow = order.iter().skip(visible_count).copied().collect();
    Fit { visible, overflow }
}

/// Clamps a width, gap or bound to a finite, non-negative value.
fn clean(value: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{Fit, fit};

    fn visible_and_overflow(items: &[(f32, u8)], available: f32) -> (Vec<usize>, Vec<usize>) {
        let fit = fit(items, available, 4.0, 20.0, 8.0, items.len());
        (fit.visible, fit.overflow)
    }

    #[test]
    fn all_fit_has_no_more_button_space_reserved() {
        // 10 + 4 + 10 = 24 fits in 24; no overflow.
        let items = [(10.0, 1), (10.0, 2)];
        assert_eq!(
            fit(&items, 24.0, 4.0, 20.0, 8.0, 2),
            Fit {
                visible: vec![0, 1],
                overflow: vec![],
            }
        );
    }

    #[test]
    fn lowest_priority_moves_to_overflow_first() {
        // Widths 40 each, gap 4, More 20. Two items plus More need 40+4+40+4+20 = 108.
        let items = [(40.0, 10), (40.0, 90), (40.0, 50)];
        let (visible, overflow) = visible_and_overflow(&items, 110.0);
        assert_eq!(visible, vec![1, 2]);
        assert_eq!(overflow, vec![0]);
    }

    #[test]
    fn overflow_is_listed_in_priority_order() {
        let items = [(30.0, 5), (30.0, 80), (30.0, 20), (30.0, 60)];
        // Room for one item plus More only.
        let (visible, overflow) = visible_and_overflow(&items, 60.0);
        assert_eq!(visible, vec![1]);
        assert_eq!(overflow, vec![3, 2, 0]);
    }

    #[test]
    fn zero_width_moves_everything_to_overflow() {
        let items = [(10.0, 1), (10.0, 2)];
        let (visible, overflow) = visible_and_overflow(&items, 0.0);
        assert!(visible.is_empty());
        assert_eq!(overflow, vec![1, 0]);
    }

    #[test]
    fn negative_and_nan_available_are_zero() {
        let items = [(10.0, 1)];
        assert!(visible_and_overflow(&items, -5.0).0.is_empty());
        assert!(visible_and_overflow(&items, f32::NAN).0.is_empty());
    }

    #[test]
    fn empty_input_is_empty() {
        assert_eq!(fit(&[], 100.0, 4.0, 20.0, 8.0, 0), Fit::default());
    }

    #[test]
    fn ties_keep_input_order() {
        let items = [(20.0, 50), (20.0, 50), (20.0, 50)];
        // Room for one item plus More: the first in input order stays.
        let (visible, overflow) = visible_and_overflow(&items, 50.0);
        assert_eq!(visible, vec![0]);
        assert_eq!(overflow, vec![1, 2]);
    }

    #[test]
    fn a_wide_high_priority_item_blocks_cheaper_lower_ones() {
        // The 100 pt item is highest priority but does not fit with More, so nothing after
        // it may jump ahead, even though the 10 pt item would fit.
        let items = [(10.0, 1), (100.0, 9)];
        let (visible, overflow) = visible_and_overflow(&items, 50.0);
        assert!(visible.is_empty());
        assert_eq!(overflow, vec![1, 0]);
    }

    #[test]
    fn hysteresis_keeps_an_overflowed_item_out_until_there_is_spare_room() {
        let items = [(40.0, 90), (40.0, 10)];
        // Last frame only the first item was visible. Two items need 40+4+40 = 84 with no More,
        // and an overflowed item needs 8 pt spare, so 84 + 8 = 92 is required to bring it back.
        let just_short = fit(&items, 90.0, 4.0, 20.0, 8.0, 1);
        assert_eq!(just_short.visible, vec![0]);
        let spare = fit(&items, 92.0, 4.0, 20.0, 8.0, 1);
        assert_eq!(spare.visible, vec![0, 1]);
        assert!(spare.overflow.is_empty());
    }

    #[test]
    fn hysteresis_does_not_delay_items_leaving_the_bar() {
        let items = [(40.0, 90), (40.0, 10)];
        // Both were visible; shrinking to 83 (below the 84 they need) must move the second one
        // out at once, with no hysteresis.
        let fit = fit(&items, 83.0, 4.0, 20.0, 8.0, 2);
        assert_eq!(fit.visible, vec![0]);
        assert_eq!(fit.overflow, vec![1]);
    }

    #[test]
    fn non_finite_widths_count_as_zero() {
        let items = [(f32::NAN, 1), (f32::INFINITY, 2)];
        let fit = fit(&items, 10.0, 0.0, 0.0, 0.0, 2);
        assert_eq!(fit.visible, vec![0, 1]);
    }
}
