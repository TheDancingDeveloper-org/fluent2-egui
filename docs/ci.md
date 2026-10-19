# CI: pull requests, the merge queue, and `main`

A pull request gets fast feedback on what it changed. The merge queue (once
enabled) and `main` run everything (WI-1886).

## What runs where

| Event | `ci.yml` |
|---|---|
| `pull_request` | only the jobs the diff touches (below); Linux only |
| `merge_group` (the queue) | everything, Linux and Windows |
| `push` to `main` | everything, Linux and Windows |

### Path scoping on a pull request

The `classify changes` job diffs the PR against its merge base:

| Changed paths | Runs |
|---|---|
| `.github/workflows/`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml` | everything except the Windows leg |
| `crates/gallery/`, `gallery/`, `docs/images/gallery/`, `scripts/gallery-screenshots.sh` | everything except the Windows leg |
| `crates/*`, `xtask/` | fmt + token check, clippy + test (Linux), wasm check, doc, msrv |
| anything else (docs, markdown, `scripts/`, `upstream.toml`) | nothing; the `ci` gate passes |

`test (windows-latest)` and `gallery-wasm` for library-only changes run in
the queue and on `main`, never on a PR.

### Gallery screenshots

`gallery-screenshots` runs when the gallery runs. It re-renders the README
gallery images in the `rust:1.95-trixie` container with Mesa lavapipe and
fails when a committed PNG in `docs/images/gallery/` differs. On failure the
rendered set is uploaded as the `gallery-screenshots` artifact. To refresh the
images locally, run `scripts/gallery-screenshots.sh --update` (needs Docker)
and commit the result.

### The `ci` gate

`ci` needs every job and passes when each one either succeeded or was skipped
by path scope. It is the single context to require, so path scoping never
leaves a required check missing.

### Concurrency

`ci.yml` cancels in progress per ref (`ci-${{ github.ref }}`). A force-push
cancels the superseded run. Each queue entry has its own
`gh-readonly-queue/main/...` ref, so entries never cancel each other.

### Runners

Every job runs on GitHub-hosted `ubuntu-latest` / `windows-latest`, and queue
runs use the same runners. No self-hosted, GCP or spot labels are involved.

## Landing

`land-pr` (workspace `.agent-tools/bin/land-pr`) detects a merge queue from
the base branch's rulesets. It enqueues with `gh pr merge --auto` and follows
the `merge_group` run. Without a queue it merges once checks pass, as before.

## Turning the queue on (operator)

This repository belongs to a personal account. **GitHub offers merge queues
only on organisation-owned repositories**, so the queue rule cannot be added
here until the repository moves to an organisation, for example
TheDancingDeveloper-org. Until then, the `merge_group` trigger is inert. PRs
still get path scoping, cancel-in-progress and the `ci` gate, and the gate can
be required by a plain ruleset. The exact rulesets are in the WI-1886 PR
description.
