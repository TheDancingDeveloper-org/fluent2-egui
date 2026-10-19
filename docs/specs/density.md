# Density: compact, medium and touch

Status: WI-1805, 2026-10-19. Gap G4 (library part) in `docs/vogt-port-gap-analysis.md`. The
phone layout itself (bottom sheet in place of "more", safe-area insets) stays with the app.

## Decisions

| # | Decision | Why |
|---|---|---|
| DN1 | `Theme` gains `density: Density` (`Compact`, `Medium`, `Touch`), Medium by default. `Theme::with_density` sets it and rebuilds `Theme::data` with `DataMetrics::for_density`. | One setting, read where the theme already is (`Theme::get`). No control gains a density argument, so apps change nothing to get Medium, and switch every control at once. |
| DN2 | Density maps **heights** in one place, `Density::height(base)`, where `base` is the Fluent 2 Medium-density height a control already uses. Compact: `base >= 32` → `base - 8` (40 → 32, 32 → 24); smaller bases are kept. Medium: unchanged. Touch: `max(base, 44)`. | Fluent 2 web has three control sizes (24/32/40) and no density token. Compact is "one size down", which keeps the result on Fluent's own size steps. Indicators under 32 (checkbox box, radio, switch track, slider thumb) have no smaller Fluent size, so Compact leaves them. |
| DN3 | **Touch grows controls to 44**: the allocated rect and the painted control are at least 44 × 44 (icon-only buttons are 44 square; checkbox, radio and switch rows are 44 tall and at least 44 wide; a slider's cross axis is 44). No transparent hit padding. | Operator decision on WI-1805. 44 pt is the Apple HIG and Fluent mobile minimum. Growing the real rect avoids overlapping hit areas, which egui's hit-testing resolves by draw order, and keeps one rect per control. |
| DN4 | Fonts, icons and indicator glyphs keep their size in every density. Content is centred in the taller or shorter rect. | Density is about targets and rhythm, not zoom. Users zoom with egui's `zoom_factor`. |
| DN5 | Spacing comes from tokens: `item_spacing` is `spacing{Horizontal,Vertical}XS` (Compact), `S` (Medium) or `M` (Touch). `interact_size.y` is `height(32)`, and its width is at least 44 in Touch. `button_padding` is unchanged. | Rule 7: sizes from tokens. egui's own widgets inside our layouts follow the same rhythm. |
| DN6 | Data views read `Theme::data`, which `with_density` rebuilds: every row height goes through `Density::row` (Compact: grid 44 → 36, list and tree 32 → 24; Touch: every row at least 44, selection cell at least 44). Pane widths and breakpoints do not change. | The data views already size from `DataMetrics` (data-views D6). Apps that set `DataMetrics` fields by hand keep their values until they call `with_density` again. |
| DN7 | **Office chrome is out of scope**: title bar, ribbon, QAT, backstage, command bar and status bar keep `ChromeMetrics` in every density. | They are desktop chrome with sizes fixed by the office-chrome spec (D2). A touch app does not show a ribbon. |

## What follows density

| Area | Base (Medium) | Where |
|---|---|---|
| Button, ToggleButton, MenuButton | `ButtonSize` 24/32/40 | `button::Metrics::at` |
| SplitButton (and its chevron box) | 24/32/40 | `split_button::Metrics::at` |
| Dropdown, Combobox fields | 24/32/40 | `dropdown::field_height` |
| Dropdown and Combobox option rows | 32 | `dropdown::option_height` |
| Input, SearchBox (single line) | field height | `input` |
| SpinButton | 32, chevron column 24 (44 wide in Touch) | `spin_button` |
| Checkbox, RadioGroup, Switch | indicator or label height | each control |
| Slider | 20 cross axis | `slider` |
| Menu items | 32 | `surfaces::menu` |
| TabList, TabStrip | tab height | `surfaces::{tab_list, tab_strip}` |
| DataGrid, TreeView, List, NavigationView rows | `DataMetrics` | `DataMetrics::for_density` |

Not affected: Link (inline text), Textarea and multi-line Input (their height is their rows),
CompoundButton (already taller than 44), and non-interactive Badge, Divider, ProgressBar,
Spinner and Field labels.

## API

```rust
pub enum Density { Compact, #[default] Medium, Touch }
pub const TOUCH_TARGET: f32 = 44.0;

impl Density {
    pub fn height(self, base: f32) -> f32;  // DN2
    pub fn row(self, base: f32) -> f32;     // same mapping, for rows
    pub fn min_target(self) -> f32;         // 44 in Touch, else 0
    pub fn item_spacing(self) -> egui::Vec2; // DN5
}

impl Theme { pub fn with_density(self, density: Density) -> Self; }
impl DataMetrics { pub fn for_density(density: Density) -> Self; }
```

`Density` derives serde behind the `serde` feature, like the other state types (data-views D5).

## Acceptance

- `tests/density.rs` measures every control in the table at all three densities: Medium keeps
  today's sizes, Compact steps 32 → 24, and in Touch every target is at least 44 × 44.
- The gallery's top bar has a Density switch that reinstalls the current theme.
