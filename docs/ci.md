# CI: pull requests and the merge queue

A pull request gets fast feedback on the packages it changed. The merge queue runs
everything. A push to `main` does not re-run `ci.yml`: the queue already passed those
checks before the commit landed. The single required context is **`ci`**; see
[`CONTRIBUTING.md`](../CONTRIBUTING.md#how-changes-land).

## What runs where

| Event | `ci.yml` |
|---|---|
| `pull_request` | the jobs the diff needs; Linux only, one OS |
| `merge_group` (the queue) | everything, Linux and Windows |
| `push` to `main` | nothing from `ci.yml` (the queue already ran it) |

Every job has the same id and name for a given package, so the required `ci` gate is the only
context a ruleset needs. `ci` needs every job and passes when each one succeeded or was skipped
by scope; a path-scoped skip can never leave the required check missing.

## Package scoping on a pull request

The `classify changes` job diffs the PR against its merge base and turns the changed paths
into a package list. Anything that changes workspace resolution or builds takes the full
workspace; an unrecognised `crates/*` path also falls back to full, never to "no check".

| Changed paths | Checks that depend on them |
|---|---|
| `.github/*`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rustfmt.toml`, `rust-toolchain.toml`, `.cargo/*` | full workspace: every job, Linux and Windows, including the downstream fixtures, regeneration drift, icon drift, `cargo deny` and the README index |
| `fixtures/*` | the downstream fixture jobs (`fixtures`, `fixtures-wasm`, `msrv`) |
| `crates/gallery/`, `gallery/`, `docs/images/gallery/`, `scripts/gallery-screenshots.sh`, `scripts/wasm-size-check.sh`, `ci/wasm-budget.txt`, `docs/wasm-size.md` | `gallery` package plus the gallery jobs (`gallery-wasm`, which also enforces `ci/wasm-budget.txt`, and `gallery-screenshots`) |
| `crates/fluent2-egui/*` | `fluent2-egui` **and** `gallery`, because the gallery enables every optional feature and is what exercises them; the README index runs with `fluent2-egui`. Also the downstream fixtures: each optional feature on its own, a wasm consumer, and the MSRV consumer |
| `crates/fluent2-icons/*`, `assets/icons/*`, `xtask/src/icons.rs` | icon drift (`cargo xtask check-icons`, offline) plus `fluent2-icons`, `fluent2-egui` and `gallery` |
| `crates/fluent2-tokens/*`, `design-tokens/*`, `scripts/gen-tokens*`, `scripts/snapshot-tokens*` | regeneration drift (`cargo xtask gen-tokens` must leave the tree clean) plus `fluent2-tokens`, `fluent2-egui` and `gallery` |
| `xtask/*` | `xtask`, regeneration drift, and the crates the generators write (`fluent2-tokens`, `fluent2-egui`, `gallery`) |
| other `scripts/*` (generators) | regeneration drift and `xtask` plus `fluent2-tokens` |
| `README.md` | the README component index (`cargo test -p fluent2-egui --test readme_index`) |
| any other `crates/*` path | full workspace (an unrecognised crate is not safe to narrow), including the downstream fixtures |
| anything else (other docs, markdown, `upstream.toml`) | the full PR set, including the downstream fixtures. An unclassified path fails safe; it never skips every check |

Clippy, nextest and the doctests run with `--workspace` or with one `-p <package>` per package
in the scope. The extra `serde` feature test and the README index run when `fluent2-egui` is in
scope, and the README index also runs for a README-only change. Regeneration drift
(`cargo xtask gen-tokens && git diff --exit-code`, in the `fmt` job) runs when tokens, a
generator or `xtask` changed, and on the full workspace. Icon drift
(`cargo xtask check-icons`) rebuilds `crates/fluent2-icons/src/generated.rs` from the
committed SVGs with no network and fails when that file or a committed SVG differs. It
runs when icon sources change (`crates/fluent2-icons/`, `assets/icons/`, `xtask/src/icons.rs`)
and on the full workspace. `cargo deny check` (advisories and licences, `deny.toml`)
runs when `Cargo.toml`, `Cargo.lock`, `deny.toml` or `.github/` change, and on the full
workspace. `release.yml` uses the same `deny.toml` for its licence step. The Windows leg and `gallery-wasm` for
library-only changes run in the queue, never on a PR or on a push to `main`.

## Downstream fixtures

`fixtures/consumer` is a workspace member with `publish = false`. It depends on
`fluent2-egui` with `default-features = false`, so a check of that crate is a
minimal consumer rather than the gallery (which turns most optional features on
together).

The `fixtures` job runs when `crates/fluent2-egui/`, `fixtures/`, `Cargo.toml`,
`Cargo.lock` or `.github/` change, and whenever the classifier takes the full
set. It is a matrix, one `cargo check -p fluent2-egui-consumer --no-default-features`
per cell: no features, each optional feature on its own (`bundled-font`,
`web-a11y`, `serde`, `terminal`, `diff`, `highlight`, `file-dialog`, `docking`,
`markdown`, `html`, `web-find`, `chart`, `chrono`), and two pairs (`serde,docking`
and `markdown,html`). `snapshots` and `system-preferences` stay out of the matrix:
the first is a dev-only test feature, the second pulls platform bindings that do
not resolve on every target. `fixtures-wasm` checks the same crate with
`--target wasm32-unknown-unknown` and no features. `msrv` installs the declared
`rust-version` (`1.95`) and checks the minimal consumer there, not the whole
workspace on stable.

Locally, from the workspace root:

```sh
cargo check -p fluent2-egui-consumer --no-default-features --locked
cargo check -p fluent2-egui-consumer --no-default-features --features serde --locked
cargo check -p fluent2-egui-consumer --no-default-features --locked --target wasm32-unknown-unknown
```

The wasm line needs the `wasm32-unknown-unknown` target installed. If it is not,
that check is not run locally; CI still runs it.

`scripts/ci-classify-test.sh` feeds sample changed-file lists through the same `case` and checks
which jobs are selected. It does not need a checkout of history.

## Build cache and runner

- Action uses in `ci.yml` are pinned to a commit SHA, with the version in a comment:
  `actions/checkout` v4.2.2, `actions/upload-artifact` v4.6.2, `dtolnay/rust-toolchain`
  (master, which resolves the `stable` and `1.95` toolchains), `taiki-e/install-action`,
  `jetli/trunk-action` v0.5.0 and `EmbarkStudios/cargo-deny-action` v2. `release.yml`
  still uses floating tags except the deny action, which shares the SHA above.
- The screenshot job runs in `rust@sha256:f49565f1…` (`1.95-trixie` on the day it was pinned).
- `Swatinem/rust-cache` is pinned by commit SHA (v2.9.3), with `shared-key: ci`, so every job
  and the queue share the build artefacts. `save-if` is false on `pull_request`: a PR reads
  the cache and only the queue writes it.
- `CARGO_INCREMENTAL: 0` and `CARGO_PROFILE_TEST_DEBUG: 0` keep the cached artefacts small.
- `cargo-nextest` is installed with the SHA-pinned `taiki-e/install-action` and runs the tests;
  doctests are a separate `cargo test --doc` step, because nextest does not run them.

## Gallery screenshots

`gallery-screenshots` runs when the gallery runs. It re-renders the README gallery images in
the `rust:1.95-trixie` container with Mesa lavapipe and fails when a committed PNG in
`docs/images/gallery/` differs. On failure the rendered set is uploaded as the
`gallery-screenshots` artifact and the step prints
`gallery snapshots changed: run scripts/gallery-screenshots.sh --update and commit the images`.
To refresh them locally, run `scripts/gallery-screenshots.sh --update` (needs Docker) and
commit the result.

## Changelog fragments

Feature PRs add `changelog.d/<work-item>-<slug>.md` instead of editing `CHANGELOG.md`, so
release notes do not conflict between parallel PRs. The release PR folds the fragments into
`[Unreleased]` and deletes them. See [`changelog.d/README.md`](../changelog.d/README.md).

## Quarantined: Pages

`.github/workflows/pages.yml` builds the gallery for GitHub Pages, but Pages is not enabled
for the repository, so every `push` to `main` fails at `actions/deploy-pages` with a 404. The
`deploy` job is `continue-on-error: true` until WI-2072 enables Pages: the build still runs
and the failure is visible, but it no longer turns every main build red.

## Concurrency

`ci.yml` cancels in progress per ref (`ci-${{ github.ref }}`). A force-push cancels the
superseded run. Each queue entry has its own `gh-readonly-queue/main/...` ref, so entries never
cancel each other.

## Runners

Every job runs on GitHub-hosted `ubuntu-latest` / `windows-latest`, and queue runs use the same
runners. No self-hosted, GCP or spot labels are involved.

## Landing

`land-pr` (workspace `.agent-tools/bin/land-pr`) detects the merge queue from the base branch's
rulesets. It enqueues with `gh pr merge --auto` and follows the `merge_group` run. Without a
queue it merges once checks pass.

## Rulesets (operator)

`TheDancingDeveloper-org/fluent2-egui` is organisation-owned, so the merge queue is available
and active: squash merges, up to three entries built and merged per run, all-green required,
60-minute timeout. The `main` ruleset requires the `ci` context and allows no bypass actors.

## Release publishing

`.github/workflows/release.yml` runs the gates on a `v*` tag and never publishes
from a tag push. Publishing is a manual `workflow_dispatch` on the tag, with
`publish_crates` ticked, and the `publish` job uses the protected `crates-io`
environment plus a `crates-io-publish` concurrency group so two publishes cannot
run at once.

`scripts/publish-crates.sh` refuses a dirty tree and a tag that is not
`v<workspace version>`. It runs `cargo deny check licenses` (`deny.toml` allows MIT, Apache-2.0 and the other
permissive licences already in the tree: BSD-2-Clause, BSD-3-Clause, BSL-1.0,
ISC, Zlib, Unicode-3.0, OFL-1.1, Ubuntu-font-1.0, CDLA-Permissive-2.0; a
copyleft or unknown licence fails) before any publish,
then `cargo package` and checks the archive lists `LICENSE`, `LICENSE-MIT`,
`LICENSE-APACHE` and `NOTICE`. `LICENSE` must be byte-for-byte the MIT text in
`LICENSE-MIT` (GitHub's licence file). `LICENSE-APACHE` stays beside it, and
`Cargo.toml` still says `MIT OR Apache-2.0`. Each of `fluent2-tokens`, `fluent2-icons` and
`fluent2-egui` is skipped when that version is already on crates.io, so a
re-run resumes after a partial failure. Without `--publish` the script dry-runs
(`cargo publish --dry-run`) and uploads nothing.
