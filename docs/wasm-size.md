# Wasm size, load time and web file handling

Measured for issue #121 on the gallery (`crates/gallery`), the wasm build that ships to GitHub
Pages. The gallery is the only binary the browser downloads, so it is what this document sizes.

## Method

- Build: `trunk build index.html --release` in `crates/gallery`, trunk 0.21.14, wasm-bindgen
  0.2.129 (matches `Cargo.lock`), `CARGO_BUILD_JOBS=4`. The gallery's trunk link sets
  `data-cargo-profile="wasm-release"`, so the wasm is built with that profile. Trunk runs the wasm
  through wasm-opt for release builds.
- Sizes: bytes of the emitted `gallery-*_bg.wasm`, raw and `gzip -9` (brotli is not installed
  here, so no brotli figure).
- Contributors: `twiggy top` and `twiggy dominators` on the baseline build. `cargo bloat` was not
  run; twiggy gives the data-segment view that cargo-bloat does not.
- Load time: headless Google Chrome 156 driven over the DevTools protocol from Node 22, against
  a local static server. Chrome runs with SwiftShader so eframe's WebGL backend starts.
- Web file handling: code paths read in `platform/file_dialog.rs` and `platform/file_drop.rs`.
  Driving the gallery's file dialog in headless Chrome was not completed (see below).

## Before and after

| Build | wasm raw | wasm gzip -9 | vs baseline (raw / gzip) |
|---|---:|---:|---:|
| Baseline: no `[profile.release]` | 11,808,581 B | 4,832,109 B | — |
| `[profile.wasm-release]`: `opt-level = "s"`, `lto = true`, `codegen-units = 1` (**applied**) | 9,447,086 B | 4,109,368 B | −20.0% / −15.0% |
| Same with `opt-level = "z"` (not applied) | 9,524,175 B | 3,940,644 B | −19.3% / −18.4% |

The applied profile is `[profile.wasm-release]` in the root `Cargo.toml`, which inherits `release`.
The gallery selects it with `data-cargo-profile="wasm-release"` on its trunk link in
`crates/gallery/index.html`, so only the wasm build uses it. It changes code generation only, so
no public API moves. `[profile.release]` is unchanged, so native release builds and downstream
users of the published crates keep their defaults.

`opt-level = "z"` is smaller on the wire: about 169 KB less gzip than `s`. It is not applied: it
trades runtime speed for size, and the issue does not measure frame cost. Pick it if the budget
below needs the extra margin.

## Where the bytes are

Twiggy's top rows on the baseline build are almost all data segments, not code. The largest code
item is about 113 KB; the top nine data segments together are about 2.5 MB. The known embedded
payloads, from the source tree and the registry:

| Payload | Size on disk | Enabled by |
|---|---:|---|
| egui default fonts (Ubuntu-Light, Hack, NotoEmoji, emoji-icon-font) | ≈ 1.42 MB | `eframe` `default_fonts`, gallery |
| syntect default syntaxes (`default_newlines` and `default_nonewlines` packdumps) | ≈ 0.74 MB | `highlight` feature |
| Inter Regular and SemiBold (`crates/fluent2-egui/fonts/`) | ≈ 0.83 MB | `fluent2-egui` `text.rs`, always |
| Fluent System Icons SVGs (`crates/fluent2-icons`) | ≈ 1.7 MB source | `fluent2-icons`, always |

Fonts and syntax tables compress poorly, so they dominate the gzip figure too. Icons are only the
part of the icon set the code references, but the set is embedded as bytes in the generated
module.

## Load time (headless Chrome, local server)

Time from navigation to the first frame with non-blank pixels. Three runs each, desktop profile,
no throttling.

| Build | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| Baseline | 141 ms | 134 ms | 146 ms |
| Size profile | 148 ms | 150 ms | 144 ms |

On a local server the two builds are indistinguishable: the transfer cost is near zero and both
paint in about 150 ms. The size difference is what a real connection pays for.

Throttled profile (4× CPU slowdown, Fast 3G at 1.6 Mbps): a throttled run was started but did not
return a result in this session, so no measured throttled time is reported. The wire-time
estimate from the gzip sizes at 1.6 Mbps is:

- Baseline: 4,832,109 B × 8 / 1.6 Mbps ≈ 24 s of download
- Size profile: 4,109,368 B × 8 / 1.6 Mbps ≈ 21 s of download

The estimate ignores JavaScript compile, which is a fixed cost and the same for both.

## Web file open and save

