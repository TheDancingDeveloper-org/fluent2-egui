#!/usr/bin/env bash
# Publish fluent2-tokens, fluent2-icons and fluent2-egui in dependency order.
# Idempotent: a crate whose version is already on crates.io is skipped, so a
# re-run resumes after a partial failure. Never publishes unless --publish is
# given; the default is a dry run (cargo publish --dry-run).
set -euo pipefail

publish=0
if [[ "${1:-}" == "--publish" ]]; then
  publish=1
elif [[ -n "${1:-}" && "${1}" != "--dry-run" ]]; then
  echo "usage: $0 [--dry-run | --publish]" >&2
  exit 2
fi

root=$(git rev-parse --show-toplevel)
cd "$root"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "refusing to publish: working tree is not clean" >&2
  git status --porcelain >&2
  exit 1
fi

tag=${GITHUB_REF_NAME:-$(git describe --tags --exact-match 2>/dev/null || true)}
version=$(cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="fluent2-egui"))')
if [[ "$tag" != "v$version" ]]; then
  echo "refusing to publish: tag '${tag:-<none>}' does not match workspace version v$version" >&2
  exit 1
fi

if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "cargo-deny is required for the licence check" >&2
  exit 1
fi
# Allowlist is deny.toml (MIT, Apache-2.0, and the permissive licences already in
# the tree, including the font licences OFL-1.1 and Ubuntu-font-1.0). A disallowed
# licence fails the check.
cargo deny check licenses

# The root LICENSE is the MIT text, byte-for-byte with LICENSE-MIT, so hosts that
# only read LICENSE see a real licence. LICENSE-APACHE stays beside it; Cargo.toml
# still says "MIT OR Apache-2.0". A placeholder or a third text fails the check.
if ! cmp -s LICENSE LICENSE-MIT; then
  echo "refusing to publish: LICENSE is not the MIT text in LICENSE-MIT" >&2
  exit 1
fi

crates=(fluent2-tokens fluent2-icons fluent2-egui)
for crate in "${crates[@]}"; do
  published=$(cargo search "$crate" --limit 1 2>/dev/null | python3 -c 'import re,sys; name=sys.argv[1]; line=sys.stdin.readline(); m=re.match(re.escape(name) + r"\s*=\s*\"([^\"]+)\"", line); print(m.group(1) if m else "")' "$crate" || true)
  if [[ "$published" == "$version" ]]; then
    echo "skip $crate $version: already on crates.io"
    continue
  fi
  echo "package $crate $version and check the archive ships licence notices"
  cargo package -p "$crate" --allow-dirty=false --list \
    | python3 -c 'import sys
need=("LICENSE","LICENSE-MIT","LICENSE-APACHE","NOTICE")
text=sys.stdin.read().splitlines()
missing=[n for n in need if not any(line.endswith("/"+n) or line==n for line in text)]
if missing:
    sys.stderr.write("crate archive missing: "+", ".join(missing)+"\n")
    sys.exit(1)
'
  if [[ "$publish" -eq 1 ]]; then
    cargo publish -p "$crate"
  else
    cargo publish -p "$crate" --dry-run
  fi
done
