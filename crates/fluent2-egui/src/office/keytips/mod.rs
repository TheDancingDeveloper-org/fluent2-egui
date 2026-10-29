//! KeyTips: Alt or F10 shows letter badges over the ribbon, and typing a key runs its target
//! (spec §6). F6 cycles focus between the chrome's regions (spec §9.3, [`regions`]).
//!
//! [`state`] is the pure state machine. This module feeds it egui input and paints the badges.
//! It runs as an egui plugin, installed by [`install`] (the ribbon and the QAT install it
//! themselves):
//!
//! - at the start of each pass it reads this frame's key events, steps the machine against the
//!   targets registered last frame, and consumes the keys it used;
//! - during the pass, ribbon tabs, ribbon controls and QAT items [`register`] their keytip and
//!   rectangle, and ask whether they were activated ([`invoked`], [`selected_tab`],
//!   [`popup_opened`]). An activated control reports exactly what a click would;
//! - at the end of the pass it assigns keys to the registered targets and paints the badges of
//!   the current level in a foreground layer.
//!
//! Alt counts only when it is pressed and released with no key or click in between, so Alt+F
//! stays the app's shortcut (spec §6.1).
//!
//! ```
//! # use fluent2_egui::office::keytips::{self, KeyTipCandidate, KeyTipLevel, TargetKind};
//! # fn demo(ui: &mut egui::Ui) {
//! keytips::install(ui.ctx());
//! let response = ui.button("Find");
//! keytips::register(
//!     ui.ctx(),
//!     KeyTipCandidate {
//!         target: "find",
//!         level: KeyTipLevel::Root,
//!         kind: TargetKind::Control,
//!         requested: Some("Q"),
//!         label: "Find".to_owned(),
//!     },
//!     response.rect,
//! );
//! if response.clicked() || keytips::invoked(ui.ctx(), "find") {
//!     // run Find
//! }
//! # }
//! ```

pub mod regions;
pub mod state;

use egui::text::{LayoutJob, TextFormat};
use egui::{Context, Event, Id, Key, LayerId, Order, Popup, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

#[rustfmt::skip]
pub use regions::{
    Region,
    mark_region,
};
#[rustfmt::skip]
pub use state::{
    KeyTipAction,
    KeyTipCandidate,
    KeyTipInput,
    KeyTipLevel,
    KeyTipResolution,
    KeyTipState,
    KeyTipStep,
    KeyTipTarget,
    KeyTipTargets,
    KeyTipsReport,
    TargetKind,
    resolve_keytips,
    step,
};

use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{Theme, c};

/// One registered target: what the app asked for, where it is, and the popup it opens.
#[derive(Clone, Debug)]
struct Registered {
    candidate: KeyTipCandidate,
    rect: Rect,
    /// The widget the KeyTip runs, which carries the access key for screen readers.
    widget: Option<Id>,
    popup: Option<Id>,
}

/// The controller kept in `ctx.data` between passes.
#[derive(Clone, Debug)]
struct Controller {
    state: KeyTipState,
    /// Alt is held.
    alt_down: bool,
    /// Something else happened while Alt was held, so its release is not a KeyTips press.
    alt_tainted: bool,
    /// Targets resolved at the end of the last pass. Input is stepped against these.
    targets: KeyTipTargets,
    /// Popup ids of last pass's popup controls, by control key.
    popups: Vec<(&'static str, Id)>,
    /// Targets registered during this pass.
    frame: Vec<Registered>,
    /// What this pass's input asked for.
    action: KeyTipAction,
    /// Targets whose badge was painted at the end of the last pass.
    shown: Vec<KeyTipTarget>,
    /// KeyTips have been on at least once, so the live region exists and says on or off.
    announced: bool,
    report: KeyTipsReport,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: KeyTipState::Idle,
            alt_down: false,
            alt_tainted: false,
            targets: KeyTipTargets::default(),
            popups: Vec::new(),
            frame: Vec::new(),
            action: KeyTipAction::None,
            shown: Vec::new(),
            announced: false,
            report: KeyTipsReport::default(),
        }
    }
}

