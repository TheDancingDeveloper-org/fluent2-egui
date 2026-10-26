# Instructions for agents

fluent2-egui is a Fluent 2 component library for egui: themes, controls and Office-style
application chrome (title bar, ribbon, keytips, backstage). Read
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before writing code. It settles crate layout,
module layout, the component API, theming, testing and gates.

## How a work item is done

1. Find the item in Vogt (index: `docs/BACKLOG.md`) and read it fully. It names the files to create or change, the upstream references
   (pinned URLs), the acceptance criteria and the commands to run.
2. Branch `wi-<number>-<short-slug>` from `main`, or from the branch the item says it stacks on.
3. Touch only the files the item lists, plus a fragment in `changelog.d/` (see [`changelog.d/README.md`](changelog.d/README.md) and [`docs/ci.md`](docs/ci.md)). Do not edit `CHANGELOG.md`; the release PR folds fragments in. If you need to touch more, or need
   a design choice the item and `docs/ARCHITECTURE.md` don't settle, **stop and ask on the work
   item**. Do not guess.
4. Run the gates in `docs/ARCHITECTURE.md` until they pass.
5. Open a PR against `main` (or the stacked branch) that names the work item. **Do not merge.**
6. CI on a PR runs only the jobs your diff touches (Linux only); the merge queue and `main`
   run everything, Windows included. The required context is `ci`. Land through the queue
   (`land-pr`, or `gh pr merge --auto`), never by rebasing a queued PR. See
   [`docs/ci.md`](docs/ci.md).

## Before you push

1. **Gallery screenshots.** The required `gallery-screenshots` check fails if a change adds or
   alters a gallery page (`crates/gallery/`, `gallery/`) without updating `docs/images/gallery/*.png`.
   Run `CARGO_BUILD_JOBS=4 scripts/gallery-screenshots.sh --update` and commit only the PNGs that
   changed. Check that each image change is intended. If the images look wrong because of a bug on
   `main`, stop and report rather than committing them.
2. **Stale or conflicting branches.** Merge `origin/main` into the branch and push normally. Don't
   rebase or force-push; the merge queue rebases on landing.
3. **Shared registration lists** (README component index, `mod.rs`, `Cargo.toml` features) conflict
   often. Keep entries one per line, in sorted order.

## Rules that are never broken

- **Language:** the library and everything that ships or builds into it is Rust. Dev-only
  scripts under `scripts/` may be Python or Node (for example the one-off Fluent token snapshot);
  their outputs are committed, and no build, test or CI gate may need Node.

- **Assets:** no Microsoft fonts (Segoe UI, Segoe UI Variable, Segoe Fluent Icons) in the repo
  or embedded in binaries, no Office artwork or logos, and no image without a row in
  `ATTRIBUTION.md`. Fluent System Icons (MIT) enter only through the `fluent2-icons`
  generator.
- **Upstream code:** follow `UPSTREAM.md` (file header, `[[adaptation]]` entry, NOTICE line).
  Never copy ArtCraft brand files or names.
- **Generated files** (token data, icon catalogue) are changed only by re-running their
  generator.
- **No panics** in library code (see ARCHITECTURE "Component API" rule 9). No `unsafe`.
- **No egui upgrade** inside a feature PR.
- The README's "not affiliated with Microsoft" line stays.
