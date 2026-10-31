# Contributing

Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) first: it settles crate and module layout,
the component API, theming and the gates. Work items live in Vogt (index in
[`docs/BACKLOG.md`](docs/BACKLOG.md)); do one item per land.

## How changes land

Changes land through graft, batched. Workers `g land` on graft line `backlog-1`. Oversight cuts one batch PR from the line (`graft-mirror pr-batch`) and merges it with `land-pr --method rebase`. When a backlog of work is under way, all of it goes into a single batch PR. Batch rules: workspace `.claude/rules/delivery-process.md` ("Landing").

- Graft line: `backlog-1`; mirror: `graft-dev-mirror-fluent2`
- Required checks: `ci` (the single required context; every job in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) must succeed or be skipped by path scope)
- Merge queue: yes (`main` is protected by a ruleset with a merge queue; `main` takes no direct pushes)

Version numbers change only in the release PR or at tag time, never in a feature PR. A
behaviour change adds a fragment under [`changelog.d/`](changelog.d/README.md); release notes
are folded there at release. The full CI map is in [`docs/ci.md`](docs/ci.md).

### What a worker runs

Check the package you changed, not the whole workspace, then run the formatting gate:

```bash
cargo check -p <crate>
cargo nextest run -p <crate>
cargo fmt --all --check
```

For the library, `cargo nextest run -p fluent2-egui` and `cargo test -p fluent2-egui --doc`.
A change to the gallery or to regenerated images also needs `cargo test -p gallery` and the
screenshot check (`scripts/gallery-screenshots.sh`). The CI `ci` job runs clippy with
`-D warnings`, nextest, doctests, the wasm build, docs and the MSRV check; run the relevant
part locally when the package is not covered by your two commands.

## Adding a component without conflicts

Shared registration lists are one entry per line and sorted, so two PRs adding a component
touch different lines:

- Create `src/controls/<name>.rs` (or `surfaces/`, `data/`).
- Add `mod <name>;` in the module's `mod.rs`, in sorted position, and one `pub use <name>::…`
  block with one name per line, sorted.
- Add one row to the matching table in `README.md`, in sorted position, and list any new
  supporting type in `crates/fluent2-egui/tests/readme_index.rs`.
- Add `changelog.d/<work-item>-<slug>.md` with the user-visible change.
- Add a gallery page as `crates/gallery/src/pages/<name>.rs`, export it from
  `crates/gallery/src/pages/mod.rs`, and add one `Box::new(...)` line to `pages()` in sorted
  position by `Page::name`. `cargo test -p gallery pages_are_sorted_by_name` keeps the order.

Do not reflow an existing list to make room for your entry; add the single line.
