//! F6 region cycling (spec §9.3).
//!
//! Each region marks itself every frame with its rectangle and its first focusable widget
//! ([`mark_region`]). The title bar, QAT, ribbon and status bar do this themselves; the app marks
//! [`Region::Content`]. F6 moves focus to the next marked region, Shift+F6 to the previous. Focus
//! lands on the widget last focused in that region, or on its first focusable widget.
//!
//! ```
//! # use fluent2_egui::office::keytips::{self, Region};
//! # fn demo(ui: &mut egui::Ui, text: &mut String) {
//! let editor = ui.text_edit_multiline(text);
//! keytips::mark_region(ui.ctx(), Region::Content, editor.rect, Some(editor.id));
//! # }
//! ```

use egui::{Context, Id, Key, Modifiers, Rect};

/// A focus region of the window, in F6 order (spec §9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Region {
    /// The title bar, with the QAT when the QAT is placed in it.
    TitleBar,
    /// The QAT, when it is placed above or below the ribbon.
    Qat,
    /// The File tab, the tab strip and the active tab's controls.
    Ribbon,
    /// The app's document or main view.
    Content,
    /// The status bar.
    StatusBar,
}

const ORDER: [Region; 5] = [
    Region::TitleBar,
    Region::Qat,
    Region::Ribbon,
    Region::Content,
    Region::StatusBar,
];

/// One region as marked this frame.
#[derive(Clone, Copy, Debug)]
struct Mark {
    region: Region,
    rect: Rect,
    first: Option<Id>,
}

#[derive(Clone, Debug, Default)]
struct Regions {
    /// Regions marked during this pass.
    frame: Vec<Mark>,
    /// Regions marked during the last pass. F6 moves between these.
    marks: Vec<Mark>,
    /// The widget last focused in each region.
    last: Vec<(Region, Id)>,
}

fn regions_id() -> Id {
    Id::unique("fluent2_regions")
}

fn with_regions<R>(ctx: &Context, f: impl FnOnce(&mut Regions) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<Regions>(regions_id())))
}

/// Mark `region` for this frame. `rect` is the region's area. `first` is its first focusable
/// widget; a region without one is skipped by F6. Marking a region twice in a frame merges the
/// rectangles and keeps the first `first` given.
///
/// See the module example.
pub fn mark_region(ctx: &Context, region: Region, rect: Rect, first: Option<Id>) {
    if !(rect.min.is_finite() && rect.max.is_finite()) {
        return;
    }
    with_regions(ctx, |regions| {
        if let Some(mark) = regions.frame.iter_mut().find(|mark| mark.region == region) {
            mark.rect = mark.rect.union(rect);
            mark.first = mark.first.or(first);
        } else {
            regions.frame.push(Mark { region, rect, first });
        }
    });
}

/// The region whose area holds the focused widget. The smallest area wins, so a QAT inside the
/// title bar counts as the QAT.
fn region_of(ctx: &Context, marks: &[Mark], focused: Id) -> Option<Region> {
    let rect = ctx.read_response(focused).map(|response| response.rect)?;
    let center = rect.center();
    marks
        .iter()
        .filter(|mark| mark.rect.contains(center) || mark.first == Some(focused))
        .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
        .map(|mark| mark.region)
}

/// Handle F6 and Shift+F6 against last pass's regions.
pub(super) fn begin_pass(ctx: &Context) {
    // Shift+F6 first: `consume_key` matches logically, so a bare F6 pattern also takes Shift+F6.
    let backward = ctx.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::F6));
    let forward = !backward && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F6));
    if !forward && !backward {
        return;
    }
    let (marks, last) = with_regions(ctx, |regions| (regions.marks.clone(), regions.last.clone()));
    let usable: Vec<&Mark> = ORDER
        .iter()
        .filter_map(|region| {
            marks
                .iter()
                .find(|mark| mark.region == *region && mark.first.is_some())
        })
        .collect();
    if usable.is_empty() {
        return;
    }
    let current = ctx
        .memory(|memory| memory.focused())
        .and_then(|focused| region_of(ctx, &marks, focused));
    let position = current.and_then(|region| usable.iter().position(|mark| mark.region == region));
    let count = usable.len();
    let next = match (position, forward) {
        (Some(index), true) => (index + 1) % count,
        (Some(index), false) => (index + count - 1) % count,
        (None, true) => 0,
        (None, false) => count - 1,
    };
    let Some(mark) = usable.get(next) else {
        return;
    };
    let remembered = last
        .iter()
        .find(|(region, _)| *region == mark.region)
        .map(|(_, id)| *id)
        .filter(|id| ctx.read_response(*id).is_some());
    if let Some(target) = remembered.or(mark.first) {
        ctx.memory_mut(|memory| memory.request_focus(target));
        ctx.request_repaint();
    }
}

/// Keep this pass's marks and remember which region holds the focus.
pub(super) fn end_pass(ctx: &Context) {
    let focused = ctx.memory(|memory| memory.focused());
    let marks = with_regions(ctx, |regions| {
        regions.marks = std::mem::take(&mut regions.frame);
        regions.marks.clone()
    });
    if let Some(focused) = focused
        && let Some(region) = region_of(ctx, &marks, focused)
    {
        with_regions(ctx, |regions| {
            regions.last.retain(|(r, _)| *r != region);
            regions.last.push((region, focused));
        });
    }
}
