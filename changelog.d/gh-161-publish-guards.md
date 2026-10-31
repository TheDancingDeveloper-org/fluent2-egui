### Changed

- Crate publishing checks that the tagged commit is on `main` and asks crates.io for each exact version, so a re-run skips crates that are already published.
- The release workflow runs a publish dry run (licence check, archive inspection, `cargo publish --dry-run`) on PRs that touch the release path and nightly.