fn controller_id() -> Id {
    Id::unique("fluent2_keytips")
}

fn with_controller<R>(ctx: &Context, f: impl FnOnce(&mut Controller) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<Controller>(controller_id())))
}

/// The plugin that runs KeyTips and F6 region cycling at the start and end of every pass.
struct KeyTipsPlugin;

impl egui::Plugin for KeyTipsPlugin {
    fn debug_name(&self) -> &'static str {
        "fluent2_keytips"
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        regions::begin_pass(ui.ctx());
        begin_pass(ui.ctx());
    }

    fn on_end_pass(&mut self, ui: &mut Ui) {
        end_pass(ui.ctx());
        regions::end_pass(ui.ctx());
    }
}

/// Install KeyTips and F6 region cycling on `ctx`. Calling it again does nothing. The ribbon
/// and the QAT call it, so an app only needs it when it registers targets without them.
///
/// ```
/// # fn demo(ctx: &egui::Context) {
/// fluent2_egui::office::keytips::install(ctx);
/// # }
/// ```
pub fn install(ctx: &Context) {
    ctx.add_plugin(KeyTipsPlugin);
}

/// Register a target for this frame: its keytip candidate and the rectangle its badge sits on.
/// Register only enabled targets, every frame they are visible.
///
/// See the module example.
pub fn register(ctx: &Context, candidate: KeyTipCandidate, rect: Rect) {
    register_with_popup(ctx, candidate, rect, None, None);
}

/// [`register`], naming the widget the KeyTip runs. That widget's AccessKit node gets the
/// target's full key sequence as its access key ("Alt, H, B"), so screen readers can say it
/// (WI-1849). The ribbon and QAT register this way.
pub fn register_for(ctx: &Context, candidate: KeyTipCandidate, rect: Rect, widget: Id) {
    register_with_popup(ctx, candidate, rect, Some(widget), None);
}

/// Register a target that opens the egui popup `popup` (a menu or gallery control). Leaving
/// the popup level with Escape or Backspace closes it.
pub(crate) fn register_with_popup(
    ctx: &Context,
    candidate: KeyTipCandidate,
    rect: Rect,
    widget: Option<Id>,
    popup: Option<Id>,
) {
    with_controller(ctx, |kt| {
        kt.frame.push(Registered {
            candidate,
            rect: sanitize(rect),
            widget,
            popup,
        });
    });
}

/// Whether KeyTips invoked `target` this frame. The target then does what a click does.
///
/// See the module example.
pub fn invoked(ctx: &Context, target: &str) -> bool {
    with_controller(
        ctx,
        |kt| matches!(kt.action, KeyTipAction::Invoke(key) if key == target),
    )
}

/// The tab KeyTips selected this frame, if any. The ribbon makes it the active tab.
pub fn selected_tab(ctx: &Context) -> Option<&'static str> {
    with_controller(ctx, |kt| match kt.action {
        KeyTipAction::SelectTab(key) => Some(key),
        _ => None,
    })
}

/// Whether KeyTips opened the popup of control `target` this frame.
pub fn popup_opened(ctx: &Context, target: &str) -> bool {
    with_controller(
        ctx,
        |kt| matches!(kt.action, KeyTipAction::OpenPopup(key) if key == target),
    )
}

/// The current KeyTips state.
///
/// ```
/// # use fluent2_egui::office::keytips::{self, KeyTipState};
/// # fn demo(ctx: &egui::Context) {
/// let active = keytips::current_state(ctx) != KeyTipState::Idle;
/// # let _ = active;
/// # }
/// ```
pub fn current_state(ctx: &Context) -> KeyTipState {
    with_controller(ctx, |kt| kt.state.clone())
}

/// The targets whose badges were painted last frame, with their keys. Empty while KeyTips are
/// off. For tests and debug overlays.
pub fn shown(ctx: &Context) -> Vec<KeyTipTarget> {
    with_controller(ctx, |kt| kt.shown.clone())
}

