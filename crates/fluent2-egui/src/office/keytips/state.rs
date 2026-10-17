//! KeyTips state machine (spec §6). Pure logic with no egui types: [`resolve_keytips`] gives
//! each target its key for one frame, and [`step`] maps one input to the next state and the
//! action the chrome takes.

use std::collections::{HashMap, HashSet};

/// Where a target is shown: the root ribbon, the controls of one tab, or the items of one
/// open menu or gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyTipLevel {
    /// Tabs, the File tab, QAT items and app-level targets.
    Root,
    /// The controls of one tab, by tab key.
    Tab(&'static str),
    /// The items of one open menu or gallery, by control key.
    Popup(&'static str),
}

/// What kind of target a key selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// A ribbon tab. Selecting it shows its controls.
    Tab,
    /// A Quick Access Toolbar item or other app-level command.
    QatItem,
    /// An ordinary control. Selecting it invokes it.
    Control,
    /// A control that opens a menu or gallery popup.
    PopupControl,
    /// An item inside an open popup.
    MenuItem,
}

/// One target as the chrome sees it this frame. `requested` is the app's `.keytip(…)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyTipCandidate {
    /// Stable identifier of the target, returned in [`KeyTipAction`].
    pub target: &'static str,
    /// Level the target is shown at.
    pub level: KeyTipLevel,
    /// Kind of target.
    pub kind: TargetKind,
    /// The key the app asked for, if any.
    pub requested: Option<&'static str>,
    /// Visible label. Auto-assignment prefers its letters.
    pub label: String,
}

/// One target with its final key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyTipTarget {
    /// Stable identifier of the target.
    pub target: &'static str,
    /// Level the target is shown at.
    pub level: KeyTipLevel,
    /// Kind of target.
    pub kind: TargetKind,
    /// The key the user types, uppercased.
    pub key: String,
}

/// Every target with its final key, for one frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyTipTargets {
    targets: Vec<KeyTipTarget>,
}

impl KeyTipTargets {
    /// Every target, in candidate order.
    pub fn iter(&self) -> impl Iterator<Item = &KeyTipTarget> {
        self.targets.iter()
    }

    /// The target at `level` with `key`, if any. Keys compare without case.
    pub fn find(&self, level: KeyTipLevel, key: &str) -> Option<&KeyTipTarget> {
        self.targets
            .iter()
            .find(|target| target.level == level && target.key.eq_ignore_ascii_case(key))
    }
}

/// What assignment did, for the app's tests and debug overlay.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyTipsReport {
    /// Targets whose requested key was already taken at their level, or clashed with a
    /// one-letter key that starts their two-letter key.
    pub conflicts: Vec<&'static str>,
    /// Requested keys that were not one or two letters or one digit.
    pub invalid: Vec<&'static str>,
    /// Targets that had no requested key.
    pub auto_assigned: Vec<&'static str>,
    /// Targets left without a key because their level has no free key left.
    pub unassigned: Vec<&'static str>,
}

/// The result of [`resolve_keytips`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyTipResolution {
    /// Every target that received a key.
    pub targets: KeyTipTargets,
    /// What assignment did.
    pub report: KeyTipsReport,
}

