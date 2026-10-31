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
  if [[ "$publish" -eq 1 ]]; then
    echo "refusing to publish: tag '${tag:-<none>}' does not match workspace version v$version" >&2
    exit 1
  fi
  echo "dry run: tag '${tag:-<none>}' does not match v$version (a publish would refuse)" >&2
fi

# Tag provenance: a publish only runs from a commit that is on main.
if [[ "$publish" -eq 1 ]]; then
  git fetch --quiet origin main
  if ! git merge-base --is-ancestor HEAD origin/main; then
    echo "refusing to publish: $(git rev-parse --short HEAD) is not on origin/main" >&2
    exit 1
  fi
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
# only read LICENSE see a real licence. LICENSE-APACHE stays beside it.
if ! cmp -s LICENSE LICENSE-MIT; then
  echo "refusing to publish: LICENSE is not the MIT text in LICENSE-MIT" >&2
  exit 1
fi

# Each published crate is dual-licensed MIT OR Apache-2.0 and ships its own
# licence texts plus a NOTICE. Root LICENSE-MIT and LICENSE-APACHE are the
# source of truth. scripts/check-crate-licences.sh fails when a crate's
# license field differs, the files differ from the root copies, or
# `cargo package --list` omits LICENSE-MIT, LICENSE-APACHE or NOTICE.
scripts/check-crate-licences.sh

crates=(fluent2-tokens fluent2-icons fluent2-egui)
for crate in "${crates[@]}"; do
  # Ask crates.io for this exact version: 200 = published, 404 = not yet.
  status=$(curl -s -o /dev/null -w '%{http_code}' \
    -A "fluent2-egui publish script (github.com/TheDancingDeveloper-org/fluent2-egui)" \
    "https://crates.io/api/v1/crates/$crate/$version" || true)
  case "$status" in
    200) published=$version ;;
    404) published= ;;
    *) echo "cannot tell whether $crate $version is published (HTTP ${status:-none})" >&2; exit 1 ;;
  esac
  if [[ "$published" == "$version" ]]; then
    echo "skip $crate $version: already on crates.io"
    continue
  fi
  echo "publish $crate $version"
  if [[ "$publish" -eq 1 ]]; then
    cargo publish -p "$crate"
  else
    cargo publish -p "$crate" --dry-run
  fi
done
