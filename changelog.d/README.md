# Changelog fragments

A pull request records its user-visible change here instead of editing
[`../CHANGELOG.md`](../CHANGELOG.md), so two PRs never conflict on the release notes.

- One file per PR, named `<work-item>-<slug>.md`, for example `wi-2071-ci-speed.md`.
- Keep the same headings as the changelog: `### Added`, `### Changed`, `### Fixed`,
  `### Removed`. Use only the ones that apply.
- One bullet per change, in the same voice as the changelog.
- The release PR folds every fragment into `[Unreleased]` and deletes the files. Feature PRs
  never edit `CHANGELOG.md` itself.

Nothing builds from these files; they are read only when a release is prepared.
