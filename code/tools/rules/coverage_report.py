"""Comprehensive Rules conformance coverage report.

Reads the generated denominator ``qa/rules/cr_obligations.json`` and prints, per
chapter, how many obligations are proven / not-testable / gap, plus the
"mechanics set" summary. The denominator (total obligation count) is always
printed. ``--check`` additionally verifies that every test named by a ``proven``
obligation exists as a ``#[test] fn`` under
``code/digimon-engine/tests/rules_conformance/*.rs`` and exits 1 if not.

    python -m tools.rules.coverage_report [--check]
    python code/tools/rules/coverage_report.py [--check]

Stdlib only (also runs as a plain script from ``scripts/verify``).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OBLIGATIONS = REPO_ROOT / "qa/rules/cr_obligations.json"
DEFAULT_TESTS = REPO_ROOT / "code/digimon-engine/tests/rules_conformance"

CHAPTER_NAMES = {
    1: "Game Overview",
    2: "Card Information",
    3: "Game Areas",
    4: "Basic Terminology",
    5: "Game Preparation",
    6: "Game Procedures",
    7: "Playing a Card",
    8: "Digivolution",
    9: "Using Cards",
    10: "Link",
    11: "Attacking",
    12: "Blocking",
    13: "Security Checks",
    14: "Battles",
    15: "Effect Rules",
    16: "Keyword Effects",
    17: "Rule Checks",
    18: "Other Information",
}

# `#[test]`, then any further attributes (`#[ignore = "..."]`, `#[should_panic(..)]`),
# then `fn name`.
_TEST_FN = re.compile(
    r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def collect_rust_test_fns(root: Path) -> set[str]:
    """Return ``{"<file stem>::<fn>"}`` for each ``#[test] fn`` in ``root/*.rs``."""
    root = Path(root)
    found: set[str] = set()
    if not root.is_dir():
        return found
    for path in sorted(root.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        for match in _TEST_FN.finditer(text):
            found.add(f"{path.stem}::{match.group(1)}")
    return found


def check_proven_tests_exist(obligations: dict, rust_test_fns: set[str]) -> list[str]:
    """Return the sorted ``module::fn`` names listed by proven obligations but absent."""
    missing: set[str] = set()
    for ob in obligations["obligations"].values():
        if ob["status"] != "proven":
            continue
        for test in ob.get("tests", []):
            if test not in rust_test_fns:
                missing.add(test)
    return sorted(missing)


def _pct(proven: int, gap: int) -> str:
    testable = proven + gap
    return "n/a" if testable == 0 else f"{100 * proven / testable:.1f}%"


def report(obligations: dict, rust_test_fns: set[str]) -> str:
    obs = obligations["obligations"]
    mech_set = set(obligations.get("mechanics_set", []))
    rows = {ch: dict(total=0, proven=0, nt=0, gap=0, blocking=0) for ch in CHAPTER_NAMES}
    mech = dict(total=0, proven=0, nt=0, blocking=0, gap=0)
    for ob in obs.values():
        row = rows.setdefault(
            ob["chapter"], dict(total=0, proven=0, nt=0, gap=0, blocking=0)
        )
        targets = [row]
        if mech_set.intersection(ob.get("mechanisms", [])):
            targets.append(mech)
        for t in targets:
            t["total"] += 1
            if ob["status"] == "proven":
                t["proven"] += 1
            elif ob["status"] == "not-testable":
                t["nt"] += 1
            else:
                t["gap"] += 1
                if ob.get("blocks_training"):
                    t["blocking"] += 1

    total = len(obs)
    lines = [
        f"Comprehensive Rules {obligations['source'].get('rules_version', '')} "
        f"conformance coverage: {total} obligations (denominator)".replace("  ", " "),
        "",
        "| Ch | Chapter | rules | proven | not-testable | gap (blocking) | gap | % proven of testable |",
        "|---|---|---|---|---|---|---|---|",
    ]
    sums = dict(total=0, proven=0, nt=0, gap=0, blocking=0)
    for ch in sorted(rows):
        r = rows[ch]
        for k in sums:
            sums[k] += r[k]
        name = CHAPTER_NAMES.get(ch, "")
        lines.append(
            f"| {ch} | {name} | {r['total']} | {r['proven']} | {r['nt']} | "
            f"{r['blocking']} | {r['gap']} | {_pct(r['proven'], r['gap'])} |"
        )
    lines.append(
        f"| Total | | {sums['total']} | {sums['proven']} | {sums['nt']} | "
        f"{sums['blocking']} | {sums['gap']} | {_pct(sums['proven'], sums['gap'])} |"
    )
    lines += [
        "",
        f"Mechanics set: proven {mech['proven']} / not-testable {mech['nt']} / "
        f"gap-blocking {mech['blocking']} / gap {mech['gap']} of {mech['total']} obligations",
        f"Rust conformance tests found: {len(rust_test_fns)}",
    ]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--obligations", type=Path, default=DEFAULT_OBLIGATIONS)
    parser.add_argument("--tests", type=Path, default=DEFAULT_TESTS)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit 1 if a proven obligation names a Rust test that does not exist",
    )
    args = parser.parse_args(argv)

    obligations = json.loads(args.obligations.read_text(encoding="utf-8"))
    fns = collect_rust_test_fns(args.tests)
    if args.check:
        missing = check_proven_tests_exist(obligations, fns)
        if missing:
            print(
                "proven obligations reference Rust tests that do not exist:",
                file=sys.stderr,
            )
            for name in missing:
                print(f"  {name}", file=sys.stderr)
            return 1
        print(f"rules coverage check: ok ({len(fns)} Rust tests found)")
        return 0
    sys.stdout.reconfigure(encoding="utf-8")
    print(report(obligations, fns), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
