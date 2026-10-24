# Tracking upstream sources

Several sources this project uses move fast. The storytold apps land hundreds of commits a day,
egui ships a breaking minor every two to four months, and Fluent's token and icon repos change
weekly. This file explains how we pin them, notice change, and stay attributable.

## The manifest

[`upstream.toml`](upstream.toml) lists every source with:

- the **pinned revision** (`rev`, a full SHA) and the date it was last **reviewed**;
- its **role**:
  - `dependency`: a Cargo dependency (egui);
  - `generated`: our files are generated from its data (Fluent tokens, Fluent System Icons);
  - `adapted`: our files contain code adapted from it;
  - `reference`: behaviour reference only, nothing copied;
- the **paths we watch**, so drift reports ignore the rest of a large repo;
- its **licence**.

`[[adaptation]]` entries map each of our files that contains adapted code to the upstream file
and revision it came from.

## Checking for drift

```
scripts/upstream-check.py                         # every source
scripts/upstream-check.py wordcraft egui          # selected sources
scripts/upstream-check.py --fail-on-adapted-drift # exit 2 if an adapted file changed upstream
```

The script uses the GitHub compare API. It reports commits since each pin, the watched paths
that changed (with a compare link), and any adapted file whose upstream source changed. Set
`GITHUB_TOKEN` or `GH_TOKEN` to avoid the 60-requests-per-hour anonymous limit.

The scheduled workflow `.github/workflows/upstream.yml` runs it weekly and on demand, and writes
the report to the job summary. It never changes the repository.

## What to do when something moved

| role | action |
|---|---|
| `dependency` (egui) | Patch releases: let Cargo update the lock and run the gates. A **new minor**: open an "egui 0.N upgrade" work item. Never fold an egui upgrade into a feature PR. |
| `generated` | Re-run the generator at the new revision. Review the diff of the generated files. Bump `rev` and `reviewed` in the same commit. |
| `adapted` | Read the upstream diff of the adapted file. Port relevant fixes, or record "reviewed, not applicable" in the commit message. Bump the `[[adaptation]]` `rev` and the source's `reviewed`. |
| `reference` | Skim the watched-path diff. If behaviour we copy changed, open a work item. Bump `rev` and `reviewed`. |

Bumping a pin is always its own commit (`upstream: bump <name> to <short-sha>`) and says what
was reviewed. A pin is never bumped without a review.

## Adapting code: attribution rules

Only sources whose `licence` allows it (MIT, Apache-2.0, or a dual licence of those). When code
is adapted:

1. Add a file header:
   ```
   // Adapted from <owner/repo> <path> @ <short-sha>.
   // Copyright (c) <year> <upstream copyright holder>. Licensed MIT OR Apache-2.0.
   ```
2. Add an `[[adaptation]]` entry to `upstream.toml`, and change the source's role to `adapted`.
3. Add a line to the "Source provenance" section of `NOTICE`.

Never copy: ArtCraft brand files or names (`docs/brand/` in storytold repos), Microsoft fonts
(Segoe *), Office artwork or logos, or images without a traceable licence (for example the
sample PNGs in laststudio/fluent-egui).

## Vendored syntax grammars

`crates/fluent2-egui/syntaxes/*.sublime-syntax` is loaded by the `highlight` feature because
syntect's bundled set has no TypeScript.

TypeScript (`TypeScript.sublime-syntax`) comes from `typescript-tmlanguage`
(microsoft/TypeScript-TmLanguage `TypeScript.tmLanguage`) converted by `sublime-syntax-convertor`
and published by rushsinging/aemeath. To re-sync it: take that published file (or regenerate it
with the pinned converter and confirm the bytes match), keep the provenance header and the
`.mts` / `.cts` extensions, keep `LICENSE-TYPESCRIPT-TMLANGUAGE.txt`, and bump both pins and
their `[[adaptation]]` revisions in the same commit.

TSX (`TypeScriptReact.sublime-syntax`) still comes from `bat` and `typescript-sublime-plugin`:
bat's conversion method applied to `TypeScriptReact.tmLanguage`. Re-sync that file from those
pins, and keep `LICENSE-TYPESCRIPT-SUBLIME.txt`.

## Review log

- **2026-10-24** (#100, WI-2425). Compared every pin to its branch head. No watched-path drift, and `reviewed` set to 2026-10-24, for rdpapp (`32e02d3a8d2c`), vte, pulldown-cmark (`61d95cedd740`; five commits, none on `pulldown-cmark/src/lib.rs` or its `Cargo.toml`), egui_commonmark, imara-diff, syntect, bat (`a9aa73d21545`; the vendored TypeScript syntax is unchanged), egui (still 0.36, no new minor), fluentui (`6d3e9d686434`; `packages/tokens/src` unchanged since `93e324f72f0e`), selawik, inter, winui-gallery, laststudio-fluent-egui and fluent-gpui. Left unbumped, because a watched path changed and the review is not finished: html5gum (`src/emitters/default.rs`, `Cargo.toml`; still crates.io 0.8.4, no new release), typescript-sublime-plugin (`TypeScript.tmLanguage`, last edit 2023, not yet re-vendored), fluentui-system-icons (298 files under `assets/`, metadata and RTL variants, generator not re-run), craft-libs (docking work in `crates/ui` on 2026-10-24; adapted `geometry.rs` / `resize.rs` not re-read), and the reference apps wordcraft, gridcraft and deckcraft (watched ui-egui files moved; behaviour we copy was not re-checked).
