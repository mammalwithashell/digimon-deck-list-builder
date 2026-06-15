#!/usr/bin/env python
"""CLI tool to run YAML test scenarios for the Digimon TCG simulator.

Usage:
    python tools/run_scenario.py qa/scenarios/my-test.yaml
    python tools/run_scenario.py qa/scenarios/ --all
    python tools/run_scenario.py qa/scenarios/ --pattern "bt24-*"
"""

import argparse
import fnmatch
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from engine_py_legacy.engine.runners.scenario_runner import ScenarioRunner, ScenarioResult


def run_single(path: Path) -> ScenarioResult:
    """Run a single scenario file."""
    runner = ScenarioRunner.from_yaml(path)
    return runner.run()


def run_batch(directory: Path, pattern: str = "*.yaml") -> list[ScenarioResult]:
    """Run all matching scenario files in a directory."""
    results = []
    files = sorted(directory.glob("**/*.yaml")) + sorted(directory.glob("**/*.yml"))
    for f in files:
        if pattern != "*.yaml" and not fnmatch.fnmatch(f.name, pattern):
            continue
        print(f"Running: {f.name} ...", end=" ", flush=True)
        result = run_single(f)
        status = "PASS" if result.passed else "FAIL"
        print(status)
        results.append(result)
    return results


def print_results(results: list[ScenarioResult], verbose: bool = False):
    """Print summary of results."""
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)

    print(f"\n{'═' * 60}")
    print(f"Results: {passed} passed, {failed} failed, {len(results)} total")
    print(f"{'═' * 60}")

    if failed > 0 or verbose:
        for r in results:
            if not r.passed or verbose:
                print(f"\n{r.summary}")


def main():
    parser = argparse.ArgumentParser(description="Run YAML test scenarios")
    parser.add_argument("path", type=Path, help="Scenario file or directory")
    parser.add_argument("--all", action="store_true", help="Run all scenarios in directory")
    parser.add_argument("--pattern", type=str, default="*.yaml",
                        help="Glob pattern for scenario files (default: *.yaml)")
    parser.add_argument("-v", "--verbose", action="store_true",
                        help="Show details for passing scenarios too")
    args = parser.parse_args()

    if args.path.is_file():
        result = run_single(args.path)
        print(result.summary)
        sys.exit(0 if result.passed else 1)
    elif args.path.is_dir():
        results = run_batch(args.path, args.pattern)
        if not results:
            print(f"No scenario files found in {args.path}")
            sys.exit(1)
        print_results(results, args.verbose)
        failed = sum(1 for r in results if not r.passed)
        sys.exit(0 if failed == 0 else 1)
    else:
        print(f"Path not found: {args.path}")
        sys.exit(1)


if __name__ == "__main__":
    main()