/// What key assignment did last frame: conflicts, invalid requests, auto-assigned and
/// unassigned targets (spec §6.2).
pub fn report(ctx: &Context) -> KeyTipsReport {
    with_controller(ctx, |kt| kt.report.clone())
}

/// Read this pass's input, step the machine and consume the keys it used.
fn begin_pass(ctx: &Context) {
    let events = ctx.input(|input| input.events.clone());
    let alt_now = ctx.input(|input| input.modifiers.alt);
    let mut kt = with_controller(ctx, std::mem::take);
    kt.action = KeyTipAction::None;
    kt.frame.clear();
    let before = kt.state.clone();

    let mut consumed = vec![false; events.len()];
    for (index, event) in events.iter().enumerate() {
        let active = kt.state != KeyTipState::Idle;
        match event {
            Event::ModifiersChanged(modifiers) => {
                if modifiers.alt && !kt.alt_down {
                    kt.alt_down = true;
                    kt.alt_tainted = false;
                } else if !modifiers.alt && kt.alt_down {
                    release_alt(ctx, &mut kt);
                }
                if kt.alt_down && (modifiers.ctrl || modifiers.shift || modifiers.command) {
                    kt.alt_tainted = true;
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                // With KeyTips on, a key typed while Alt is still held continues the sequence,
                // as in Office (WI-1927, after gridcraft 4ed93c3b). Alt's release then does not
                // toggle KeyTips off. Ctrl, Shift or Command with it still make it a shortcut.
                let alt_held = modifiers.alt || kt.alt_down;
                let other = modifiers.ctrl || modifiers.shift || modifiers.command;
                if active && alt_held && !other {
                    kt.alt_tainted = true;
                    let input = match key {
                        Key::Escape => Some(KeyTipInput::Escape),
                        Key::Backspace => Some(KeyTipInput::Backspace),
                        _ => key_char(*key).map(KeyTipInput::Char),
                    };
                    if let Some(input) = input {
                        apply(ctx, &mut kt, input);
                        consumed[index] = true;
                    }
                    continue;
                }
                if alt_held {
                    // Alt is a modifier here (Alt+F): its release must not toggle KeyTips.
                    if !kt.alt_down {
                        kt.alt_down = true;
                    }
                    kt.alt_tainted = true;
                    continue;
                }
                if !modifiers.is_none() {
                    continue;
                }
                let input = match key {
                    Key::F10 => Some(KeyTipInput::Activate),
                    Key::Escape if active => Some(KeyTipInput::Escape),
                    Key::Backspace if active => Some(KeyTipInput::Backspace),
                    _ if active => key_char(*key).map(KeyTipInput::Char),
                    _ => None,
                };
                if let Some(input) = input {
                    apply(ctx, &mut kt, input);
                    consumed[index] = true;
                }
            }
            Event::Key {
                key, pressed: false, ..
            } if active && (key_char(*key).is_some() || matches!(key, Key::Escape | Key::Backspace)) => {
                consumed[index] = true;
            }
            Event::Text(_) if active => consumed[index] = true,
            Event::PointerButton { pressed: true, .. } => {
                if kt.alt_down {
                    kt.alt_tainted = true;
                }
                if active {
                    apply(ctx, &mut kt, KeyTipInput::Dismiss);
                }
            }
            Event::WindowFocused(false) => {
                kt.alt_down = false;
                if active {
                    apply(ctx, &mut kt, KeyTipInput::Dismiss);
                }
            }
            _ => {}
        }
    }
    // A backend that reports modifiers only in `RawInput::modifiers` still releases Alt here.
    if kt.alt_down && !alt_now {
        release_alt(ctx, &mut kt);
    }

    if consumed.iter().any(|used| *used) {
        ctx.input_mut(|input| {
            let mut index = 0;
            input.events.retain(|_| {
                let keep = !consumed.get(index).copied().unwrap_or(false);
                index += 1;
                keep
            });
        });
    }
    if kt.state != before {
        // The new level's badges are painted at the end of the next pass, once its targets
        // have registered.
        ctx.request_repaint();
    }
    with_controller(ctx, |slot| *slot = kt);
}

/// Alt went up. With nothing in between, it is a KeyTips press.
fn release_alt(ctx: &Context, kt: &mut Controller) {
    kt.alt_down = false;
    if !kt.alt_tainted {
        apply(ctx, kt, KeyTipInput::Activate);
    }
    kt.alt_tainted = false;
}

/// Step the machine with `input` and record its action.
fn apply(ctx: &Context, kt: &mut Controller, input: KeyTipInput) {
    let leaving_popup = match &kt.state {
        KeyTipState::Popup { control, .. } => Some(*control),
        _ => None,
    };
    let next = step(&kt.state, input.clone(), &kt.targets);
    // Escape or Backspace from a popup level closes the popup it opened.
    if let Some(control) = leaving_popup
        && matches!(input, KeyTipInput::Escape | KeyTipInput::Backspace)
        && let Some((_, popup)) = kt.popups.iter().find(|(key, _)| *key == control)
    {
        Popup::close_id(ctx, *popup);
    }
    kt.state = next.state;
    match next.action {
        KeyTipAction::None | KeyTipAction::Show => {}
        action => kt.action = action,
    }
}

/// The character a key types at a KeyTips level: an ASCII letter or digit.
fn key_char(key: Key) -> Option<char> {
    let mut chars = key.name().chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_uppercase()),
        _ => None,
    }
}

