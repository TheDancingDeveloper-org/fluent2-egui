//! Ribbon adaptive scaling (spec §5): scale steps, the pure layout planner, and the width memo.
//!
//! [`plan`] has no egui `Ui`. It takes each group's width at every step it allows and picks
//! the step per group for the available width, following the reduction order (§5.2) and the
//! hysteresis band (§5.3).

use std::hash::Hash;

use egui::{Context, Id};

/// The scale step a group renders at (spec §5.1), widest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScalingStep {
    /// Large controls, with labels under icons where the control has them.
    Large,
    /// Small controls, with labels beside icons.
    Medium,
    /// Icon only; labels move into tooltips.
    Small,
    /// One collapsed button that opens the group in a popup.
    Collapsed,
}

impl ScalingStep {
    const ALL: [Self; 4] = [Self::Large, Self::Medium, Self::Small, Self::Collapsed];

    fn index(self) -> usize {
        match self {
            Self::Large => 0,
            Self::Medium => 1,
            Self::Small => 2,
            Self::Collapsed => 3,
        }
    }
}

/// The steps a group may use, widest first. Always ends with [`ScalingStep::Collapsed`].
///
/// ```
/// # use fluent2_egui::office::ribbon::{ScalingStep, ScalingSteps};
/// let steps = ScalingSteps::new(&[ScalingStep::Small, ScalingStep::Large, ScalingStep::Small]);
/// assert_eq!(steps.as_slice(), &[ScalingStep::Large, ScalingStep::Small, ScalingStep::Collapsed]);
/// assert_eq!(ScalingSteps::default(), ScalingSteps::all());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScalingSteps {
    steps: Vec<ScalingStep>,
}

impl ScalingSteps {
    /// All four steps. The default.
    pub fn all() -> Self {
        Self {
            steps: ScalingStep::ALL.to_vec(),
        }
    }

    /// The given steps, sorted widest first, duplicates removed. `Collapsed` is always added.
    pub fn new(steps: &[ScalingStep]) -> Self {
        let mut steps = steps.to_vec();
        steps.push(ScalingStep::Collapsed);
        steps.sort_unstable();
        steps.dedup();
        Self { steps }
    }

    /// The steps, widest first.
    pub fn as_slice(&self) -> &[ScalingStep] {
        &self.steps
    }

    /// Whether `step` is allowed.
    pub fn contains(&self, step: ScalingStep) -> bool {
        self.steps.contains(&step)
    }
}

impl Default for ScalingSteps {
    fn default() -> Self {
        Self::all()
    }
}

/// One group's width, in points, at each step it allows (spec §5.4).
///
/// ```
/// # use fluent2_egui::office::ribbon::{GroupMeasure, ScalingStep, ScalingSteps};
/// let group = GroupMeasure::new(&ScalingSteps::all(), [200.0, 160.0, 120.0, 36.0]);
/// assert_eq!(group.width(ScalingStep::Medium), Some(160.0));
/// let no_medium = ScalingSteps::new(&[ScalingStep::Large, ScalingStep::Small]);
/// let group = GroupMeasure::new(&no_medium, [200.0, 160.0, 120.0, 36.0]);
/// assert_eq!(group.width(ScalingStep::Medium), None);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct GroupMeasure {
    widths: [Option<f32>; 4],
}

impl GroupMeasure {
    /// `widths` holds the width at Large, Medium, Small and Collapsed. Steps not in `steps`
    /// are dropped. Non-finite or negative widths are treated as 0.
    pub fn new(steps: &ScalingSteps, widths: [f32; 4]) -> Self {
        let mut out = [None; 4];
        for (step, width) in ScalingStep::ALL.iter().zip(widths) {
            if let Some(slot) = out.get_mut(step.index())
                && steps.contains(*step)
            {
                *slot = Some(clean(width));
            }
        }
        Self { widths: out }
    }

    /// The width at `step`, or `None` when the group does not allow it.
    pub fn width(&self, step: ScalingStep) -> Option<f32> {
        self.widths.get(step.index()).copied().flatten()
    }

