#!/usr/bin/env python
"""Batch QA runner: execute all scenarios for an archetype and generate a report.

Usage:
    python tools/run_qa_batch.py --archetype "medusamon" --scenarios qa/scenarios/medusamon/
    python tools/run_qa_batch.py -a "royal-knights" -s qa/scenarios/royal-knights/ -o qa/qa-reports/
"""

import argparse
import sys
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from digimon_gym.engine.runners.scenario_runner import ScenarioRunner, ScenarioResult


def run_batch(scenario_dir: Path) -> list[ScenarioResult]:
    """Run all YAML scenarios in a directory."""
    results = []
    files = sorted(list(scenario_dir.glob("*.yaml")) + list(scenario_dir.glob("*.yml")))

    if not files:
        print(f"No scenario files found in {scenario_dir}")
        return results

    for f in files:
        print(f"  {f.stem} ... ", end="", flush=True)
        try:
            result = ScenarioRunner.from_yaml(f).run()
            status = "PASS" if result.passed else "FAIL"
            print(status)
            results.append(result)
        except Exception as e:
            print(f"ERROR: {e}")
            results.append(ScenarioResult(
                name=f.stem, passed=False,
                assertions_passed=0, assertions_failed=0,
                assertion_details=[], action_log=[],
                error=str(e),
            ))

    return results


def generate_report(archetype: str, results: list[ScenarioResult]) -> str:
    """Generate a markdown QA report from batch results."""
    today = date.today().isoformat()
    passed = [r for r in results if r.passed]
    failed = [r for r in results if not r.passed]
    errors = [r for r in results if r.error]

    lines = [
        f"# Scenario QA Report — {archetype}",
        "",
        f"## Summary",
        f"- **Date**: {today}",
        f"- **Archetype**: {archetype}",
        f"- **Total Scenarios**: {len(results)}",
        f"- **Passed**: {len(passed)}",
        f"- **Failed**: {len(failed)}",
        f"- **Errors**: {len(errors)}",
        "",
    ]

    if failed:
        lines.append("## Failed Scenarios")
        lines.append("")
        lines.append("| Scenario | Assertions | Details |")
        lines.append("|----------|-----------|---------|")
        for r in failed:
            total_a = r.assertions_passed + r.assertions_failed
            detail = r.error or "; ".join(
                a.detail for a in r.assertion_details if not a.passed
            )[:100]
            lines.append(f"| {r.name} | {r.assertions_passed}/{total_a} | {detail} |")
        lines.append("")

    if passed:
        lines.append("## Passed Scenarios")
        lines.append("")
        lines.append("| Scenario | Assertions |")
        lines.append("|----------|-----------|")
        for r in passed:
            total_a = r.assertions_passed + r.assertions_failed
            lines.append(f"| {r.name} | {r.assertions_passed}/{total_a} |")
        lines.append("")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Run batch QA scenarios and generate report")
    parser.add_argument("--archetype", "-a", required=True, help="Archetype name")
    parser.add_argument("--scenarios", "-s", type=Path, required=True, help="Scenario directory")
    parser.add_argument("--output", "-o", type=Path, default=None,
                        help="Output directory for report (default: print to stdout)")
    args = parser.parse_args()

    print(f"Running scenarios for {args.archetype}...")
    results = run_batch(args.scenarios)

    if not results:
        sys.exit(1)

    report = generate_report(args.archetype, results)

    if args.output:
        args.output.mkdir(parents=True, exist_ok=True)
        today = date.today().isoformat()
        report_path = args.output / f"{today}-{args.archetype}-scenarios.md"
        report_path.write_text(report, encoding="utf-8")
        print(f"\nReport saved to: {report_path}")
    else:
        print(f"\n{report}")

    failed = sum(1 for r in results if not r.passed)
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
