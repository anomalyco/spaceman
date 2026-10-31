#!/usr/bin/env bash
# Checks folder shape for tracked sources:
#   prefix family  x.rs or x/ plus 2+ x_*.rs siblings, or 3+ x_*.rs alone: error, "move into x/"
#   folder breadth > 40 non-test source files in one folder: warn; > 60: error
# Files directly in examples/, benches/ and bin/ are skipped (each is its own program). Test files
# (*_tests.rs, tests.rs, *.test.ts, tests/ ...), generated and vendored files are not counted. The
# prefix is the stem up to the first "_" (Rust) or "_"/"-" (TypeScript). Existing violations are
# listed in scripts/folder-shape-exceptions.txt ("<key> [ceiling]", key is "<dir>/<prefix>_*" or
# "<dir>/"; the ceiling is optional); ceilings only go down.
# Usage: scripts/folder-shape.sh [--warn-only] [--update-exceptions [--allow-raise]]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
exec python3 - "$@" <<'PY'
import os, re, subprocess, sys
from collections import defaultdict

WARN, HARD = 40, 60
EXC = "scripts/folder-shape-exceptions.txt"
SOURCE = re.compile(r"\.(rs|tsx?|swift)$")
SKIPPED = re.compile(r"(^|/)(node_modules|vendor|vendored|third_party|target|dist|build|\.build|out|generated|__generated__|gen|fixtures?|Pods|DerivedData)/|\.d\.ts$|\.(generated|gen|g|pb)\.(rs|tsx?|swift)$|_generated\.rs$")
TEST = re.compile(r"(^|/)(tests?|benches|e2e|__tests__)/|(^|/)[^/]*Tests/|(^|/)tests?\.rs$|_tests?\.rs$|\.(test|spec)\.tsx?$|Tests\.swift$")
PROGRAMS = re.compile(r"(^|/)(examples|benches|bin)$")
args = sys.argv[1:]
warn_only = "--warn-only" in args
update = "--update-exceptions" in args
raise_ok = "--allow-raise" in args
for a in args:
    if a not in ("--warn-only", "--update-exceptions", "--allow-raise"):
        sys.exit(f"unknown option: {a}")

files = [f for f in subprocess.check_output(["git", "ls-files"], text=True).split("\n") if f]
dirs = set()
for f in files:
    parts = f.split("/")[:-1]
    dirs.update("/".join(parts[:i + 1]) for i in range(len(parts)))
by_folder = defaultdict(list)
for f in files:
    if SOURCE.search(f) and not SKIPPED.search(f) and not TEST.search(f):
        by_folder[os.path.dirname(f) or "."].append(f)

found = {}  # key -> (count, hint)
for folder, paths in by_folder.items():
    if len(paths) > WARN:
        found[folder + "/"] = (len(paths), "group into capability subfolders")
    stems = []
    for p in paths:
        name = os.path.basename(p)
        if name.endswith(".rs"):
            if PROGRAMS.search(folder):
                continue
            stems.append(("rs", name[:-3], "_"))
        elif re.search(r"\.tsx?$", name):
            stems.append(("ts", re.sub(r"\.tsx?$", "", name), "_-"))
    members = defaultdict(int)
    for lang, stem, seps in stems:
        at = next((i for i, c in enumerate(stem) if c in seps), -1)
        if 0 < at < len(stem) - 1:
            members[(lang, stem[:at])] += 1
    for (lang, prefix), n in members.items():
        base = prefix if folder == "." else f"{folder}/{prefix}"
        anchored = base in dirs or any(l == lang and s == prefix for l, s, _ in stems)
        if n >= (2 if anchored else 3):
            found[f"{base}{'_' if lang == 'rs' else '-'}*"] = (n, f"move into {prefix}/")

def load():
    out = {}
    if os.path.exists(EXC):
        for line in open(EXC):
            line = line.split("#")[0].strip()
            if line:
                parts = line.split()
                key = parts[0] if parts[0].endswith("*") or parts[0].endswith("/") else parts[0] + "/"
                out[key] = int(parts[1]) if len(parts) > 1 else 10**9
    return out

exc = load()
if update:
    new = {}
    for k, (n, _) in found.items():
        if not (k.endswith("*") or n > HARD):
            continue
        if k not in exc:
            if raise_ok:
                new[k] = n
            else:
                print(f"not listing new violation {k} ({n}); use --allow-raise", file=sys.stderr)
        else:
            new[k] = n if (n <= exc[k] or raise_ok) else exc[k]
    with open(EXC, "w") as fh:
        fh.write("# <key> <ceiling>: existing folder-shape violations (see scripts/folder-shape.sh). Ceilings only go down.\n")
        fh.write("# Refresh with scripts/folder-shape.sh --update-exceptions.\n")
        for k in sorted(new):
            fh.write(f"{k} {new[k]}\n")
    print(f"wrote {EXC} ({len(new)} entries)")
    sys.exit(0)

errors = warns = 0
gh = os.environ.get("GITHUB_ACTIONS") == "true"
rows = []
for k, (n, hint) in sorted(found.items(), key=lambda kv: (-kv[1][0], kv[0])):
    fam = k.endswith("*")
    if fam or n > HARD:
        limit = exc.get(k)
        if limit is not None and n <= limit:
            warns += 1
            rows.append(f"{n:6d}  warn   {k} ({hint}; exception, ceiling {limit})")
        else:
            errors += 1
            rows.append(f"{n:6d}  ERROR  {k} ({hint}" + (f"; limit {limit}" if limit is not None and limit < 10**9 else "") + ")")
            if gh:
                print(f"::error file={k}::{n} files: {hint}")
    else:
        warns += 1
        rows.append(f"{n:6d}  warn   {k} ({hint}, soft limit {WARN})")
if rows:
    print("\n".join(rows))
families = sum(1 for k in found if k.endswith("*"))
print(f"folder shape: families: {families}, folders over {WARN}: {len(found) - families}, errors: {errors}, warnings: {warns}")
sys.exit(0 if warn_only or errors == 0 else 1)
PY
