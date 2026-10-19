//! Pager: First, Previous, page numbers with ellipses, Next and Last, and an optional page-size
//! Dropdown. Built from [`Button`] and [`Dropdown`]; there is no Fluent 2 pager component.
//!
//! Pages are 0-based in the API and shown 1-based. The window of page numbers always shows the
//! first and last page, the current page with `siblings` pages on each side, and an ellipsis for
//! each gap of two or more pages. It keeps the same number of slots as the current page moves,
//! so the buttons do not shift under the pointer.

use egui::{Id, Response, Ui};
use fluent2_icons::Icon;

use super::button::Metrics;
use super::{Appearance, Button, ButtonSize, Dropdown};

/// One slot in the page window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageItem {
    /// A page button (0-based page).
    Page(usize),
    /// A gap of skipped pages.
    Ellipsis,
}

/// The page buttons to show for `current` of `page_count` pages, with `siblings` pages each
/// side of the current one.
///
/// Every window has the same number of slots, `2 * siblings + 5`, once `page_count` is at least
/// that: first, an ellipsis or page, the siblings and current page, an ellipsis or page, last.
/// Fewer pages are all shown. `current` past the end is clamped; 0 pages give an empty window.
///
/// ```
/// use fluent2_egui::controls::{PageItem::{Ellipsis, Page}, page_window};
///
/// assert_eq!(page_window(5, 10, 1), vec![Page(0), Ellipsis, Page(4), Page(5), Page(6), Ellipsis, Page(9)]);
/// assert_eq!(page_window(0, 3, 1), vec![Page(0), Page(1), Page(2)]);
/// assert!(page_window(0, 0, 1).is_empty());
/// ```
#[must_use]
pub fn page_window(current: usize, page_count: usize, siblings: usize) -> Vec<PageItem> {
    if page_count == 0 {
        return Vec::new();
    }
    let last = page_count - 1;
    let current = current.min(last);
    let siblings = siblings.min(page_count);
    let slots = siblings.saturating_mul(2).saturating_add(5);
    if page_count <= slots {
        return (0..page_count).map(PageItem::Page).collect();
    }
    // The middle band of 2 * siblings + 1 pages, kept clear of the first two and last two slots.
    let band = 2 * siblings + 1;
    let start = current.saturating_sub(siblings).clamp(2, last - 1 - band);
    let end = start + band; // exclusive
    let mut items = Vec::with_capacity(slots);
    items.push(PageItem::Page(0));
    items.push(if start == 2 {
        PageItem::Page(1)
    } else {
        PageItem::Ellipsis
    });
    items.extend((start..end).map(PageItem::Page));
    items.push(if end == last - 1 {
        PageItem::Page(last - 1)
    } else {
        PageItem::Ellipsis
    });
    items.push(PageItem::Page(last));
    items
}

/// Pages needed for `items` at `page_size` per page (at least 1 per page). 0 items is 0 pages.
///
/// ```
/// assert_eq!(fluent2_egui::controls::page_count(101, 25), 5);
/// assert_eq!(fluent2_egui::controls::page_count(0, 25), 0);
/// ```
#[must_use]
pub fn page_count(items: usize, page_size: usize) -> usize {
    items.div_ceil(page_size.max(1))
}

/// What a Pager reported this frame.
#[derive(Debug)]
pub struct PagerResponse {
    /// The row of controls.
    pub response: Response,
    /// The new page, when the user moved to another one this frame.
    pub page: Option<usize>,
    /// The new page size, when the user picked one this frame. The page is then the one holding
    /// the first item of the old page, also reported in [`Self::page`].
    pub page_size: Option<usize>,
}

/// A pager over `page_count` pages. Build it, then call [`Pager::show`].
///
/// ```
/// use fluent2_egui::controls::{Pager, page_count};
///
/// # fn demo(ui: &mut egui::Ui, page: &mut usize, page_size: &mut usize, rows: usize) {
/// let response = Pager::new("audit", page, page_count(rows, *page_size))
///     .page_sizes(page_size, &[25, 50, 100])
///     .show(ui);
/// if let Some(new_page) = response.page {
///     // fetch new_page
/// }
/// # }
/// ```
pub struct Pager<'a> {
    id_salt: Id,
    page: &'a mut usize,
    page_count: usize,
    siblings: usize,
    first_last: bool,
    sizes: Option<(&'a mut usize, Vec<usize>)>,
    size: ButtonSize,
}

