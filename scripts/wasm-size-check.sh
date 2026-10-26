#!/usr/bin/env bash
# Compare the gallery wasm build's compressed size to ci/wasm-budget.txt.
# Usage: scripts/wasm-size-check.sh [dist-dir]
# dist-dir defaults to crates/gallery/dist. Exits 0 under budget, 1 over, 2 if no wasm.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="${1:-$root/crates/gallery/dist}"
budget_file="$root/ci/wasm-budget.txt"
wasm="$(find "$dist" -maxdepth 1 -name '*_bg.wasm' -print -quit || true)"
if [[ -z "$wasm" ]]; then
  echo "wasm-size-check: no *_bg.wasm under $dist" >&2
  exit 2
fi
raw="$(wc -c < "$wasm" | tr -d ' ')"
gzip_bytes="$(gzip -9 -c "$wasm" | wc -c | tr -d ' ')"
if command -v brotli >/dev/null 2>&1; then
  brotli_bytes="$(brotli -c -q 11 "$wasm" | wc -c | tr -d ' ')"
else
  brotli_bytes="n/a"
fi
# Budget lines are "name bytes", comments and blanks ignored. gzip is the gate.
gzip_budget="$(awk '$1 == "gzip" { print $2; exit }' "$budget_file")"
brotli_budget="$(awk '$1 == "brotli" { print $2; exit }' "$budget_file")"
if [[ -z "$gzip_budget" ]]; then
  echo "wasm-size-check: no gzip budget in $budget_file" >&2
  exit 2
fi
report="$dist/wasm-size.txt"
{
  echo "file $(basename "$wasm")"
  echo "raw $raw"
  echo "gzip $gzip_bytes (budget $gzip_budget)"
  echo "brotli $brotli_bytes (budget ${brotli_budget:-none})"
} | tee "$report"
fail=0
if (( gzip_bytes > gzip_budget )); then
  echo "wasm-size-check: gzip $gzip_bytes exceeds budget $gzip_budget" >&2
  fail=1
fi
if [[ "$brotli_bytes" != "n/a" && -n "$brotli_budget" ]] && (( brotli_bytes > brotli_budget )); then
  echo "wasm-size-check: brotli $brotli_bytes exceeds budget $brotli_budget" >&2
  fail=1
fi
exit "$fail"
