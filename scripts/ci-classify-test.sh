#!/usr/bin/env bash
# Sample changed-file lists through the PR classifier in .github/workflows/ci.yml.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
workflow="$root/.github/workflows/ci.yml"
body="$(awk '
  /while IFS= read -r f; do/ { grab=1; next }
  /done </ { exit }
  grab { print }
' "$workflow")"
[[ -n "$body" ]] || { echo "classifier loop not found" >&2; exit 1; }

classify() {
  local input="$1"
  rust=false gallery=false screenshots=false fixtures=false full=false readme=false tokens=false icons=false deny=false
  packages=()
  add_package() {
    local pkg
    for pkg in "${packages[@]}"; do
      [[ "$pkg" == "$1" ]] && return 0
    done
    packages+=("$1")
  }
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    eval "$body"
  done <<<"$input"
  if [[ "$rust" == true && "$full" == false && "$fixtures" == false && ${#packages[@]} -eq 0 ]]; then
    full=true
  fi
  printf 'rust=%s gallery=%s screenshots=%s fixtures=%s full=%s readme=%s tokens=%s icons=%s deny=%s packages=%s\n' \
    "$rust" "$gallery" "$screenshots" "$fixtures" "$full" "$readme" "$tokens" "$icons" "$deny" "${packages[*]:-}"
}

expect() {
  local name="$1" files="$2" want="$3" got
  got="$(classify "$files")"
  if [[ "$got" != "$want" ]]; then
    echo "FAIL $name" >&2
    echo "  got:  $got" >&2
    echo "  want: $want" >&2
    exit 1
  fi
  echo "ok $name"
}

expect readme "README.md" \
  "rust=true gallery=false screenshots=false fixtures=false full=false readme=true tokens=false icons=false deny=false packages=fluent2-egui"
expect docs "docs/ARCHITECTURE.md" \
  "rust=true gallery=true screenshots=true fixtures=true full=true readme=true tokens=true icons=true deny=true packages="
expect generator "scripts/gen-icons.sh" \
  "rust=true gallery=false screenshots=false fixtures=false full=false readme=false tokens=true icons=false deny=false packages=xtask fluent2-tokens"
expect tokens "crates/fluent2-tokens/data/light.json" \
  "rust=true gallery=false screenshots=true fixtures=false full=false readme=false tokens=true icons=false deny=false packages=fluent2-tokens fluent2-egui gallery"
expect icons "crates/fluent2-icons/src/lib.rs" \
  "rust=true gallery=false screenshots=true fixtures=false full=false readme=false tokens=false icons=true deny=false packages=fluent2-icons fluent2-egui gallery"
expect empty "" \
  "rust=false gallery=false screenshots=false fixtures=false full=false readme=false tokens=false icons=false deny=false packages="

expect wasm_budget "ci/wasm-budget.txt" \
  "rust=true gallery=true screenshots=true fixtures=false full=false readme=false tokens=false icons=false deny=false packages=gallery"
expect wasm_doc "docs/wasm-size.md" \
  "rust=true gallery=true screenshots=true fixtures=false full=false readme=false tokens=false icons=false deny=false packages=gallery"
expect library "crates/fluent2-egui/src/lib.rs" \
  "rust=true gallery=false screenshots=true fixtures=true full=false readme=false tokens=false icons=false deny=false packages=fluent2-egui gallery"
expect fixture "fixtures/consumer/src/lib.rs" \
  "rust=true gallery=false screenshots=false fixtures=true full=false readme=false tokens=false icons=false deny=false packages="
expect lockfile "Cargo.lock" \
  "rust=true gallery=true screenshots=true fixtures=true full=true readme=true tokens=true icons=true deny=true packages="
expect xtask "xtask/src/main.rs" \
  "rust=true gallery=false screenshots=true fixtures=false full=false readme=false tokens=true icons=false deny=false packages=xtask fluent2-tokens fluent2-egui gallery"
echo "ci-classify-test: passed"
