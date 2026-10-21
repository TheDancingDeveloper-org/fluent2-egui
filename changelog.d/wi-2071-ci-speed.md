### Changed
- CI on a pull request now checks only the packages its diff touches, with a shared,
  SHA-pinned Rust cache and `cargo-nextest`; the merge queue and `main` still run the full
  Linux and Windows matrix. The module, gallery page, README and supporting-type lists are
  sorted one entry per line, and changelog entries are per-PR fragments under `changelog.d/`,
  so parallel PRs no longer conflict on shared lists. The gallery screenshot job now names
  the command to refresh a changed image (WI-2071).
