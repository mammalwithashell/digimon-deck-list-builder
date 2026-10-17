"""Comprehensive Rules conformance coverage report.

Reads the generated denominator ``qa/rules/cr_obligations.json`` and prints, per
chapter, how many obligations are proven / not-testable / gap, plus the
"mechanics set" summary. The denominator (total obligation count) is always
printed. ``--check`` additionally verifies that every test named by a ``proven``
obligation exists as a ``#[test] fn`` under
``code/digimon-engine/tests/rules_conformance/*.rs``, and that at least one listed
test is non-``#[ignore]``d and ``cite!``s the obligation's id; it exits 1 if not
(listed tests that do not cite the id only warn).

    python -m tools.rules.coverage_report [--check]
    python code/tools/rules/coverage_report.py [--check]

Stdlib only (also runs as a plain script from ``scripts/verify``).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
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
# then `fn name`. Group 1 is the attribute run between `#[test]` and `fn`, group 2 the name.
_TEST_FN = re.compile(
    r"#\[test\]((?:\s*#\[[^\]]*\])*)\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
_CITE_ID = re.compile(r'cite!\(\s*"([^"]+)"')


@dataclass(frozen=True)
class TestInfo:
    __test__ = False  # not a pytest class, despite the name

    ignored: bool
    cited_ids: frozenset[str]


def collect_rust_tests(root: Path) -> dict[str, TestInfo]:
    """Return ``{"<file stem>::<fn>": TestInfo}`` for each ``#[test] fn`` in ``root/*.rs``.

    ``ignored`` is true when an ``#[ignore`` attribute sits between ``#[test]``
    and ``fn``. ``cited_ids`` are the ids of the ``cite!("...")`` calls in the
    test body, taken as the text from the ``fn`` to the next ``#[test]`` (or EOF).
    """
    root = Path(root)
    found: dict[str, TestInfo] = {}
    if not root.is_dir():
        return found
    for path in sorted(root.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        matches = list(_TEST_FN.finditer(text))
        for i, match in enumerate(matches):
            end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
            body = text[match.end() : end]
            found[f"{path.stem}::{match.group(2)}"] = TestInfo(
                ignored="#[ignore" in match.group(1),
                cited_ids=frozenset(_CITE_ID.findall(body)),
            )
    return found


def collect_rust_test_fns(root: Path) -> set[str]:
    """Return ``{"<file stem>::<fn>"}`` for each ``#[test] fn`` in ``root/*.rs``."""
    return set(collect_rust_tests(root))


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


def check_proven_quality(
    obligations: dict, tests: dict[str, TestInfo]
) -> tuple[list[str], list[str]]:
    """Return ``(errors, warnings)`` for the ``proven`` obligations.

    A ``proven`` obligation is an error when it lists no tests, when none of its
    listed tests exists, or when no listed test is both non-ignored and cites the
    obligation's id. A listed test that exists but does not cite the id is a
    warning (secondary tests legitimately cover another half of the rule).
    """
    errors: list[str] = []
    warnings: list[str] = []
    for rule_id, ob in sorted(obligations["obligations"].items()):
        if ob["status"] != "proven":
            continue
        listed = ob.get("tests", [])
        if not listed:
            errors.append(f"{rule_id}: proven but lists no tests")
            continue
        existing = [t for t in listed if t in tests]
        if not existing:
            errors.append(f"{rule_id}: none of its listed tests exists ({', '.join(listed)})")
            continue
        for t in existing:
            if rule_id not in tests[t].cited_ids:
                warnings.append(f"{rule_id}: listed test {t} does not cite the rule")
        if not any(not tests[t].ignored and rule_id in tests[t].cited_ids for t in existing):
            errors.append(
                f"{rule_id}: proven, but no listed test is both non-ignored and cites "
                f"the rule ({', '.join(listed)})"
            )
    return errors, warnings


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
        help="exit 1 if a proven obligation is not backed by an existing, non-ignored, citing test",
    )
    args = parser.parse_args(argv)

    obligations = json.loads(args.obligations.read_text(encoding="utf-8"))
    tests = collect_rust_tests(args.tests)
    fns = set(tests)
    if args.check:
        failed = False
        missing = check_proven_tests_exist(obligations, fns)
        if missing:
            failed = True
            print(
                "proven obligations reference Rust tests that do not exist:",
                file=sys.stderr,
            )
            for name in missing:
                print(f"  {name}", file=sys.stderr)
        errors, warnings = check_proven_quality(obligations, tests)
        for line in warnings:
            print(f"WARNING: {line}", file=sys.stderr)
        if errors:
            failed = True
            print("proven obligations are not backed by a live, citing test:", file=sys.stderr)
            for line in errors:
                print(f"  {line}", file=sys.stderr)
        if failed:
            return 1
        print(f"rules coverage check: ok ({len(fns)} Rust tests found, {len(warnings)} warnings)")
        return 0
    sys.stdout.reconfigure(encoding="utf-8")
    print(report(obligations, fns), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
