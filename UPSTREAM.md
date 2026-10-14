# Tracking upstream sources

Several sources this project uses move fast. The storytold apps land hundreds of commits a day,
egui ships a breaking minor every two to four months, and Fluent's token and icon repos change
weekly. This file explains how we pin them, notice change, and stay attributable.

## The manifest

[`upstream.toml`](upstream.toml) lists every source with:

- the **pinned revision** (`rev`, a full SHA) and the date it was last **reviewed**;
- its **role**:
  - `dependency`: a Cargo dependency (egui);
  - `generated`: our files are generated from its data (Fluent tokens, Fluent System Icons);
  - `adapted`: our files contain code adapted from it;
  - `reference`: behaviour reference only, nothing copied;
- the **paths we watch**, so drift reports ignore the rest of a large repo;
- its **licence**.

`[[adaptation]]` entries map each of our files that contains adapted code to the upstream file
and revision it came from.

## Checking for drift

```
scripts/upstream-check.py                         # every source
scripts/upstream-check.py wordcraft egui          # selected sources
scripts/upstream-check.py --fail-on-adapted-drift # exit 2 if an adapted file changed upstream
```

The script uses the GitHub compare API. It reports commits since each pin, the watched paths
that changed (with a compare link), and any adapted file whose upstream source changed. Set
`GITHUB_TOKEN` or `GH_TOKEN` to avoid the 60-requests-per-hour anonymous limit.

The scheduled workflow `.github/workflows/upstream.yml` runs it weekly and on demand, and writes
the report to the job summary. It never changes the repository.

## What to do when something moved

| role | action |
|---|---|
| `dependency` (egui) | Patch releases: let Cargo update the lock and run the gates. A **new minor**: open an "egui 0.N upgrade" work item. Never fold an egui upgrade into a feature PR. |
| `generated` | Re-run the generator at the new revision. Review the diff of the generated files. Bump `rev` and `reviewed` in the same commit. |
| `adapted` | Read the upstream diff of the adapted file. Port relevant fixes, or record "reviewed, not applicable" in the commit message. Bump the `[[adaptation]]` `rev` and the source's `reviewed`. |
| `reference` | Skim the watched-path diff. If behaviour we copy changed, open a work item. Bump `rev` and `reviewed`. |

Bumping a pin is always its own commit (`upstream: bump <name> to <short-sha>`) and says what
was reviewed. A pin is never bumped without a review.

## Adapting code: attribution rules

Only sources whose `licence` allows it (MIT, Apache-2.0, or a dual licence of those). When code
is adapted:

1. Add a file header:
   ```
   // Adapted from <owner/repo> <path> @ <short-sha>.
   // Copyright (c) <year> <upstream copyright holder>. Licensed MIT OR Apache-2.0.
   ```
2. Add an `[[adaptation]]` entry to `upstream.toml`, and change the source's role to `adapted`.
3. Add a line to the "Source provenance" section of `NOTICE`.

Never copy: ArtCraft brand files or names (`docs/brand/` in storytold repos), Microsoft fonts
(Segoe *), Office artwork or logos, or images without a traceable licence (for example the
sample PNGs in laststudio/fluent-egui).
