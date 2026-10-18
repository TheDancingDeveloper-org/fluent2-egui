#!/usr/bin/env python3
"""Report how far each upstream in upstream.toml has moved since its pinned revision.

For every [[source]] it asks GitHub how many commits the branch is ahead of `rev` and which of
the `watch` paths changed, then prints a Markdown report. Every [[adaptation]] whose upstream
file changed is listed separately, because those may need the same fix here.

Usage:
    scripts/upstream-check.py                  # all sources, Markdown to stdout
    scripts/upstream-check.py egui wordcraft   # only these sources
    scripts/upstream-check.py --fail-on-adapted-drift

Auth: uses $GITHUB_TOKEN or $GH_TOKEN when set (60 unauthenticated requests per hour otherwise).
Exit status: 0 normally; 2 with --fail-on-adapted-drift when an adapted file changed upstream;
1 on a configuration or network error.
"""

from __future__ import annotations

import json
import os
import sys
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
API = "https://api.github.com"


def gh(path: str) -> dict:
    req = urllib.request.Request(API + path, headers={"Accept": "application/vnd.github+json",
                                                      "User-Agent": "fluent2-egui-upstream-check"})
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


def watched(filename: str, watch: list[str]) -> bool:
    return any(filename == w or filename.startswith(w.rstrip("/") + "/") for w in watch)


def changed_via_commits(src: dict, head: str) -> list[str]:
    """Watched files changed between `rev` and `head`, found per watched path.

    Used when the compare file list is truncated. For each watched path, page through the
    commits that touched it (newest first) until `rev` is reached, then read each commit's files.
    """
    out: set[str] = set()
    for w in src.get("watch", []):
        page, done = 1, False
        while not done:
            commits = gh(f"/repos/{src['repo']}/commits?sha={head}&path={w}&per_page=100&page={page}")
            for c in commits:
                if c["sha"] == src["rev"] or gh(
                        f"/repos/{src['repo']}/compare/{src['rev']}...{c['sha']}")["status"] in ("behind", "identical"):
                    done = True
                    break
                for f in gh(f"/repos/{src['repo']}/commits/{c['sha']}").get("files", []):
                    if watched(f["filename"], [w]):
                        out.add(f["filename"])
            if len(commits) < 100:
                done = True
            page += 1
    return sorted(out)


def check(src: dict) -> dict:
    cmp = gh(f"/repos/{src['repo']}/compare/{src['rev']}...{src['branch']}")
    files = [f["filename"] for f in cmp.get("files", [])]
    ahead = cmp.get("ahead_by", 0)
    commits = cmp.get("commits", [])
    # The compare API lists at most 300 files and 250 commits. Past either limit, take the branch
    # head directly and find the watched changes with per-path commit queries instead.
    truncated = len(files) >= 300 or len(commits) < ahead
    if truncated:
        head = gh(f"/repos/{src['repo']}/commits/{src['branch']}")["sha"]
        changed = changed_via_commits(src, head)
    else:
        head = commits[-1]["sha"] if commits else src["rev"]
        changed = sorted(f for f in files if watched(f, src.get("watch", [])))
    return {"ahead": ahead, "head": head, "changed": changed, "truncated": truncated}


def main(argv: list[str]) -> int:
    fail_on_adapted = "--fail-on-adapted-drift" in argv
    names = [a for a in argv if not a.startswith("--")]
    manifest = tomllib.loads((ROOT / "upstream.toml").read_text())
    sources = [s for s in manifest.get("source", []) if not names or s["name"] in names]
    if names and len(sources) != len(set(names)):
        known = {s["name"] for s in manifest.get("source", [])}
        print(f"unknown source(s): {sorted(set(names) - known)}", file=sys.stderr)
        return 1

    results = {}
    print("# Upstream drift report\n")
    print("| source | role | pinned | reviewed | commits since | watched paths changed |")
    print("|---|---|---|---|---:|---|")
    for s in sources:
        try:
            r = check(s)
        except (urllib.error.URLError, KeyError) as e:
            print(f"| {s['name']} | {s['role']} | `{s['rev'][:12]}` | {s['reviewed']} | error | {e} |")
            continue
        results[s["name"]] = r
        changed = f"{len(r['changed'])}{' (per-path query: compare truncated)' if r['truncated'] else ''}"
        print(f"| {s['name']} | {s['role']} | `{s['rev'][:12]}` | {s['reviewed']} | {r['ahead']} | {changed} |")

    for s in sources:
        r = results.get(s["name"])
        if not r or not r["changed"]:
            continue
        print(f"\n## {s['name']} ({s['repo']}) — {r['ahead']} commits, head `{r['head'][:12]}`\n")
        print(f"Compare: https://github.com/{s['repo']}/compare/{s['rev'][:12]}...{r['head'][:12]}\n")
        for f in r["changed"]:
            print(f"- `{f}`")

    drifted = []
    for a in manifest.get("adaptation", []):
        r = results.get(a["source"])
        if r and a["path"] in r["changed"]:
            drifted.append(a)
    if drifted:
        print("\n## Adapted files whose upstream changed\n")
        for a in drifted:
            print(f"- `{a['ours']}` ← {a['source']} `{a['path']}` (taken at `{a['rev'][:12]}`)")
    return 2 if (fail_on_adapted and drifted) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