    /// The allowed steps, widest first.
    fn allowed(&self) -> impl Iterator<Item = ScalingStep> + '_ {
        ScalingStep::ALL
            .into_iter()
            .filter(|step| self.width(*step).is_some())
    }

    fn widest(&self) -> ScalingStep {
        self.allowed().next().unwrap_or(ScalingStep::Collapsed)
    }

    /// The next allowed step narrower than `step`.
    fn narrower(&self, step: ScalingStep) -> Option<ScalingStep> {
        self.allowed().find(|next| *next > step)
    }

    fn width_or_zero(&self, step: ScalingStep) -> f32 {
        self.width(step).unwrap_or(0.0)
    }
}

/// Fixed sizes the planner uses, in points (spec §5.2, §5.3, §10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutMetrics {
    /// Gap between groups (`ChromeMetrics::group_separator`).
    pub separator: f32,
    /// Spare room needed before a group steps up (`ChromeMetrics::hysteresis`).
    pub hysteresis: f32,
    /// Width of the trailing `…` overflow button (`ChromeMetrics::collapsed_button`).
    pub overflow_button: f32,
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            separator: 8.0,
            hysteresis: 8.0,
            overflow_button: 36.0,
        }
    }
}

/// The step chosen for each group, and how many trailing groups moved to the `…` menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    steps: Vec<ScalingStep>,
    overflow: usize,
    width: f32,
    /// Position in the reduction sequence; the hysteresis anchor for the next frame.
    level: usize,
}

impl Plan {
    /// The step of each group, in visual order. Overflowed groups read `Collapsed`.
    pub fn steps(&self) -> &[ScalingStep] {
        &self.steps
    }

    /// The step of group `index`, or `None` past the end.
    pub fn step(&self, index: usize) -> Option<ScalingStep> {
        self.steps.get(index).copied()
    }

    /// How many groups, counted from the right, sit in the trailing `…` menu (spec §5.2).
    pub fn overflow(&self) -> usize {
        self.overflow
    }

    /// How many groups are drawn in the body, from the left.
    pub fn visible(&self) -> usize {
        self.steps.len() - self.overflow
    }

    /// Total width of the planned row, separators and overflow button included.
    pub fn width(&self) -> f32 {
        self.width
    }
}

/// Pick a step for each group so the row fits in `available` (spec §5.2, §5.3).
///
/// The reductions form one fixed sequence: from all groups at their widest, the rightmost
/// group not yet `Collapsed` steps to its next allowed step; once all are collapsed, the
/// rightmost groups move one at a time into the `…` menu. Growth walks the sequence back.
///
/// - Without `previous`, the plan is the first point in the sequence that fits.
/// - With `previous`, the plan steps down from it as soon as the row exceeds `available`,
///   and steps up only while the wider row plus `metrics.hysteresis` still fits.
///
/// A `previous` plan for a different number of groups is ignored. Non-finite or negative
/// widths and `available` are treated as 0.
///
/// ```
/// # use fluent2_egui::office::ribbon::{plan, GroupMeasure, LayoutMetrics, ScalingStep, ScalingSteps};
/// let all = ScalingSteps::all();
/// let groups = [
///     GroupMeasure::new(&all, [200.0, 160.0, 120.0, 36.0]),
///     GroupMeasure::new(&all, [320.0, 260.0, 230.0, 36.0]),
///     GroupMeasure::new(&all, [280.0, 240.0, 200.0, 36.0]),
///     GroupMeasure::new(&all, [360.0, 300.0, 260.0, 36.0]),
/// ];
/// let p = plan(&groups, 900.0, None, LayoutMetrics::default());
/// use ScalingStep::*;
/// assert_eq!(p.steps(), &[Large, Large, Large, Collapsed]);
/// assert_eq!(p.width(), 860.0);
/// ```
pub fn plan(
    groups: &[GroupMeasure],
    available: f32,
    previous: Option<&Plan>,
    metrics: LayoutMetrics,
) -> Plan {
    let available = clean(available);
    let hysteresis = clean(metrics.hysteresis);
    let sequence = Sequence::new(groups, metrics);
    let last = sequence.len().saturating_sub(1);
    let fits = |level: usize| sequence.width(level) <= available;

    let level = match previous.filter(|previous| previous.steps.len() == groups.len()) {
        None => (0..=last).find(|&level| fits(level)).unwrap_or(last),
        Some(previous) => {
            let start = previous.level.min(last);
            if fits(start) {
                let mut level = start;
                while level > 0 && sequence.width(level - 1) + hysteresis <= available {
                    level -= 1;
                }
                level
            } else {
                (start..=last).find(|&level| fits(level)).unwrap_or(last)
            }
        }
    };
    sequence.plan(level)
}

