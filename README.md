# fluent2-egui

Fluent 2 themes, controls and Office-style application chrome for [egui](https://github.com/emilk/egui):
light, dark and high-contrast themes generated from Fluent 2 design tokens, Fluent System Icons, and
the pieces desktop productivity apps need: title bar, quick access toolbar, ribbon with keytips,
backstage/File view, command bar, status bar, data grid and tree view. It runs natively on
Windows and Linux, and on the web through wasm.

**Status: scaffold (0.0.0).** Nothing is usable yet. Milestones:

| milestone | contents |
|---|---|
| M0 Foundations | token and icon generators, theme installation, fonts, focus ring, gallery (native and wasm), CI |
| M1 Office chrome | title bar, QAT, ribbon, keytips, backstage, status bar |
| M2 Core controls | buttons, inputs, selection controls, menus, dialog, message bar, toast, tab list |
| M3 Data and layout | data grid, tree view, navigation view, split view, docking |
| M4 0.1 release | docs, crates.io, hosted gallery |

## Crates

| crate | purpose |
|---|---|
| `fluent2-tokens` | Fluent 2 design tokens as plain Rust data (no UI dependency) |
| `fluent2-icons` | Fluent System Icons subset (MIT) |
| `fluent2-egui` | the egui library |

## egui compatibility

| fluent2-egui | egui |
|---|---|
| 0.0.x | 0.36 |

## Upstream sources

This project generates from, adapts and follows several fast-moving upstreams (egui, Fluent UI
tokens, Fluent System Icons, the storytold Craft apps). They are pinned and tracked in
[`upstream.toml`](upstream.toml); see [`UPSTREAM.md`](UPSTREAM.md).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, except
`fluent2-icons`, which is MIT only. Notices are in [NOTICE](NOTICE) and per-asset attribution is
in [ATTRIBUTION.md](ATTRIBUTION.md).

Not affiliated with or endorsed by Microsoft. "Fluent" refers to Microsoft's public Fluent 2
design system, whose open-source tokens and icons this project uses under their MIT licences.
Microsoft fonts are never bundled. On Windows the system's Segoe UI is used at runtime.
