# WI-1588: UI font options, rendered

Pixel snapshots of the gallery's Typography sample (every `TypeRamp` step) and a Buttons sample
(five appearances × three sizes), at 100% and 150% pixels per point. Named `{option}-{page}-{scale}.png`.

| option | how it was rendered | regular | semibold steps |
|---|---|---|---|
| `inter` | the default build (`bundled-font` on): Inter 4.1 Regular and SemiBold, static | Inter | Inter SemiBold |
| `egui-default` | `--no-default-features --features snapshots`: egui's bundled Ubuntu-Light, no semibold face | Ubuntu-Light | Ubuntu-Light (no bold) |
| `selawik` | temporary local swap of the bundled files for Selawik (microsoft/Selawik @ 89362e84, compiled from the UFO sources with fontmake, not committed) | Selawik Regular | Selawik **Bold** (Selawik has no SemiBold) |

Not rendered: **system font discovery** (Cantarell, Noto Sans). The code does not implement it, and this machine has only DejaVu and Liberation families.

Coverage (from the cmap of each Regular face): Inter 2,852 glyphs including Greek. Selawik 348 glyphs, Latin only.

Regenerate the `inter` and `egui-default` sets with `UPDATE_SNAPSHOTS=1 cargo test -p fluent2-egui --features snapshots --test font_snapshots`, and the same with `--no-default-features` added for `egui-default`.