/// Assign keys to this pass's targets and paint the badges of the current level.
fn end_pass(ctx: &Context) {
    let mut kt = with_controller(ctx, std::mem::take);
    let frame = std::mem::take(&mut kt.frame);
    let candidates: Vec<KeyTipCandidate> = frame.iter().map(|reg| reg.candidate.clone()).collect();
    let resolution = resolve_keytips(&candidates);
    kt.targets = resolution.targets;
    kt.report = resolution.report;
    kt.popups = frame
        .iter()
        .filter_map(|reg| reg.popup.map(|popup| (reg.candidate.target, popup)))
        .collect();
    kt.action = KeyTipAction::None;

    kt.shown = match visible_level(&kt.state) {
        Some((level, prefix)) => kt
            .targets
            .iter()
            .filter(|target| {
                target.level == level && prefix.is_none_or(|first| target.key.starts_with(first))
            })
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    expose_access_keys(ctx, &kt.targets, &frame);
    announce(ctx, &mut kt);
    if !kt.shown.is_empty() {
        let prefix = match &kt.state {
            KeyTipState::Prefix { first, .. } => Some(*first),
            _ => None,
        };
        paint_badges(ctx, &kt.shown, &frame, prefix);
    }
    with_controller(ctx, |slot| *slot = kt);
}

/// Set each registered widget's AccessKit access key to its target's full key sequence (WI-1849).
fn expose_access_keys(ctx: &Context, targets: &KeyTipTargets, frame: &[Registered]) {
    for reg in frame {
        let Some(widget) = reg.widget else {
            continue;
        };
        let Some(keys) = key_sequence(targets, reg.candidate.level, reg.candidate.target) else {
            continue;
        };
        ctx.accesskit_node_builder(widget, |node| {
            node.set_access_key(format!("Alt, {}", keys.join(", ")));
        });
    }
}

/// The keys typed from Root to reach `target` at `level`: the tab's key, then the popup
/// control's, then the target's own. `None` when a key on the path is unassigned.
pub(crate) fn key_sequence(targets: &KeyTipTargets, level: KeyTipLevel, target: &str) -> Option<Vec<String>> {
    let own = targets
        .iter()
        .find(|t| t.level == level && t.target == target)?
        .key
        .clone();
    let mut keys = match level {
        KeyTipLevel::Root => Vec::new(),
        KeyTipLevel::Tab(tab) => key_sequence(targets, KeyTipLevel::Root, tab)?,
        KeyTipLevel::Popup(control) => {
            let parent = targets.iter().find(|t| t.target == control)?;
            key_sequence(targets, parent.level, control)?
        }
    };
    keys.push(own);
    Some(keys)
}

/// The id of the polite live region that says when KeyTips turn on and off.
pub fn live_region_id() -> Id {
    Id::unique("fluent2_keytips_live")
}

/// Keep a polite live region saying whether KeyTips are on (WI-1849). It appears the first time
/// KeyTips turn on, so an app that never uses them has no extra node.
fn announce(ctx: &Context, kt: &mut Controller) {
    let on = kt.state != KeyTipState::Idle;
    kt.announced |= on;
    if !kt.announced {
        return;
    }
    let text = if on {
        "KeyTips on. Type a key, or press Escape to leave."
    } else {
        "KeyTips off."
    };
    ctx.accesskit_node_builder(live_region_id(), |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_live(egui::accesskit::Live::Polite);
        node.set_label(text);
    });
}

