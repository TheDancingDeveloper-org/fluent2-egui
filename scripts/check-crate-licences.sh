#!/usr/bin/env bash
# Fail if a published crate's licence field is not MIT OR Apache-2.0, or if
# `cargo package --list` omits LICENSE-MIT, LICENSE-APACHE or NOTICE.
# Used by CI and by scripts/publish-crates.sh (the publish script also checks
# the files match the workspace root).
set -euo pipefail
cd "$(dirname "$0")/.."

crates=(fluent2-tokens fluent2-icons fluent2-egui)
for crate in "${crates[@]}"; do
  got=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; name=sys.argv[1]; print(next(p.get("license") or "" for p in json.load(sys.stdin)["packages"] if p["name"]==name))' "$crate")
  if [[ "$got" != "MIT OR Apache-2.0" ]]; then
    echo "licence check: $crate license field is '${got}', want MIT OR Apache-2.0" >&2
    exit 1
  fi
  if ! cmp -s LICENSE-MIT "crates/$crate/LICENSE-MIT" || ! cmp -s LICENSE-APACHE "crates/$crate/LICENSE-APACHE"; then
    echo "licence check: $crate licence files differ from the root copies" >&2
    exit 1
  fi
  if [[ ! -s "crates/$crate/NOTICE" ]]; then
    echo "licence check: $crate has no NOTICE" >&2
    exit 1
  fi
  echo "licence check: package list for $crate"
  cargo package -p "$crate" --allow-dirty --list | python3 -c 'import sys
crate=sys.argv[1]
need=["LICENSE-MIT","LICENSE-APACHE","NOTICE"]
if crate=="fluent2-icons":
    need.append("LICENSE-FLUENT-ICONS")
if crate=="fluent2-egui":
    need += ["fonts/LICENSE-INTER.txt","fonts/LICENSE-NOTO-EMOJI.txt","syntaxes/LICENSE-TYPESCRIPT-TMLANGUAGE.txt","syntaxes/LICENSE-TYPESCRIPT-SUBLIME.txt"]
text=sys.stdin.read().splitlines()
missing=[n for n in need if not any(line==n for line in text)]
if missing:
    sys.stderr.write("crate archive missing: "+", ".join(missing)+"\n")
    sys.exit(1)
' "$crate"
done

# Self-test: a nested path must not satisfy a crate-root requirement.
python3 - <<'EOF'
lines = ["src/foo/NOTICE", "LICENSE-MIT", "LICENSE-APACHE"]
need = ["NOTICE"]
missing = [n for n in need if not any(line == n for line in lines)]
if missing != ["NOTICE"]:
    raise SystemExit("self-test failed: nested NOTICE satisfied the root requirement")
if not any(line == "LICENSE-MIT" for line in lines):
    raise SystemExit("self-test failed: exact root path not accepted")
print("licence check: nested NOTICE self-test ok")
EOF

