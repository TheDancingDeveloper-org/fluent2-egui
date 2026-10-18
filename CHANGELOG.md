# Changelog

All notable changes are recorded here, following [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Vogt GUI port gap analysis (`docs/vogt-port-gap-analysis.md`): Vogt web screens and patterns mapped to fluent2-egui, with gaps sized and assigned to fluent2 or the app.
- Quick Access Toolbar (`office::qat`): icon-only commands with tooltips, a customise menu to show, hide and reorder items and pick the position, and `QatState` owned by the app with `QatState::apply` (WI-1561).
- DataGrid core (`fluent2_egui::data::DataGrid`): columns with fixed, auto and fraction widths clamped to min and max, a virtualised body that calls the cell closure only for visible rows, and a sticky header. `Theme::data` holds the data-view sizes (`DataMetrics`); the optional `serde` feature makes `DataGridState` serialisable (WI-1581).
- Data views spec (`docs/specs/data-views.md`): DataGrid, TreeView, List, NavigationView, SplitView and the docking decision (WI-1580).
- `StatusBar` (bottom strip with left and right slots and a zoom slider) and `CommandBar` (priority overflow into a `More` menu), with `Theme::chrome` holding the office chrome sizes (WI-1569).
- Bundled Inter Regular and SemiBold (OFL-1.1) as the UI font wherever Segoe UI is not available: Linux, macOS, wasm, and Windows without Segoe UI. Segoe UI stays first on Windows. Cargo feature `bundled-font`, on by default (WI-1588).
- KeyTips state machine as pure logic: `office::keytips::state` with `step`, `resolve_keytips` and two-letter keys. No rendering yet (WI-1566).
- Ribbon tab strip: `office::ribbon::Ribbon` with the File tab, content and contextual tabs (with headers), double-click and Ctrl+F1 collapse, and the peek popup. Groups are not built yet (WI-1562).
- Backstage: full-window File view with a navigation column, page closures, Escape and back-arrow close, and focus return to the opener (`office::backstage`, WI-1568).
- Gallery Backstage page: a File button opens the backstage with Home, New, Open, Info and Options placeholder pages (WI-1568).
- GitHub Pages workflow: the wasm gallery is built with trunk and deployed on push to main (WI-1540).
- Dialog surface: modal, non-modal and alert kinds, focus trap, Escape and default-button policy, focus restoration to the opener (WI-1552).
- `fluent2_egui::platform`: OS accent, high contrast, reduced motion and colour scheme behind the `system-preferences` feature (Windows and the Linux desktop portal; unknown elsewhere, including wasm). `theme_from_system` picks light, dark or high contrast and sets reduced motion (WI-1541).
- Window resize edges for undecorated windows: `office::window_edges::window_resize_edges` (WI-1560).
- Office `TitleBar` for undecorated windows: caption buttons, drag, double-click maximise and slots. `Theme::chrome` holds the chrome sizes (`ChromeMetrics`). Gallery example `chrome` (WI-1559).
- Caption glyphs in the `fluent2-icons` catalogue: Subtract (minimise), Maximize and Square Multiple (restore) (WI-1559).
- Tooltip (`Response::fluent_tooltip`) and Popover surfaces (WI-1551).
- TabList and Card surfaces (WI-1554).
- Dropdown and Combobox controls (WI-1549). An open Dropdown takes Escape, so an enclosing dialog stays open (WI-1552).
- Display controls: `Link`, `Badge`, `Divider`, `Field`, `ProgressBar` and `Spinner` (WI-1550).
- ToggleButton and CompoundButton controls (WI-1543).
- SplitButton and MenuButton controls (WI-1544).
- Menu and ContextMenu: Fluent frame, items with icon, shortcut, checkmark, submenu, divider and group header (WI-1545).
- Checkbox, RadioGroup and Switch controls (WI-1546).
- Input, SearchBox and Textarea: Fluent frame around egui `TextEdit` (WI-1547).
- `SpinButton` and `Slider` controls (WI-1548).
- Button control: primary, secondary, outline, subtle and transparent; small, medium and large; icon and icon-only (WI-1542).
- Gallery app (`cargo run -p gallery`): theme switcher, accent seed, alias-colour swatches, type ramp and icon grid. Wasm entry via trunk (WI-1539).
- `Icon` widget and `fluent2_egui::install`: Fluent System Icons at 16/20/24 px, tinted from the theme (WI-1536).
- Foundation helpers: keyboard focus ring, elevation shadows and motion curves (`foundation::{focus, elevation, motion}`).
- `fluent2_egui::theme`: `Theme`, `ThemeKind` and `theme::install`, mapping Fluent alias tokens onto egui visuals.
- Fluent type ramp as egui text styles, and runtime loading of the Windows system UI font (WI-1538). No font file is added.
- `fluent2_tokens` re-exports `AliasColors` and `Shadows`.
- Brand ramps: `BrandRamp`, `FLUENT`, `from_seed`, and brand-aware `light` / `dark` token sets.
- `cargo xtask gen-tokens` generates the `fluent2-tokens` Rust modules from the JSON snapshot.
- Fluent 2 token snapshot (`crates/fluent2-tokens/data/*.json`) from `@fluentui/tokens@1.0.0-alpha.24`.
- Workspace scaffold: `fluent2-tokens`, `fluent2-icons`, `fluent2-egui`.
- `fluent2-icons`: committed Fluent System Icons subset and `cargo xtask gen-icons`.
- Upstream tracking: `upstream.toml`, `UPSTREAM.md`, `scripts/upstream-check.py` and a weekly
  drift workflow.
