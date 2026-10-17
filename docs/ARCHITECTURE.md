# Architecture and conventions

This document fixes the decisions a contributor would otherwise have to make. Work items point
here. If an item needs a choice this document does not settle, **stop and ask on the item**.
Do not invent a new convention.

## Language

The library, generators (`xtask`) and examples are Rust. Dev-only scripts in `scripts/` may be
Python or Node; their outputs are committed and nothing in the build or the CI gates needs Node.

## Crates

| crate | depends on | licence | contents |
|---|---|---|---|
| `fluent2-tokens` | nothing | MIT OR Apache-2.0 | Generated Fluent 2 token data. No egui, no I/O at runtime. |
| `fluent2-icons` | nothing | MIT | Generated subset of Fluent System Icons (SVG bytes plus a name enum). |
| `fluent2-egui` | `egui`, `fluent2-tokens`, `fluent2-icons` | MIT OR Apache-2.0 | Theme installation, foundation, controls, surfaces, Office chrome, data views. |
| `gallery` (later, `publish = false`) | `fluent2-egui`, `eframe` | MIT OR Apache-2.0 | Demo app, native and wasm. |

Nothing below `fluent2-egui` may depend on egui. Platform-specific code (Windows accent, Linux
portal) lives in `fluent2-egui::platform`, behind `cfg` and an optional cargo feature, and
always has a non-platform fallback.

## Module layout of `fluent2-egui`

```
src/
  lib.rs            re-exports; `prelude` module
  theme/            Theme, ThemeKind {Light, Dark, HighContrast}, install(ctx, &Theme), Theme::get(ctx)
  foundation/       icon.rs, text.rs (type ramp helpers), focus.rs, elevation.rs, motion.rs
  controls/         one file per control: button.rs, split_button.rs, checkbox.rs, ...
  surfaces/         dialog.rs, popover.rs, menu.rs, message_bar.rs, toast.rs, tab_list.rs, card.rs
  office/           title_bar.rs, qat.rs, ribbon/ (mod.rs, tab.rs, group.rs, layout.rs), keytips.rs, backstage.rs, status_bar.rs, command_bar.rs
  data/             data_grid/, tree_view.rs, list.rs, nav_view.rs
  platform/         windows.rs, linux.rs, fallback.rs
tests/              one file per component (see Testing)
```

A new component is one new file in the right folder, plus one `pub mod`/`pub use` line, plus
one test file. Do not reorganise folders as part of a component item.

## Component API (every component follows this)

1. **Builder struct implementing `egui::Widget`.** Example:
   `ui.add(Button::new("Save").appearance(Appearance::Primary).icon(Icon::Save))` returns
   `egui::Response`. Constructors take only the required content. Everything else is a
   chainable method with a Fluent default.
2. **Multi-part controls return a named struct**, never a tuple. Example: `SplitButton::show(ui)`
   returns `SplitButtonResponse { main: Response, menu: Response }`.
3. **Containers take a closure** and return what it returns:
   `Dialog::new(id).title("…").show(ctx, |ui| { … }) -> Option<R>` (`None` when closed).
4. **IDs are supplied by the caller** (`impl std::hash::Hash`) or derived from the parent `Ui`
   id. Never derive an id from a translated label.
5. **No command dispatch.** Components return responses or values. The app decides what
   happens.
6. **Enabled state** comes from egui (`ui.add_enabled(false, …)`, `ui.is_enabled()`). Do not
   add an `enabled` field.
7. **Colours, sizes, radii, spacing and durations come from the theme.**
   `let t = Theme::get(ui.ctx());` then `t.color.neutral_background_1`,
   `t.size.spacing_horizontal_m` and so on. A literal `Color32` or a magic number outside
   `theme/` is a bug.
8. **Accessibility is part of "done".** Every interactive component calls
   `response.widget_info(|| egui::WidgetInfo::labeled(WidgetType::…, enabled, label))`. When
   egui's `WidgetType` has no matching role, use `ctx.accesskit_node_builder(id, |node| …)` to
   set the AccessKit role. Every component is reachable by keyboard and paints the Fluent
   focus ring (`foundation::focus::paint_focus_ring`) when `response.has_focus()`.
9. **No panics** in non-test code: no `unwrap`, `expect`, `panic!` or indexing on caller
   data. Non-finite geometry (NaN sizes, inverted ranges) is clamped, never asserted.
10. **Public items have doc comments** with a short example (`missing_docs` is a warning; CI
    treats warnings as errors).

## Theme

- `Theme` is a plain struct built from `fluent2-tokens`:
  `Theme::light(brand: BrandRamp)`, `Theme::dark(brand)`, `Theme::high_contrast()`.
  `BrandRamp::from_seed(Color32)` builds the 16-step ramp. The default ramp is Fluent's own
  brand ramp, not any Office product colour.
- `theme::install(ctx, &theme)` writes egui `Visuals` and `Spacing` and stores the `Theme` in
  `ctx.data`. `Theme::get(ctx)` reads it back, falling back to `Theme::light(default)`.
- Field names mirror Fluent token names in snake_case (`colorNeutralForeground1` becomes
  `color.neutral_foreground_1`), so a token can be found in Fluent's documentation by name.

## Fonts

On Windows, load "Segoe UI Variable" (then "Segoe UI") from the OS at runtime. Elsewhere use an
open fallback chosen in a dedicated item. **Never commit or embed a Microsoft font file.**

## Testing

- Each component has `tests/<component>.rs` using `egui_kittest` **without** the wgpu feature.
  Those tests find the widget through the AccessKit tree by role and label, and check:
  rendering without panic in light, dark and high contrast; click or keyboard activation;
  disabled behaviour; focus-ring presence. Pattern: `tests/README.md` (added with the first
  component).
- Pixel snapshot tests (wgpu) sit behind the `snapshots` cargo feature and run in a separate
  CI job.
- Generators (`xtask` or `scripts/`) have their own tests that the generated output is
  reproducible: running the generator twice gives byte-identical files.

## Gates (run before every commit; CI runs the same)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p fluent2-egui --features serde
cargo check -p fluent2-egui --target wasm32-unknown-unknown
```

## Versioning

- `0.x` while egui is `0.x`. **One egui minor per release line**: the workspace pins
  `egui = "0.<minor>"`. Moving to a new egui minor is its own work item and bumps our minor.
- `fluent2-tokens` and `fluent2-icons` version independently of egui.
- `CHANGELOG.md` uses Keep a Changelog sections. Every PR that changes behaviour adds a line.

## Upstreams

Every external source we generate from, adapt code from or follow as a reference is listed in
`upstream.toml`. The rules are in `UPSTREAM.md`. When a work item adapts code, the same PR adds
the `[[adaptation]]` entry and the file header.
