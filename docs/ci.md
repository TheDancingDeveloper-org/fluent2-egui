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
| `.github/*`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `rust-toolchain.toml`, `.cargo/*` | full workspace: every job, Linux and Windows, including regeneration drift and the README index |
| `crates/gallery/`, `gallery/`, `docs/images/gallery/`, `scripts/gallery-screenshots.sh` | `gallery` package plus the gallery jobs (`gallery-wasm`, `gallery-screenshots`) |
| `crates/fluent2-egui/*` | `fluent2-egui` **and** `gallery`, because the gallery enables every optional feature and is what exercises them; the README index runs with `fluent2-egui` |
| `crates/fluent2-icons/*`, `assets/icons/*` | `fluent2-icons` plus the component interaction tests (`fluent2-egui` and `gallery`) |
| `crates/fluent2-tokens/*`, `design-tokens/*`, `scripts/gen-tokens*`, `scripts/snapshot-tokens*` | regeneration drift (`cargo xtask gen-tokens` must leave the tree clean) plus `fluent2-tokens`, `fluent2-egui` and `gallery` |
| `xtask/*` | `xtask`, regeneration drift, and the crates the generators write (`fluent2-tokens`, `fluent2-egui`, `gallery`) |
| other `scripts/*` (generators) | regeneration drift and `xtask` plus `fluent2-tokens` |
| `README.md` | the README component index (`cargo test -p fluent2-egui --test readme_index`) |
| any other `crates/*` path | full workspace (an unrecognised crate is not safe to narrow) |
| anything else (other docs, markdown, `upstream.toml`) | the full PR set. An unclassified path fails safe; it never skips every check |

Clippy, nextest and the doctests run with `--workspace` or with one `-p <package>` per package
in the scope. The extra `serde` feature test and the README index run when `fluent2-egui` is in
scope, and the README index also runs for a README-only change. Regeneration drift
(`cargo xtask gen-tokens && git diff --exit-code`, in the `fmt` job) runs when tokens, a
generator or `xtask` changed, and on the full workspace. The Windows leg and `gallery-wasm` for
library-only changes run in the queue, never on a PR or on a push to `main`.

`scripts/ci-classify-test.sh` feeds sample changed-file lists through the same `case` and checks
which jobs are selected. It does not need a checkout of history.

## Build cache and runner

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
