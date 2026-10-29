### Fixed

- Builds with `RUSTFLAGS=-D warnings` under `--no-default-features` and single optional features (including `html`), and on `wasm32-unknown-unknown`. Colour emoji still installs when `bundled-font` is on.