/// The reduction sequence (spec §5.2) over one set of groups.
struct Sequence<'g> {
    groups: &'g [GroupMeasure],
    metrics: LayoutMetrics,
    /// Steps at each level before overflow begins.
    states: Vec<Vec<ScalingStep>>,
}

impl<'g> Sequence<'g> {
    fn new(groups: &'g [GroupMeasure], metrics: LayoutMetrics) -> Self {
        let mut current: Vec<ScalingStep> = groups.iter().map(GroupMeasure::widest).collect();
        let mut states = vec![current.clone()];
        for index in (0..groups.len()).rev() {
            let Some(group) = groups.get(index) else {
                continue;
            };
            while let Some(step) = current.get(index).copied()
                && let Some(next) = group.narrower(step)
            {
                if let Some(slot) = current.get_mut(index) {
                    *slot = next;
                }
                states.push(current.clone());
            }
        }
        Self {
            groups,
            metrics,
            states,
        }
    }

    /// Levels: every state, then one more per overflowed group.
    fn len(&self) -> usize {
        self.states.len() + self.groups.len()
    }

    /// Steps and overflow count at `level`.
    fn at(&self, level: usize) -> (&[ScalingStep], usize) {
        let collapsed_index = self.states.len().saturating_sub(1);
        let state = self
            .states
            .get(level.min(collapsed_index))
            .map_or(&[][..], Vec::as_slice);
        let overflow = level.saturating_sub(collapsed_index).min(self.groups.len());
        (state, overflow)
    }

    fn width(&self, level: usize) -> f32 {
        let (steps, overflow) = self.at(level);
        let separator = clean(self.metrics.separator);
        let visible = self.groups.len() - overflow;
        let mut items = visible;
        let mut width: f32 = self
            .groups
            .iter()
            .zip(steps)
            .take(visible)
            .map(|(group, step)| group.width_or_zero(*step))
            .sum();
        if overflow > 0 {
            width += clean(self.metrics.overflow_button);
            items += 1;
        }
        width + separator * items.saturating_sub(1) as f32
    }

    fn plan(&self, level: usize) -> Plan {
        let (steps, overflow) = self.at(level);
        Plan {
            steps: steps.to_vec(),
            overflow,
            width: self.width(level),
            level,
        }
    }
}

