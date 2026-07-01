#!/usr/bin/env python3
"""PendingSelection callback audit for cloneable-engine cutover task 6.1.

This is a ratchet gate, not the final 6.1 hard-zero gate. It classifies
remaining `PendingSelection` installs so cloneability work can distinguish:

  - callback_only: legacy executor sites with no nearby resume data.
  - resume_backed: coexistence sites that install both callback and resume data.
  - resume_stub: resume-backed sites whose legacy callback is inert/panic-only.

The default CLI fails only when callback-only counts exceed the checked-in
baseline. Ratchet the baseline down as sites migrate; once it reaches zero, the
legacy closure executor can be deleted and this gate can become a strict check.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping, Sequence


CALLBACK_ONLY = "callback_only"
RESUME_BACKED = "resume_backed"
RESUME_STUB = "resume_stub"

DEFAULT_ROOT = Path(__file__).resolve().parents[1] / "digimon-engine" / "src"

# Per-file callback_only baseline. RATCHET DOWN as callback-only selections are
# converted to data-backed resume frames; never increase without review.
DEFAULT_CALLBACK_ONLY_BASELINE: dict[str, int] = {
    "effect_context/selections.rs": 16,
}

PENDING_SELECTION_RE = re.compile(
    r"pending_selection\s*=\s*Some\s*\(\s*PendingSelection\s*\{"
)
FN_CONTEXT_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)"
)
NOOP_CALLBACK_RE = re.compile(
    r"callback\s*:\s*Box::new\s*\(\s*(?:move\s*)?\|[^|]*\|\s*\{\s*\}\s*\)",
    re.DOTALL,
)
RESUME_STUB_TEXT_RE = re.compile(
    r"(closure path must NOT run|must resolve through ResumeFrame|resume path drives this|data path)",
    re.IGNORECASE,
)
INDIRECT_RESUME_PARK_RE = re.compile(r"\bpark_[A-Za-z0-9_]*resume[A-Za-z0-9_]*\s*\(")


@dataclass(frozen=True)
class Finding:
    relpath: str
    line: int
    category: str
    function: str | None
    prompt: str | None

    def format(self) -> str:
        suffix = f" ({self.function})" if self.function else ""
        prompt = f" - {self.prompt}" if self.prompt else ""
        return f"{self.relpath}:{self.line}: {self.category}{suffix}{prompt}"


@dataclass(frozen=True)
class BaselineViolation:
    relpath: str
    actual: int
    baseline: int

    def format(self) -> str:
        return (
            f"{self.relpath}: callback_only = {self.actual} "
            f"(baseline {self.baseline})"
        )


def audit_paths(paths: Iterable[Path], *, root: Path) -> list[Finding]:
    """Classify PendingSelection callback installs in Rust source files."""
    root = root.resolve()
    findings: list[Finding] = []
    for path in sorted(Path(p) for p in paths):
        if path.suffix != ".rs" or not path.exists():
            continue
        source = path.read_text(encoding="utf-8")
        source = _blank_cfg_test_modules(source)
        relpath = _relative_path(path, root)
        findings.extend(_audit_source(source, relpath))
    return findings


def summarize(findings: Sequence[Finding]) -> dict[str, int]:
    counts = Counter(finding.category for finding in findings)
    return {
        CALLBACK_ONLY: counts[CALLBACK_ONLY],
        RESUME_BACKED: counts[RESUME_BACKED],
        RESUME_STUB: counts[RESUME_STUB],
    }


def compare_to_baseline(
    findings: Sequence[Finding], baseline: Mapping[str, int]
) -> list[BaselineViolation]:
    actual = Counter(
        finding.relpath
        for finding in findings
        if finding.category == CALLBACK_ONLY
    )
    relpaths = sorted(set(actual) | set(baseline))
    violations = []
    for relpath in relpaths:
        count = actual.get(relpath, 0)
        allowed = baseline.get(relpath, 0)
        if count > allowed:
            violations.append(BaselineViolation(relpath, count, allowed))
    return violations


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=DEFAULT_ROOT,
        help="Rust source root to audit.",
    )
    parser.add_argument(
        "--strict-zero",
        action="store_true",
        help="Fail if any callback-only selection remains.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable findings.",
    )
    parser.add_argument(
        "--show-findings",
        action="store_true",
        help="Print each finding in addition to the summary.",
    )
    args = parser.parse_args(argv)

    findings = audit_paths(args.root.rglob("*.rs"), root=args.root)
    summary = summarize(findings)
    violations = compare_to_baseline(findings, DEFAULT_CALLBACK_ONLY_BASELINE)
    strict_failed = args.strict_zero and summary[CALLBACK_ONLY] > 0

    if args.json:
        print(
            json.dumps(
                {
                    "summary": summary,
                    "violations": [v.__dict__ for v in violations],
                    "findings": [f.__dict__ for f in findings],
                },
                indent=2,
                sort_keys=True,
            )
        )
    else:
        print(
            "pending-selection callback audit: "
            f"{CALLBACK_ONLY}={summary[CALLBACK_ONLY]}, "
            f"{RESUME_BACKED}={summary[RESUME_BACKED]}, "
            f"{RESUME_STUB}={summary[RESUME_STUB]}"
        )
        if args.show_findings:
            for finding in findings:
                print(f"  {finding.format()}")
        if violations:
            print("\nNEW callback-only selections above baseline:")
            for violation in violations:
                print(f"  {violation.format()}")
        if strict_failed:
            print("\nStrict 6.1 gate failed: callback-only selections remain.")

    return 1 if violations or strict_failed else 0


def _audit_source(source: str, relpath: str) -> list[Finding]:
    lines = source.splitlines()
    findings = []
    for match in PENDING_SELECTION_RE.finditer(source):
        line = source.count("\n", 0, match.start()) + 1
        window = _window(lines, line, before=24, after=180)
        category = _classify_window(window)
        findings.append(
            Finding(
                relpath=relpath,
                line=line,
                category=category,
                function=_find_function_context(lines, line),
                prompt=_find_prompt(window),
            )
        )
    return findings


def _classify_window(window: str) -> str:
    has_resume = (
        "pending_selection_resume = Some" in window
        or bool(INDIRECT_RESUME_PARK_RE.search(window))
    )
    if has_resume and _is_resume_stub(window):
        return RESUME_STUB
    if has_resume:
        return RESUME_BACKED
    return CALLBACK_ONLY


def _is_resume_stub(window: str) -> bool:
    if RESUME_STUB_TEXT_RE.search(window):
        return True
    if "panic!" in window and "Resume" in window:
        return True
    return bool(NOOP_CALLBACK_RE.search(window))


def _find_function_context(lines: Sequence[str], line: int) -> str | None:
    for candidate in range(line - 1, -1, -1):
        match = FN_CONTEXT_RE.match(lines[candidate])
        if match:
            return match.group(1)
    return None


def _find_prompt(window: str) -> str | None:
    match = re.search(r'prompt\s*:\s*"([^"]+)"\.to_string\(\)', window)
    if match:
        return match.group(1)
    match = re.search(r'prompt\s*:\s*"([^"]+)"\.into\(\)', window)
    if match:
        return match.group(1)
    return None


def _window(lines: Sequence[str], line: int, *, before: int, after: int) -> str:
    index = max(0, line - 1)
    start = max(0, index - before)
    end = min(len(lines), index + after + 1)
    return "\n".join(lines[start:end])


def _relative_path(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root).as_posix()
    except ValueError:
        return path.name


def _blank_cfg_test_modules(source: str) -> str:
    """Blank `#[cfg(test)] mod tests { ... }` bodies while preserving lines."""
    lines = source.splitlines(keepends=True)
    output: list[str] = []
    pending_cfg_test = False
    skipping = False
    depth = 0

    for line in lines:
        stripped = line.strip()
        if skipping:
            output.append("\n")
            depth += _brace_delta(line)
            if depth <= 0:
                skipping = False
            continue

        if pending_cfg_test:
            if re.search(r"\bmod\s+tests\b", line) and "{" in line:
                output.append("\n")
                depth = _brace_delta(line)
                skipping = depth > 0
                pending_cfg_test = False
                continue
            if stripped.startswith("#["):
                output.append(line)
                continue
            pending_cfg_test = False

        if stripped.startswith("#[cfg(test)]"):
            pending_cfg_test = True
            output.append("\n")
            continue

        output.append(line)

    return "".join(output)


def _brace_delta(line: str) -> int:
    return line.count("{") - line.count("}")


if __name__ == "__main__":
    sys.exit(main())