/// Assigns keys to `candidates` by the rules in spec §6.2.
///
/// Requested keys go first; the first target at a level keeps a key. Then the rest are
/// auto-assigned: one letter from the label, then `A` to `Z`, then `1` to `9`. A level with more
/// than 26 targets uses two-letter keys for its auto-assigned targets instead. Keys stay
/// prefix-free, so no one-letter key is also the start of a two-letter key at its level.
pub fn resolve_keytips(candidates: &[KeyTipCandidate]) -> KeyTipResolution {
    let mut report = KeyTipsReport::default();
    let mut keys: Vec<Option<String>> = vec![None; candidates.len()];

    // Requested keys: the first target at a level keeps the key.
    let mut claimed: HashSet<(KeyTipLevel, String)> = HashSet::new();
    for (candidate, slot) in candidates.iter().zip(keys.iter_mut()) {
        let Some(requested) = candidate.requested else {
            continue;
        };
        match normalize_requested(requested) {
            None => report.invalid.push(candidate.target),
            Some(key) => {
                if claimed.insert((candidate.level, key.clone())) {
                    *slot = Some(key);
                } else {
                    report.conflicts.push(candidate.target);
                }
            }
        }
    }

    // A two-letter request whose first letter is a one-letter key at its level clashes with it.
    let singles: HashSet<(KeyTipLevel, char)> = candidates
        .iter()
        .zip(keys.iter())
        .filter_map(|(candidate, key)| match key.as_deref() {
            Some(key) if key.len() == 1 => key.chars().next().map(|ch| (candidate.level, ch)),
            _ => None,
        })
        .collect();
    for (candidate, slot) in candidates.iter().zip(keys.iter_mut()) {
        let clash = match slot.as_deref() {
            Some(key) if key.len() == 2 => key
                .chars()
                .next()
                .is_some_and(|ch| singles.contains(&(candidate.level, ch))),
            _ => false,
        };
        if clash {
            *slot = None;
            report.conflicts.push(candidate.target);
        }
    }

    // Auto-assignment, in candidate order. A level with more than 26 targets uses pairs.
    let mut counts: HashMap<KeyTipLevel, usize> = HashMap::new();
    for candidate in candidates {
        *counts.entry(candidate.level).or_insert(0) += 1;
    }
    let mut pairs_from: HashSet<KeyTipLevel> = counts
        .iter()
        .filter(|(_, count)| **count > 26)
        .map(|(level, _)| *level)
        .collect();
    let mut taken: HashSet<(KeyTipLevel, String)> = candidates
        .iter()
        .zip(keys.iter())
        .filter_map(|(candidate, key)| key.clone().map(|key| (candidate.level, key)))
        .collect();

    for (candidate, slot) in candidates.iter().zip(keys.iter_mut()) {
        if candidate.requested.is_none() {
            report.auto_assigned.push(candidate.target);
        }
        if slot.is_some() {
            continue;
        }
        let mut key = if pairs_from.contains(&candidate.level) {
            None
        } else {
            pick_single(candidate, &taken)
        };
        if key.is_none() {
            // Once a level moves to pairs it never goes back to one-letter keys.
            pairs_from.insert(candidate.level);
            key = pick_pair(candidate, &taken);
        }
        match key {
            Some(key) => {
                taken.insert((candidate.level, key.clone()));
                *slot = Some(key);
            }
            None => report.unassigned.push(candidate.target),
        }
    }

    let targets = candidates
        .iter()
        .zip(keys)
        .filter_map(|(candidate, key)| {
            key.map(|key| KeyTipTarget {
                target: candidate.target,
                level: candidate.level,
                kind: candidate.kind,
                key,
            })
        })
        .collect();

    KeyTipResolution {
        targets: KeyTipTargets { targets },
        report,
    }
}

/// Uppercases a requested key. `None` when it is not one ASCII letter, two ASCII letters or one
/// ASCII digit.
fn normalize_requested(requested: &str) -> Option<String> {
    let key = requested.to_ascii_uppercase();
    let valid = match key.as_bytes() {
        [byte] => byte.is_ascii_uppercase() || byte.is_ascii_digit(),
        [first, second] => first.is_ascii_uppercase() && second.is_ascii_uppercase(),
        _ => false,
    };
    valid.then_some(key)
}

/// The first free one-letter key: a letter or digit of the label, then `A` to `Z`, then `1` to
/// `9`. A key that starts a taken two-letter key is not free.
fn pick_single(candidate: &KeyTipCandidate, taken: &HashSet<(KeyTipLevel, String)>) -> Option<String> {
    let level = candidate.level;
    candidate
        .label
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|ch| ch.to_ascii_uppercase())
        .chain('A'..='Z')
        .chain('1'..='9')
        .map(|ch| ch.to_string())
        .find(|key| {
            !taken.contains(&(level, key.clone()))
                && !taken.iter().any(|(taken_level, taken_key)| {
                    *taken_level == level && taken_key.len() == 2 && taken_key.starts_with(key.as_str())
                })
        })
}

/// The first free two-letter key. The first letter is a letter of the label, then `A` to `Z`.
/// It is never a one-letter key already taken at the level.
fn pick_pair(candidate: &KeyTipCandidate, taken: &HashSet<(KeyTipLevel, String)>) -> Option<String> {
    let level = candidate.level;
    candidate
        .label
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|ch| ch.to_ascii_uppercase())
        .chain('A'..='Z')
        .flat_map(|first| ('A'..='Z').map(move |second| (first, second)))
        .find_map(|(first, second)| {
            if taken.contains(&(level, first.to_string())) {
                return None;
            }
            let key = format!("{first}{second}");
            (!taken.contains(&(level, key.clone()))).then_some(key)
        })
}

