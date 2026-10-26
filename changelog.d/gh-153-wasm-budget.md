### Changed

- The gallery wasm size budget is enforced. `gallery-wasm` runs `scripts/wasm-size-check.sh` against `ci/wasm-budget.txt` (gzip and brotli, each 4,500,000 bytes) and uploads the size report. Raise the budget in that file when a change needs the room (gh:TheDancingDeveloper-org/fluent2-egui#153).

### Added

- `cargo bench -p fluent2-egui --bench large_views` prints frame time for a 10,000-row list, a 10,000-row data grid and a combobox of 10,000 options. It is not a required check (gh:TheDancingDeveloper-org/fluent2-egui#153).
