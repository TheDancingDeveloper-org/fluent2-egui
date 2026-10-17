# Changelog

All notable changes are recorded here, following [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- TabList and Card surfaces (WI-1554).
- Dropdown and Combobox controls (WI-1549).
- Display controls: `Link`, `Badge`, `Divider`, `Field`, `ProgressBar` and `Spinner` (WI-1550).
- ToggleButton and CompoundButton controls (WI-1543).
- SplitButton and MenuButton controls (WI-1544).
- Menu and ContextMenu: Fluent frame, items with icon, shortcut, checkmark, submenu, divider and group header (WI-1545).
- Checkbox, RadioGroup and Switch controls (WI-1546).
- Input, SearchBox and Textarea: Fluent frame around egui `TextEdit` (WI-1547).
- `SpinButton` and `Slider` controls (WI-1548).
- Button control: primary, secondary, outline, subtle and transparent; small, medium and large; icon and icon-only (WI-1542).
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