/// The state of the KeyTips machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipState {
    /// KeyTips are not shown. Alt or F10 leaves this state.
    Idle,
    /// Root level: tabs, QAT and app-level targets are shown.
    Root,
    /// A content tab was chosen. Its controls are shown.
    Tab {
        /// Key of the chosen tab.
        tab: &'static str,
    },
    /// A menu or gallery popup opened from a control is shown.
    Popup {
        /// Key of the tab the control sits on.
        tab: &'static str,
        /// Key of the control that opened the popup.
        control: &'static str,
    },
    /// The first letter of a two-letter key was typed. `parent` is the state it was typed in;
    /// Escape or Backspace returns to it.
    Prefix {
        /// The state the first letter was typed in.
        parent: Box<KeyTipState>,
        /// The first letter, uppercased.
        first: char,
    },
}

/// One input to the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipInput {
    /// A letter or digit key.
    Char(char),
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Alt or F10 pressed.
    Activate,
    /// Mouse click anywhere, or focus moved outside the chrome.
    Dismiss,
}

/// What the chrome does after a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipAction {
    /// Nothing changes.
    None,
    /// Show the targets for the new state's level.
    Show,
    /// Switch to a tab and show its controls.
    SelectTab(&'static str),
    /// Invoke a target as a normal click or activation.
    Invoke(&'static str),
    /// Open a control's popup (menu, gallery).
    OpenPopup(&'static str),
}

/// The next state and the action to take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyTipStep {
    /// State after the input.
    pub state: KeyTipState,
    /// What the chrome does.
    pub action: KeyTipAction,
}

/// Pure transition. No egui input is read here, so the table is unit-testable.
pub fn step(state: &KeyTipState, input: KeyTipInput, targets: &KeyTipTargets) -> KeyTipStep {
    match state {
        KeyTipState::Idle => match input {
            KeyTipInput::Activate => KeyTipStep {
                state: KeyTipState::Root,
                action: KeyTipAction::Show,
            },
            _ => stay(state, KeyTipAction::None),
        },
        KeyTipState::Root | KeyTipState::Tab { .. } | KeyTipState::Popup { .. } => match input {
            KeyTipInput::Char(ch) => char_step(state, ch, targets),
            KeyTipInput::Escape | KeyTipInput::Backspace => back(state),
            KeyTipInput::Activate | KeyTipInput::Dismiss => stay(&KeyTipState::Idle, KeyTipAction::None),
        },
        KeyTipState::Prefix { parent, first } => match input {
            KeyTipInput::Char(ch) => {
                let key = format!("{first}{}", ch.to_ascii_uppercase());
                match targets.find(level_of(parent), &key) {
                    Some(target) => choose(parent, target),
                    None => stay(state, KeyTipAction::None),
                }
            }
            KeyTipInput::Escape | KeyTipInput::Backspace => stay(parent, KeyTipAction::Show),
            KeyTipInput::Activate | KeyTipInput::Dismiss => stay(&KeyTipState::Idle, KeyTipAction::None),
        },
    }
}

/// The level whose targets a state shows. A prefix shows its parent's level.
fn level_of(state: &KeyTipState) -> KeyTipLevel {
    match state {
        KeyTipState::Tab { tab } => KeyTipLevel::Tab(tab),
        KeyTipState::Popup { control, .. } => KeyTipLevel::Popup(control),
        KeyTipState::Prefix { parent, .. } => level_of(parent),
        KeyTipState::Idle | KeyTipState::Root => KeyTipLevel::Root,
    }
}

/// A letter or digit at a root, tab or popup level: a one-letter match wins, then a two-letter
/// prefix, else nothing changes.
fn char_step(state: &KeyTipState, ch: char, targets: &KeyTipTargets) -> KeyTipStep {
    let upper = ch.to_ascii_uppercase();
    let level = level_of(state);
    if let Some(target) = targets.find(level, &upper.to_string()) {
        return choose(state, target);
    }
    let starts_pair = targets
        .iter()
        .any(|target| target.level == level && target.key.len() == 2 && target.key.starts_with(upper));
    if starts_pair {
        stay(
            &KeyTipState::Prefix {
                parent: Box::new(state.clone()),
                first: upper,
            },
            KeyTipAction::None,
        )
    } else {
        stay(state, KeyTipAction::None)
    }
}

/// Acts on a target chosen at `state`'s level.
fn choose(state: &KeyTipState, target: &KeyTipTarget) -> KeyTipStep {
    match (state, target.kind) {
        (KeyTipState::Root, TargetKind::Tab) => KeyTipStep {
            state: KeyTipState::Tab { tab: target.target },
            action: KeyTipAction::SelectTab(target.target),
        },
        (KeyTipState::Tab { tab }, TargetKind::PopupControl) => KeyTipStep {
            state: KeyTipState::Popup {
                tab,
                control: target.target,
            },
            action: KeyTipAction::OpenPopup(target.target),
        },
        _ => KeyTipStep {
            state: KeyTipState::Idle,
            action: KeyTipAction::Invoke(target.target),
        },
    }
}

/// Escape and Backspace: one level back, with the level's targets shown again.
fn back(state: &KeyTipState) -> KeyTipStep {
    match state {
        KeyTipState::Tab { .. } => stay(&KeyTipState::Root, KeyTipAction::Show),
        KeyTipState::Popup { tab, .. } => stay(&KeyTipState::Tab { tab }, KeyTipAction::Show),
        _ => stay(&KeyTipState::Idle, KeyTipAction::None),
    }
}

fn stay(state: &KeyTipState, action: KeyTipAction) -> KeyTipStep {
    KeyTipStep {
        state: state.clone(),
        action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        target: &'static str,
        level: KeyTipLevel,
        kind: TargetKind,
        requested: Option<&'static str>,
        label: &str,
    ) -> KeyTipCandidate {
        KeyTipCandidate {
            target,
            level,
            kind,
            requested,
            label: label.to_owned(),
        }
    }

    /// Root: File (requested "F"), Home, Insert, and Save (requested "S"). Home tab: Paste
    /// (requested "V"), Bold (requested "1"), and the Font popup control (auto "F"). The Font
    /// popup holds Big and Small (auto "B" and "S"). Keys come from `resolve_keytips`.
    fn sample() -> KeyTipTargets {
        resolve_keytips(&[
            candidate("file", KeyTipLevel::Root, TargetKind::Tab, Some("F"), "File"),
            candidate("home", KeyTipLevel::Root, TargetKind::Tab, None, "Home"),
            candidate("insert", KeyTipLevel::Root, TargetKind::Tab, None, "Insert"),
            candidate("save", KeyTipLevel::Root, TargetKind::QatItem, Some("S"), "Save"),
            candidate(
                "paste",
                KeyTipLevel::Tab("home"),
                TargetKind::Control,
                Some("V"),
                "Paste",
            ),
            candidate(
                "bold",
                KeyTipLevel::Tab("home"),
                TargetKind::Control,
                Some("1"),
                "Bold",
            ),
            candidate(
                "font",
                KeyTipLevel::Tab("home"),
                TargetKind::PopupControl,
                None,
                "Font",
            ),
            candidate(
                "big",
                KeyTipLevel::Popup("font"),
                TargetKind::MenuItem,
                None,
                "Big",
            ),
            candidate(
                "small",
                KeyTipLevel::Popup("font"),
                TargetKind::MenuItem,
                None,
                "Small",
            ),
        ])
        .targets
    }

    fn run(state: &KeyTipState, input: KeyTipInput, targets: &KeyTipTargets) -> KeyTipStep {
        step(state, input, targets)
    }

    fn ch(c: char) -> KeyTipInput {
        KeyTipInput::Char(c)
    }

    fn tab(t: &'static str) -> KeyTipState {
        KeyTipState::Tab { tab: t }
    }

    fn popup(t: &'static str, c: &'static str) -> KeyTipState {
        KeyTipState::Popup { tab: t, control: c }
    }

    fn prefix(parent: KeyTipState, first: char) -> KeyTipState {
        KeyTipState::Prefix {
            parent: Box::new(parent),
            first,
        }
    }

    #[test]
    fn requested_keys_are_uppercased_and_validated() {
        assert_eq!(normalize_requested("f").as_deref(), Some("F"));
        assert_eq!(normalize_requested("7").as_deref(), Some("7"));
        assert_eq!(normalize_requested("qa").as_deref(), Some("QA"));
        assert_eq!(normalize_requested(""), None);
        assert_eq!(normalize_requested("ABC"), None);
        assert_eq!(normalize_requested("-"), None);
        assert_eq!(normalize_requested("é"), None);
        assert_eq!(normalize_requested("A1"), None);
    }

    #[test]
    fn invalid_request_is_auto_assigned_and_reported() {
        let out = resolve_keytips(&[candidate(
            "x",
            KeyTipLevel::Root,
            TargetKind::QatItem,
            Some("##"),
            "Xray",
        )]);
        assert_eq!(out.report.invalid, vec!["x"]);
        assert_eq!(out.report.auto_assigned, Vec::<&str>::new());
        assert_eq!(
            out.targets.find(KeyTipLevel::Root, "X").map(|t| t.target),
            Some("x")
        );
    }

    #[test]
    fn first_requested_key_wins_and_second_is_reassigned() {
        let out = resolve_keytips(&[
            candidate("a", KeyTipLevel::Root, TargetKind::QatItem, Some("S"), "Alpha"),
            candidate("b", KeyTipLevel::Root, TargetKind::QatItem, Some("s"), "Beta"),
        ]);
        assert_eq!(out.report.conflicts, vec!["b"]);
        assert_eq!(
            out.targets.find(KeyTipLevel::Root, "S").map(|t| t.target),
            Some("a")
        );
        // The second target falls back to its label's next free letter.
        assert_eq!(
            out.targets.find(KeyTipLevel::Root, "B").map(|t| t.target),
            Some("b")
        );
    }

    #[test]
    fn same_key_at_different_levels_does_not_conflict() {
        let out = resolve_keytips(&[
            candidate("a", KeyTipLevel::Root, TargetKind::Tab, Some("H"), "Home"),
            candidate(
                "b",
                KeyTipLevel::Tab("home"),
                TargetKind::Control,
                Some("H"),
                "Hide",
            ),
        ]);
        assert!(out.report.conflicts.is_empty());
    }

    #[test]
    fn auto_assignment_prefers_label_letters_then_alphabet_then_digits() {
        let out = resolve_keytips(&[
            candidate("a", KeyTipLevel::Root, TargetKind::Tab, None, "Home"),
            candidate("b", KeyTipLevel::Root, TargetKind::Tab, None, "Hmm"),
            candidate("c", KeyTipLevel::Root, TargetKind::Tab, None, "Home"),
        ]);
        let keys: Vec<&str> = out.targets.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, vec!["H", "M", "O"]);
    }

    #[test]
    fn auto_assignment_is_deterministic_and_collision_free_for_200_labels() {
        let labels: Vec<String> = (0..200).map(|i| format!("Label {i:03} zq{}", i % 7)).collect();
        let build = || -> Vec<KeyTipCandidate> {
            labels
                .iter()
                .map(|label| candidate("t", KeyTipLevel::Root, TargetKind::Control, None, label))
                .collect()
        };
        let first = resolve_keytips(&build());
        let second = resolve_keytips(&build());
        assert_eq!(first, second);
        assert!(first.report.unassigned.is_empty());
        assert_eq!(first.targets.iter().count(), 200);

        let mut keys: Vec<&str> = first.targets.iter().map(|t| t.key.as_str()).collect();
        assert!(
            keys.iter()
                .all(|k| k.len() == 2 && k.chars().all(|c| c.is_ascii_uppercase()))
        );
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 200);
    }

    #[test]
    fn level_with_no_free_key_reports_unassigned() {
        // 26 * 26 two-letter keys exist, and a requested "A" blocks the 26 keys starting with A.
        let mut candidates: Vec<KeyTipCandidate> = (0..700)
            .map(|_| candidate("t", KeyTipLevel::Root, TargetKind::Control, None, "x"))
            .collect();
        candidates.push(candidate(
            "req",
            KeyTipLevel::Root,
            TargetKind::QatItem,
            Some("A"),
            "Req",
        ));
        let out = resolve_keytips(&candidates);
        assert!(!out.report.unassigned.is_empty());
        assert_eq!(out.targets.iter().count() + out.report.unassigned.len(), 701);
        assert!(
            out.targets
                .find(KeyTipLevel::Root, "A")
                .is_some_and(|t| t.target == "req")
        );
    }

    #[test]
    fn two_letter_request_clashing_with_one_letter_key_is_reassigned() {
        let out = resolve_keytips(&[
            candidate(
                "single",
                KeyTipLevel::Root,
                TargetKind::QatItem,
                Some("A"),
                "Alpha",
            ),
            candidate("pair", KeyTipLevel::Root, TargetKind::QatItem, Some("AB"), "Beta"),
        ]);
        assert_eq!(out.report.conflicts, vec!["pair"]);
        assert!(
            out.targets
                .find(KeyTipLevel::Root, "A")
                .is_some_and(|t| t.target == "single")
        );
        assert!(out.targets.iter().all(|t| t.key.len() == 1 || t.target != "pair"));
    }

    #[test]
    fn root_tab_char_selects_tab() {
        let targets = sample();
        let out = run(&KeyTipState::Root, ch('h'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: tab("home"),
                action: KeyTipAction::SelectTab("home")
            }
        );
    }

    #[test]
    fn root_qat_char_invokes_and_closes() {
        let targets = sample();
        let out = run(&KeyTipState::Root, ch('s'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: KeyTipState::Idle,
                action: KeyTipAction::Invoke("save")
            }
        );
    }

    #[test]
    fn root_unknown_char_stays_at_root() {
        let targets = sample();
        let out = run(&KeyTipState::Root, ch('z'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: KeyTipState::Root,
                action: KeyTipAction::None
            }
        );
    }

    #[test]
    fn root_escape_backspace_activate_dismiss_close() {
        let targets = sample();
        for input in [
            KeyTipInput::Escape,
            KeyTipInput::Backspace,
            KeyTipInput::Activate,
            KeyTipInput::Dismiss,
        ] {
            let out = run(&KeyTipState::Root, input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: KeyTipState::Idle,
                    action: KeyTipAction::None
                }
            );
        }
    }

    #[test]
    fn tab_control_char_invokes_and_closes() {
        let targets = sample();
        let out = run(&tab("home"), ch('v'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: KeyTipState::Idle,
                action: KeyTipAction::Invoke("paste")
            }
        );
    }

    #[test]
    fn tab_popup_control_char_opens_popup() {
        let targets = sample();
        let out = run(&tab("home"), ch('f'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: popup("home", "font"),
                action: KeyTipAction::OpenPopup("font")
            }
        );
    }

    #[test]
    fn tab_unknown_char_stays_on_tab() {
        let targets = sample();
        let out = run(&tab("home"), ch('z'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: tab("home"),
                action: KeyTipAction::None
            }
        );
    }

    #[test]
    fn tab_escape_backspace_return_to_root() {
        let targets = sample();
        for input in [KeyTipInput::Escape, KeyTipInput::Backspace] {
            let out = run(&tab("home"), input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: KeyTipState::Root,
                    action: KeyTipAction::Show
                }
            );
        }
    }

    #[test]
    fn tab_activate_dismiss_close() {
        let targets = sample();
        for input in [KeyTipInput::Activate, KeyTipInput::Dismiss] {
            let out = run(&tab("home"), input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: KeyTipState::Idle,
                    action: KeyTipAction::None
                }
            );
        }
    }

    #[test]
    fn popup_item_char_invokes_item_and_closes() {
        let targets = sample();
        let out = run(&popup("home", "font"), ch('b'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: KeyTipState::Idle,
                action: KeyTipAction::Invoke("big")
            }
        );
    }

    #[test]
    fn popup_unknown_char_stays_in_popup() {
        let targets = sample();
        let out = run(&popup("home", "font"), ch('z'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: popup("home", "font"),
                action: KeyTipAction::None
            }
        );
    }

    #[test]
    fn popup_escape_backspace_return_to_tab() {
        let targets = sample();
        for input in [KeyTipInput::Escape, KeyTipInput::Backspace] {
            let out = run(&popup("home", "font"), input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: tab("home"),
                    action: KeyTipAction::Show
                }
            );
        }
    }

    #[test]
    fn popup_activate_dismiss_close() {
        let targets = sample();
        for input in [KeyTipInput::Activate, KeyTipInput::Dismiss] {
            let out = run(&popup("home", "font"), input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: KeyTipState::Idle,
                    action: KeyTipAction::None
                }
            );
        }
    }

    #[test]
    fn idle_activate_shows_root_and_other_inputs_do_nothing() {
        let targets = sample();
        assert_eq!(
            run(&KeyTipState::Idle, KeyTipInput::Activate, &targets),
            KeyTipStep {
                state: KeyTipState::Root,
                action: KeyTipAction::Show
            }
        );
        for input in [
            ch('h'),
            KeyTipInput::Escape,
            KeyTipInput::Backspace,
            KeyTipInput::Dismiss,
        ] {
            assert_eq!(
                run(&KeyTipState::Idle, input, &targets),
                KeyTipStep {
                    state: KeyTipState::Idle,
                    action: KeyTipAction::None
                }
            );
        }
    }

    #[test]
    fn escape_steps_back_one_level_at_a_time() {
        let targets = sample();
        let mut state = popup("home", "font");
        state = run(&state, KeyTipInput::Escape, &targets).state;
        assert_eq!(state, tab("home"));
        state = run(&state, KeyTipInput::Escape, &targets).state;
        assert_eq!(state, KeyTipState::Root);
        state = run(&state, KeyTipInput::Escape, &targets).state;
        assert_eq!(state, KeyTipState::Idle);
    }

    /// Thirty QAT items labelled "X", so the level uses two-letter keys starting with X.
    fn pair_targets() -> KeyTipTargets {
        const NAMES: [&str; 30] = [
            "t00", "t01", "t02", "t03", "t04", "t05", "t06", "t07", "t08", "t09", "t10", "t11", "t12", "t13",
            "t14", "t15", "t16", "t17", "t18", "t19", "t20", "t21", "t22", "t23", "t24", "t25", "t26", "t27",
            "t28", "t29",
        ];
        let candidates: Vec<KeyTipCandidate> = NAMES
            .iter()
            .map(|name| candidate(name, KeyTipLevel::Root, TargetKind::QatItem, None, "X"))
            .collect();
        resolve_keytips(&candidates).targets
    }

    #[test]
    fn two_letter_sequence_selects_target_at_its_level() {
        let targets = pair_targets();
        let pair = targets
            .iter()
            .find(|t| t.key.len() == 2)
            .map(|t| (t.key.clone(), t.target));
        assert!(pair.is_some(), "more than 26 targets gives two-letter keys");
        let (key, target) = pair.unwrap_or_default();
        let first = key.chars().next().unwrap_or('?');
        let second = key.chars().nth(1).unwrap_or('?');

        let typed = run(&KeyTipState::Root, ch(first), &targets);
        assert_eq!(typed.state, prefix(KeyTipState::Root, first));
        assert_eq!(typed.action, KeyTipAction::None);

        let done = run(&typed.state, ch(second), &targets);
        assert_eq!(
            done,
            KeyTipStep {
                state: KeyTipState::Idle,
                action: KeyTipAction::Invoke(target)
            }
        );
    }

    #[test]
    fn prefix_unknown_second_letter_stays_in_prefix() {
        let targets = pair_targets();
        let typed = run(&KeyTipState::Root, ch('X'), &targets);
        let out = run(&typed.state, ch('#'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: typed.state.clone(),
                action: KeyTipAction::None
            }
        );
    }

    #[test]
    fn unknown_first_letter_with_no_pair_does_nothing() {
        let targets = pair_targets();
        let out = run(&KeyTipState::Root, ch('Q'), &targets);
        assert_eq!(
            out,
            KeyTipStep {
                state: KeyTipState::Root,
                action: KeyTipAction::None
            }
        );
    }

    #[test]
    fn prefix_escape_and_backspace_cancel_to_parent() {
        let targets = sample();
        let parent = popup("home", "font");
        let typed = prefix(parent.clone(), 'Z');
        for input in [KeyTipInput::Escape, KeyTipInput::Backspace] {
            let out = run(&typed, input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: parent.clone(),
                    action: KeyTipAction::Show
                }
            );
        }
    }

    #[test]
    fn prefix_activate_dismiss_close() {
        let targets = sample();
        let typed = prefix(tab("home"), 'Z');
        for input in [KeyTipInput::Activate, KeyTipInput::Dismiss] {
            let out = run(&typed, input, &targets);
            assert_eq!(
                out,
                KeyTipStep {
                    state: KeyTipState::Idle,
                    action: KeyTipAction::None
                }
            );
        }
    }

    #[test]
    fn keys_are_matched_without_case() {
        let targets = sample();
        assert!(targets.find(KeyTipLevel::Root, "f").is_some());
        let out = run(&KeyTipState::Root, ch('F'), &targets);
        assert_eq!(out.state, tab("file"));
    }
}
