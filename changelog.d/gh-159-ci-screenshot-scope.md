### Changed

- PR CI now runs the gallery screenshot check for changes to `fluent2-egui`, icons, tokens and `xtask`, because they change the rendered pages. Before, it waited for the merge queue. `gallery-wasm` still runs only for gallery changes on a PR.