The gallery's File dialog page has three actions: Open file, Open several, Save sample. On wasm:

- **Open** runs through `rfd`'s async file dialog on the browser event loop. The app polls the
  outcome each frame; nothing blocks. Filters are not applied on the web.
- **Save** writes through the browser's download prompt. The file name comes from the app.
- **Drop** (`FileDropZone`) reads dropped files with the browser's async `File` read. Files over
  `max_bytes` (default 50 MiB) are refused before reading, using the `File` size, so the
  page never holds an oversized buffer.

Not measured: a scripted run of the Open and Save buttons in headless Chrome. It needs a file
chooser intercept and a way to click into the egui canvas, which the gallery does not expose by
URL. The code paths above are from reading the source, not from a browser run.

## Decisions and recommendations

**Applied:** the `wasm-release` profile above (`opt-level = "s"`, `lto = true`,
`codegen-units = 1`), selected only by the gallery's trunk build. Native builds keep
`[profile.release]`, so their speed, build time and output are unchanged.

**CI cost:** the fat LTO and `codegen-units = 1` make the final wasm link of the gallery trunk job
serial. That may add a few minutes to that job on a cold cache. Job time before and after is not
measured here; the PR records it.

**Not applied, cheap to consider later:**

1. `opt-level = "z"` for the wasm build only: about 170 KB less gzip, at a speed cost that
   has not been measured.
2. Drop eframe's `default_fonts` from the gallery and rely on Inter for the text it shows: about
   1.4 MB raw, but emoji and the egui icon glyphs would go missing, so this needs a visual check.
3. Subset Inter to the Latin range the gallery uses: up to about 0.8 MB raw. Changes glyph
   coverage, so it is a design call, not a safe win.
4. Remove the `highlight` feature from the gallery: about 0.7 MB raw, but the CodeView page
   goes with it.

The optional `color-emoji` feature embeds Noto Emoji COLR (~2.7 MB raw). The gallery turns
the feature on, but the font bytes are not compiled for `wasm32`, so the default gallery wasm
keeps egui's monochrome emoji fallbacks and stays inside the budget below. Native builds that
enable `color-emoji` still install the colour face.

**Budget (enforced):** gzip ≤ 4,500,000 bytes and brotli ≤ 4,500,000 bytes for the gallery
wasm (`*_bg.wasm` from `trunk build index.html --release`). The numbers live in
`ci/wasm-budget.txt`. `scripts/wasm-size-check.sh` compares the build and exits 1 when either
measured size is over its line (brotli is checked only when the `brotli` command is installed).
The `gallery-wasm` job runs that script and uploads `wasm-size.txt` as the `wasm-size` artifact,
including when the gate fails. The job runs on the gallery path class from the change classifier
(`crates/gallery/`, `gallery/`, `docs/images/gallery/`, `scripts/gallery-screenshots.sh`,
`scripts/wasm-size-check.sh`, `ci/wasm-budget.txt`, `docs/wasm-size.md`) and on the full set.

To raise the budget, change the number in `ci/wasm-budget.txt` in the same change that grows the
wasm, and say why in this section. A raise is a conscious decision, not a silent one: the gate
fails the build otherwise. 4,500,000 bytes is the previous recommendation (about 10% over the
4,109,368 byte gzip of the size-profile build). Pick `opt-level = "z"` only if a raise would
otherwise be needed.

Frame time for a 10,000-row list, a 10,000-row data grid and a combobox of 10,000 options is
`cargo bench -p fluent2-egui --bench large_views` (`crates/fluent2-egui/benches/large_views.rs`).
It prints wall time, allocation count and allocated bytes per frame over 60 frames after a
warm-up frame. Allocations come from `stats_alloc` installed as the bench binary's global
allocator (a dev-dependency; no `unsafe`). It is not a required check and it does not fail CI.

## Follow-ups

Still not done:

- Throttled time-to-first-frame (4× CPU, Fast 3G). A throttled headless run was started for the
  original measurement and did not return; it is not part of this gate.
- A scripted Open and Save pass of the gallery file dialog. It needs a file-chooser intercept
  and a click into the egui canvas, which the gallery does not expose by URL.

Done alongside the budget:

- Measure throttled time-to-first-frame in a headless run that waits for the wasm download to
  finish, then a scripted Open and Save pass. (Kept as the original note; both stay open, see above.)
- Measure the gallery trunk job's CI time before and after the profile lands.
- Add the before and after table to the gallery docs and the README defaults, as the issue
  asks, once the budget is agreed.
