### Added

- Design doc `docs/specs/web-accessibility-layer.md`: a hidden accessible DOM layer that mirrors the AccessKit tree on wasm (issue #108).
- Gallery prototype behind the `web-a11y-dom` feature: on wasm, labels and buttons are mirrored as transparent, positioned DOM elements so browser find and screen readers see them; activating a mirrored button clicks it in egui.