/// Clamp a width or bound to a finite, non-negative value.
fn clean(value: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

/// A group's step widths, memoised in `ctx.data` per (ribbon id, tab key, group key) and
/// recomputed when `content` (a hash of keys, labels and allowed steps) changes (spec §5.4).
/// The memo is an optimisation: `measure` gives the same result without it.
pub(crate) fn cached_measure(
    ctx: &Context,
    ribbon: Id,
    tab: &'static str,
    group: &'static str,
    content: impl Hash + std::fmt::Debug,
    measure: impl FnOnce() -> GroupMeasure,
) -> GroupMeasure {
    let key = ribbon.with(("measure", tab, group));
    let hash = Id::new(content);
    if let Some((stored, widths)) = ctx.data(|data| data.get_temp::<(Id, GroupMeasure)>(key))
        && stored == hash
    {
        return widths;
    }
    let widths = measure();
    ctx.data_mut(|data| data.insert_temp(key, (hash, widths.clone())));
    widths
}

/// Plan the active tab with last frame's plan for the same tab as the hysteresis anchor, and
/// store the result for the next frame. A tab switch starts fresh.
pub(crate) fn plan_tab(
    ctx: &Context,
    ribbon: Id,
    tab: &'static str,
    groups: &[GroupMeasure],
    available: f32,
    metrics: LayoutMetrics,
) -> Plan {
    let key = ribbon.with(("plan", tab));
    let previous = ctx.data(|data| data.get_temp::<Plan>(key));
    let next = plan(groups, available, previous.as_ref(), metrics);
    ctx.data_mut(|data| data.insert_temp(key, next.clone()));
    next
}

#[cfg(test)]
mod tests {
    use super::ScalingStep::{Collapsed as C, Large as L, Medium as M, Small as S};
    use super::*;

    fn all(widths: [f32; 4]) -> GroupMeasure {
        GroupMeasure::new(&ScalingSteps::all(), widths)
    }

    /// The four groups of the spec's worked example (§5.5).
    fn example() -> Vec<GroupMeasure> {
        vec![
            all([200.0, 160.0, 120.0, 36.0]),
            all([320.0, 260.0, 230.0, 36.0]),
            all([280.0, 240.0, 200.0, 36.0]),
            all([360.0, 300.0, 260.0, 36.0]),
        ]
    }

    fn fresh(groups: &[GroupMeasure], available: f32) -> Plan {
        plan(groups, available, None, LayoutMetrics::default())
    }

    fn from(groups: &[GroupMeasure], available: f32, previous: &Plan) -> Plan {
        plan(groups, available, Some(previous), LayoutMetrics::default())
    }

    #[test]
    fn worked_example_reduces_styles_to_collapsed() {
        let p = fresh(&example(), 900.0);
        assert_eq!(p.steps(), &[L, L, L, C]);
        assert_eq!(p.width(), 860.0);
        assert_eq!(p.overflow(), 0);
    }

    #[test]
    fn worked_example_growth_follows_the_hysteresis_band() {
        let groups = example();
        let at_900 = fresh(&groups, 900.0);
        assert_eq!(from(&groups, 1000.0, &at_900).steps(), &[L, L, L, C]);
        assert_eq!(from(&groups, 1090.0, &at_900).steps(), &[L, L, L, C]);
        let at_1100 = from(&groups, 1100.0, &at_900);
        assert_eq!(at_1100.steps(), &[L, L, L, S]);
        assert_eq!(at_1100.width(), 1084.0);
    }

    #[test]
    fn everything_fits_at_large() {
        let p = fresh(&example(), 1184.0);
        assert_eq!(p.steps(), &[L, L, L, L]);
    }

    #[test]
    fn reduction_finishes_the_rightmost_group_before_the_next() {
        // 860 is all-but-Styles at Large. 859 must touch Paragraph next.
        let p = fresh(&example(), 859.0);
        assert_eq!(p.steps(), &[L, L, M, C]);
        assert_eq!(p.width(), 820.0);
    }

    #[test]
    fn steps_down_at_once_when_the_row_no_longer_fits() {
        let groups = example();
        let at_900 = fresh(&groups, 900.0);
        let p = from(&groups, 859.0, &at_900);
        assert_eq!(p.steps(), &[L, L, M, C]);
    }

    #[test]
    fn more_width_never_narrows_any_group() {
        let groups = example();
        let mut previous = fresh(&groups, 0.0);
        let mut w = 0.0;
        while w <= 1300.0 {
            let p = fresh(&groups, w);
            for (before, after) in previous.steps().iter().zip(p.steps()) {
                assert!(after <= before, "at {w}: {before:?} -> {after:?}");
            }
            assert!(p.overflow() <= previous.overflow(), "at {w}");
            previous = p;
            w += 1.0;
        }
    }

    #[test]
    fn widths_inside_the_band_do_not_flip_the_plan() {
        let groups = example();
        let mut p = fresh(&groups, 900.0);
        let anchor = p.clone();
        for w in [902.0, 900.0, 898.0, 900.0, 902.0] {
            p = from(&groups, w, &p);
            assert_eq!(p, anchor, "at {w}");
        }
        // At the 1084 threshold: crossing it by 2 pt either way changes nothing.
        let mut p = from(&groups, 1094.0, &anchor);
        assert_eq!(p.steps(), &[L, L, L, S]);
        let s_plan = p.clone();
        for w in [1086.0, 1088.0, 1084.0, 1090.0, 1086.0] {
            p = from(&groups, w, &p);
            assert_eq!(p, s_plan, "at {w}");
        }
    }

    #[test]
    fn everything_collapses_and_then_overflows() {
        let groups = example();
        // 4 × 36 + 3 × 8 = 168.
        let p = fresh(&groups, 168.0);
        assert_eq!(p.steps(), &[C, C, C, C]);
        assert_eq!(p.overflow(), 0);
        // One group moves to the menu: 3 × 36 + 36 + 3 × 8 still 168, so two must go.
        let p = fresh(&groups, 167.0);
        assert_eq!(p.overflow(), 2);
        assert_eq!(p.visible(), 2);
        assert_eq!(p.width(), 3.0 * 36.0 + 2.0 * 8.0);
        let p = fresh(&groups, 0.0);
        assert_eq!(p.overflow(), 4);
        assert_eq!(p.visible(), 0);
    }

    #[test]
    fn nan_and_negative_widths_count_as_zero() {
        let groups = vec![
            all([f32::NAN, -5.0, f32::INFINITY, -1.0]),
            all([100.0, 80.0, 60.0, 36.0]),
        ];
        assert_eq!(groups[0].width(L), Some(0.0));
        let p = fresh(&groups, 108.0);
        assert_eq!(p.steps(), &[L, L]);
        assert_eq!(p.width(), 108.0);
        let p = fresh(&groups, f32::NAN);
        assert_eq!(p.overflow(), 2);
        assert_eq!(p.steps(), &[C, C]);
    }

    #[test]
    fn a_group_without_medium_goes_large_to_small() {
        let no_medium = ScalingSteps::new(&[L, S]);
        let groups = vec![GroupMeasure::new(&no_medium, [200.0, 160.0, 120.0, 36.0])];
        assert_eq!(fresh(&groups, 199.0).steps(), &[S]);
        assert_eq!(fresh(&groups, 119.0).steps(), &[C]);
    }

    #[test]
    fn a_collapsed_only_group_starts_collapsed() {
        let groups = vec![
            GroupMeasure::new(&ScalingSteps::new(&[]), [200.0, 160.0, 120.0, 36.0]),
            all([100.0, 80.0, 60.0, 36.0]),
        ];
        assert_eq!(fresh(&groups, 1000.0).steps(), &[C, L]);
    }

    #[test]
    fn a_previous_plan_for_other_groups_is_ignored() {
        let groups = example();
        let other = fresh(&groups[..2], 0.0);
        assert_eq!(from(&groups, 1184.0, &other).steps(), &[L, L, L, L]);
    }

    #[test]
    fn overflowed_groups_return_only_with_spare_room() {
        let groups = example();
        let p = fresh(&groups, 160.0);
        assert_eq!(p.overflow(), 2);
        // All four collapsed need 168; 170 is inside the band.
        assert_eq!(from(&groups, 170.0, &p).overflow(), 2);
        assert_eq!(from(&groups, 176.0, &p).overflow(), 0);
    }

    #[test]
    fn no_groups_gives_an_empty_plan() {
        let p = fresh(&[], 500.0);
        assert!(p.steps().is_empty());
        assert_eq!(p.overflow(), 0);
        assert_eq!(p.width(), 0.0);
    }

    #[test]
    fn the_memo_recomputes_on_a_content_change() {
        let ctx = Context::default();
        let id = Id::new("ribbon");
        let a = cached_measure(&ctx, id, "home", "font", 1, || all([1.0, 1.0, 1.0, 1.0]));
        let b = cached_measure(&ctx, id, "home", "font", 1, || all([2.0, 2.0, 2.0, 2.0]));
        assert_eq!(a, b);
        let c = cached_measure(&ctx, id, "home", "font", 2, || all([2.0, 2.0, 2.0, 2.0]));
        assert_eq!(c.width(L), Some(2.0));
    }

    #[test]
    fn plan_tab_keeps_the_anchor_per_tab() {
        let ctx = Context::default();
        let id = Id::new("ribbon");
        let groups = example();
        let m = LayoutMetrics::default();
        plan_tab(&ctx, id, "home", &groups, 900.0, m);
        assert_eq!(
            plan_tab(&ctx, id, "home", &groups, 1090.0, m).steps(),
            &[L, L, L, C]
        );
        // Another tab has no anchor, so it plans fresh.
        assert_eq!(
            plan_tab(&ctx, id, "insert", &groups, 1090.0, m).steps(),
            &[L, L, L, S]
        );
    }
}