impl<'a> Pager<'a> {
    /// A pager. `page` is the current 0-based page and is updated in place; it is clamped into
    /// `0..page_count` every frame.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        page: &'a mut usize,
        page_count: usize,
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            page,
            page_count,
            siblings: 1,
            first_last: true,
            sizes: None,
            size: ButtonSize::Medium,
        }
    }

    /// Pages shown each side of the current one. Default 1.
    #[must_use]
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Show the First and Last buttons. Default on.
    #[must_use]
    pub fn first_last(mut self, on: bool) -> Self {
        self.first_last = on;
        self
    }

    /// Add a page-size Dropdown with these choices. `page_size` is updated in place. A size
    /// that is not in `sizes` shows as unselected.
    #[must_use]
    pub fn page_sizes(mut self, page_size: &'a mut usize, sizes: &[usize]) -> Self {
        self.sizes = Some((page_size, sizes.to_vec()));
        self
    }

    /// Button size. Default medium.
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Draw the pager in a horizontal row.
    pub fn show(self, ui: &mut Ui) -> PagerResponse {
        let Pager {
            id_salt,
            page,
            page_count,
            siblings,
            first_last,
            sizes,
            size,
        } = self;
        *page = (*page).min(page_count.saturating_sub(1));
        let current = *page;
        let last = page_count.saturating_sub(1);
        let mut target: Option<usize> = None;
        let mut new_size = None;

        let inner = ui.horizontal(|ui| {
            let has_prev = page_count > 0 && current > 0;
            let has_next = page_count > 0 && current < last;
            let nav = |text: &str, salt: &str| {
                Button::new(text)
                    .appearance(Appearance::Subtle)
                    .size(size)
                    .min_width(0.0)
                    .id_salt((id_salt, salt))
            };
            if first_last && ui.add_enabled(has_prev, nav("First", "first")).clicked() {
                target = Some(0);
            }
            let prev = Button::icon_only(Icon::ChevronLeft, "Previous page")
                .appearance(Appearance::Subtle)
                .size(size)
                .id_salt((id_salt, "prev"));
            if ui.add_enabled(has_prev, prev).clicked() {
                target = Some(current.saturating_sub(1));
            }
            for (slot, item) in page_window(current, page_count, siblings).into_iter().enumerate() {
                match item {
                    PageItem::Page(n) => {
                        let appearance = if n == current {
                            Appearance::Primary
                        } else {
                            Appearance::Subtle
                        };
                        // Page numbers are square at least, not the 96 pt text-button minimum.
                        let button = Button::new((n + 1).to_string())
                            .appearance(appearance)
                            .size(size)
                            .min_width(Metrics::of(size).min_height)
                            .id_salt((id_salt, "page", slot));
                        let response = ui.add(button);
                        if response.clicked() && n != current {
                            target = Some(n);
                        }
                    }
                    PageItem::Ellipsis => {
                        ui.label("…");
                    }
                }
            }
            let next = Button::icon_only(Icon::ChevronRight, "Next page")
                .appearance(Appearance::Subtle)
                .size(size)
                .id_salt((id_salt, "next"));
            if ui.add_enabled(has_next, next).clicked() {
                target = Some((current + 1).min(last));
            }
            if first_last && ui.add_enabled(has_next, nav("Last", "last")).clicked() {
                target = Some(last);
            }
            if let Some((page_size, choices)) = sizes {
                ui.add_space(8.0);
                let labels: Vec<String> = choices.iter().map(|n| format!("{n} per page")).collect();
                let mut selected = choices.iter().position(|n| n == page_size);
                let before = selected;
                ui.add(
                    Dropdown::new((id_salt, "size"), &mut selected, &labels)
                        .placeholder("Page size")
                        .size(size)
                        .width(132.0),
                );
                if selected != before
                    && let Some(chosen) = selected.and_then(|i| choices.get(i)).copied()
                {
                    let chosen = chosen.max(1);
                    // Keep the first item of the old page in view.
                    let first_item = current.saturating_mul((*page_size).max(1));
                    *page_size = chosen;
                    new_size = Some(chosen);
                    target = Some(first_item / chosen);
                }
            }
        });

        let page_changed = target.filter(|&t| t != current || new_size.is_some());
        if let Some(t) = page_changed {
            *page = t;
        }
        PagerResponse {
            response: inner.response,
            page: page_changed,
            page_size: new_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageItem::{Ellipsis, Page};
    use super::{PageItem, page_count, page_window};

    fn render(items: &[PageItem]) -> String {
        items
            .iter()
            .map(|i| match i {
                Page(n) => (n + 1).to_string(),
                Ellipsis => "…".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn zero_and_one_page() {
        assert!(page_window(0, 0, 1).is_empty());
        assert!(page_window(7, 0, 1).is_empty());
        assert_eq!(page_window(0, 1, 1), vec![Page(0)]);
        assert_eq!(
            page_window(9, 1, 1),
            vec![Page(0)],
            "current past the end is clamped"
        );
    }

    #[test]
    fn small_counts_show_every_page() {
        assert_eq!(render(&page_window(0, 7, 1)), "1 2 3 4 5 6 7");
        assert_eq!(render(&page_window(6, 7, 1)), "1 2 3 4 5 6 7");
    }

    #[test]
    fn window_moves_with_the_current_page() {
        let w = |p| render(&page_window(p, 20, 1));
        assert_eq!(w(0), "1 2 3 4 5 … 20");
        assert_eq!(w(3), "1 2 3 4 5 … 20");
        assert_eq!(w(4), "1 … 4 5 6 … 20");
        assert_eq!(w(10), "1 … 10 11 12 … 20");
        assert_eq!(w(15), "1 … 15 16 17 … 20");
        assert_eq!(w(16), "1 … 16 17 18 19 20");
        assert_eq!(w(19), "1 … 16 17 18 19 20");
    }

    #[test]
    fn slot_count_is_constant_and_current_is_always_shown() {
        for siblings in 0..4 {
            for count in 1..40 {
                let slots = (2 * siblings + 5).min(count);
                for current in 0..count {
                    let items = page_window(current, count, siblings);
                    assert_eq!(
                        items.len(),
                        slots,
                        "count {count} current {current} siblings {siblings}"
                    );
                    assert!(items.contains(&Page(current)));
                    assert_eq!(items.first(), Some(&Page(0)));
                    assert_eq!(items.last(), Some(&Page(count - 1)));
                    // Pages ascend and an ellipsis always stands for at least two pages.
                    let mut prev: Option<usize> = None;
                    let mut gap = false;
                    for item in &items {
                        match item {
                            Page(n) => {
                                if let Some(p) = prev {
                                    assert!(*n > p);
                                    assert_eq!(gap, *n > p + 1, "{}", render(&items));
                                    if gap {
                                        assert!(*n >= p + 3, "ellipsis hides one page: {}", render(&items));
                                    }
                                }
                                prev = Some(*n);
                                gap = false;
                            }
                            Ellipsis => gap = true,
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn huge_counts_do_not_overflow() {
        let items = page_window(usize::MAX - 1, usize::MAX, 2);
        assert!(items.contains(&Page(usize::MAX - 1)));
        assert_eq!(page_window(3, 10, 0).len(), 5);
    }

    #[test]
    fn page_count_rounds_up() {
        assert_eq!(page_count(0, 10), 0);
        assert_eq!(page_count(1, 10), 1);
        assert_eq!(page_count(10, 10), 1);
        assert_eq!(page_count(11, 10), 2);
        assert_eq!(page_count(5, 0), 5, "page size 0 is treated as 1");
    }
}
