"""Run every DCGO exam scenario sim-only, each against the deck book that holds its decks.

`dcgo-harness exam --scenario qa/dcgo-exams/ --decks <one book>` can only resolve the
`rest:` deck names of ONE book, and the per-set books reuse names for different lists
(`tm-quiet-red`, `quiet-opponent`, ...), so they cannot be merged into one either. A
single-book run therefore "fails" every scenario whose decks live in another book -- it
never lowers them, which measures nothing. This driver picks, per scenario, the books
whose deck names cover all of its `rest:` names (books in the scenario's own directory
first; the harness default `data/starter_decks.json` last) and passes the first one that
lowers and asserts clean.

Exit status is non-zero if any scenario fails under every candidate book, or has no
candidate book at all. Usage (from the repo root):

    python code/tools/exam_sim_all.py --harness target/release/dcgo-harness
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys

import yaml

DEFAULT_BOOK = "data/starter_decks.json"


def load_books(root: str) -> list[tuple[str, set[str]]]:
    books: list[tuple[str, set[str]]] = []
    for path in sorted(glob.glob(os.path.join(root, "**", "*.json"), recursive=True)):
        try:
            data = json.load(open(path, encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if isinstance(data, dict) and isinstance(data.get("decks"), list):
            names = {d.get("name") for d in data["decks"] if isinstance(d, dict)}
            books.append((path, names))
    starter = json.load(open(DEFAULT_BOOK, encoding="utf-8"))
    starter_names = {d.get("id") or d.get("name") for d in starter.get("starter_decks", [])}
    books.append((DEFAULT_BOOK, starter_names))
    return books


def rest_names(scenario: str) -> set[str]:
    spec = yaml.safe_load(open(scenario, encoding="utf-8"))
    return {seat.get("rest") for seat in spec.get("decks", {}).values()}


def candidates(scenario: str, books: list[tuple[str, set[str]]]) -> list[str]:
    needed = rest_names(scenario)
    here = os.path.dirname(scenario)
    found = [path for path, names in books if needed <= names]
    return sorted(found, key=lambda p: (p == DEFAULT_BOOK, os.path.dirname(p) != here, p))


def run_one(harness: str, scenario: str, book: str) -> tuple[bool, list[str]]:
    cmd = [harness, "exam", "--scenario", scenario, "--sim-only", "--cards-json", "data/cards.json"]
    if book != DEFAULT_BOOK:
        cmd += ["--decks", book]
    out = subprocess.run(cmd, capture_output=True, text=True)
    text = out.stdout + out.stderr
    ok = out.returncode == 0 and "/ failed 0" in text
    return ok, [line.strip() for line in text.splitlines() if "FAILED" in line]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--harness", default="target/release/dcgo-harness")
    parser.add_argument("--scenarios", default="qa/dcgo-exams")
    args = parser.parse_args(argv)

    books = load_books(args.scenarios)
    scenarios = sorted(glob.glob(os.path.join(args.scenarios, "**", "*.yaml"), recursive=True))
    passed, failed, no_book = 0, [], []
    for scenario in scenarios:
        books_for = candidates(scenario, books)
        if not books_for:
            no_book.append(scenario)
            continue
        last: list[str] = []
        for book in books_for:
            ok, last = run_one(args.harness, scenario, book)
            if ok:
                passed += 1
                break
        else:
            failed.append((scenario, books_for[0], last))

    for scenario, book, lines in failed:
        print(f"FAILED {scenario} (book {book})")
        for line in lines[:3]:
            print(f"    {line}")
    for scenario in no_book:
        print(f"NO BOOK {scenario}: no deck book holds all of its rest: decks")
    print(
        f"exam-sim: {len(scenarios)} scenarios / {passed} passed / "
        f"{len(failed)} failed / {len(no_book)} without a book"
    )
    print("mode sim-only (no oracle: this can only re-check what a previous oracle run confirmed)")
    return 1 if failed or no_book else 0


if __name__ == "__main__":
    sys.exit(main())