/// The level whose badges a state shows, and the typed first letter for a prefix state.
fn visible_level(state: &KeyTipState) -> Option<(KeyTipLevel, Option<char>)> {
    match state {
        KeyTipState::Idle => None,
        KeyTipState::Root => Some((KeyTipLevel::Root, None)),
        KeyTipState::Tab { tab } => Some((KeyTipLevel::Tab(tab), None)),
        KeyTipState::Popup { control, .. } => Some((KeyTipLevel::Popup(control), None)),
        KeyTipState::Prefix { parent, first } => {
            visible_level(parent).map(|(level, _)| (level, Some(*first)))
        }
    }
}

/// Paint one badge per shown target (spec §6.1, §10). The typed prefix is in the brand colour.
fn paint_badges(ctx: &Context, shown: &[KeyTipTarget], frame: &[Registered], prefix: Option<char>) {
    let theme = Theme::get(ctx);
    let color = &theme.color;
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::unique("fluent2_keytips_badges")));
    let font = font_id(TypeRamp::Caption2);
    let side = theme.chrome.keytip_badge;
    let offset = spacing_horizontal::XXS;
    for target in shown {
        let Some(reg) = frame
            .iter()
            .find(|reg| reg.candidate.target == target.target && reg.candidate.level == target.level)
        else {
            continue;
        };
        let mut job = LayoutJob::default();
        let split = usize::from(prefix.is_some_and(|first| target.key.starts_with(first)));
        let (typed, rest) = target.key.split_at(split.min(target.key.len()));
        for (text, ink) in [
            (typed, color.brand_foreground_1),
            (rest, color.neutral_foreground_1),
        ] {
            if !text.is_empty() {
                job.append(text, 0.0, TextFormat::simple(font.clone(), c(ink)));
            }
        }
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
        let size = Vec2::new(
            side.max(galley.size().x + 2.0 * spacing_horizontal::XXS),
            side.max(galley.size().y),
        );
        // Tab badges sit under the tab label; every other badge at the target's top-left.
        let min = if reg.candidate.kind == TargetKind::Tab {
            Pos2::new(
                reg.rect.center().x - size.x / 2.0,
                reg.rect.bottom() - size.y / 2.0,
            )
        } else {
            reg.rect.min + Vec2::splat(offset)
        };
        let badge = Rect::from_min_size(min, size);
        let corner = egui::CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8);
        painter.rect(
            badge,
            corner,
            c(color.neutral_background_1),
            Stroke::new(stroke_width::THIN, c(color.neutral_stroke_1)),
            StrokeKind::Inside,
        );
        painter.galley(
            badge.center() - galley.size() / 2.0,
            galley,
            c(color.neutral_foreground_1),
        );
    }
}

/// A rectangle with non-finite coordinates becomes an empty one at the origin (rule 9).
fn sanitize(rect: Rect) -> Rect {
    if rect.min.is_finite() && rect.max.is_finite() {
        rect
    } else {
        Rect::ZERO
    }
}
