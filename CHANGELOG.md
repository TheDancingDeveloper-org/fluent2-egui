# Changelog

All notable changes are recorded here, following [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Fluent type ramp as egui text styles, and runtime loading of the Windows system UI font (WI-1538). No font file is added.
- Brand ramps: `BrandRamp`, `FLUENT`, `from_seed`, and brand-aware `light` / `dark` token sets.
- `cargo xtask gen-tokens` generates the `fluent2-tokens` Rust modules from the JSON snapshot.
- Fluent 2 token snapshot (`crates/fluent2-tokens/data/*.json`) from `@fluentui/tokens@1.0.0-alpha.24`.
- Workspace scaffold: `fluent2-tokens`, `fluent2-icons`, `fluent2-egui`.
- Upstream tracking: `upstream.toml`, `UPSTREAM.md`, `scripts/upstream-check.py` and a weekly
  drift workflow.
