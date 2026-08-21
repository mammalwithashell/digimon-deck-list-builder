"""`coverage` entry point: clause list + a DCGO recording corpus -> what fired.

This is the second half of the clause-coverage pipeline (see the package
README). Usage::

    PYTHONPATH=code python -m tools.clause_coverage.coverage \\
        --clauses clauses.json --recordings-dir D:\\dcgo-build\\vb-corpus2 --out coverage.json

Three measurement tiers, from most to least reliable given today's
recording schema (`docs/DCGO_RECORDING_SCHEMA.md`):

- **card-level**: which deck cards ever appeared on a battle area, measured
  from the union of `board_p0`/`board_p1` snapshots carried on every
  `action`/`selection` row. Reliable and always present.
- **prompt-level**: which `selection.prompt` kinds fired, and how often.
  Reliable and always present.
- **clause-level**: recordings carry NO effect-activation events (no "card
  X's clause Y fired" row) -- a card's presence on the board is not
  evidence any specific clause of that card executed (e.g. a card can sit
  on the board all game without its `[On Deletion]` clause ever running).
  Every clause is therefore reported UNKNOWN, honestly, not "not covered".
  This is the CORRECT output today, and the UNKNOWN count is the report's
  headline number precisely because it's the thing most tempting to fudge.
  The cheap next step (not built here -- out of scope, see README): hook
  DCGO's existing player-log activation lines (e.g.
  "Activate_Optional_Effect_Execute: Siriusmon") into the recorder.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path


def _iter_recording_files(recordings_dir: Path) -> list[Path]:
    return sorted(Path(recordings_dir).glob("*.jsonl"))


def _load_rows(path: Path) -> tuple[list[dict], int]:
    rows: list[dict] = []
    malformed = 0
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                malformed += 1
    return rows, malformed


def build_coverage_report(clauses: list[dict], deck_card_ids: list[str], recordings_dir: Path) -> dict:
    files = _iter_recording_files(recordings_dir)

    board_cards: set[str] = set()
    prompt_counts: Counter = Counter()
    action_rows = 0
    selection_rows = 0
    action_rows_with_card_id = 0
    action_detail_rows = 0
    games = 0
    malformed_total = 0

    for path in files:
        rows, malformed = _load_rows(path)
        malformed_total += malformed
        saw_game_start = False
        for row in rows:
            row_type = row.get("type")
            if row_type == "game_start":
                saw_game_start = True
            elif row_type == "action":
                action_rows += 1
                if "card_id" in row:
                    action_rows_with_card_id += 1
                for key in ("board_p0", "board_p1"):
                    for cid in row.get(key) or []:
                        board_cards.add(cid)
            elif row_type == "action_detail":
                action_detail_rows += 1
            elif row_type == "selection":
                selection_rows += 1
                prompt = row.get("prompt")
                if prompt:
                    prompt_counts[prompt] += 1
                for key in ("board_p0", "board_p1"):
                    for cid in row.get(key) or []:
                        board_cards.add(cid)
        if saw_game_start:
            games += 1

    deck_set = set(deck_card_ids)
    on_board = sorted(board_cards & deck_set)
    never_on_board = sorted(deck_set - board_cards)
    unexpected_on_board = sorted(board_cards - deck_set)

    if action_rows_with_card_id == 0 and action_detail_rows == 0:
        schema_note = (
            "This corpus predates the 2026-08-20 action.card_id / "
            "action_detail diagnostic fields (docs/DCGO_RECORDING_SCHEMA.md) "
            "-- card-level presence below is derived entirely from "
            "board_p0/board_p1 snapshots, not from card_id."
        )
    else:
        schema_note = (
            f"{action_rows_with_card_id}/{action_rows} action rows carry "
            f"card_id and {action_detail_rows} action_detail rows are "
            "present -- a future revision could use these for a stronger "
            "played-vs-digivolved attribution; this report still uses only "
            "board_p0/board_p1 for card-level presence."
        )

    by_zone = Counter(c["zone"] for c in clauses)
    total_clauses = len(clauses)

    return {
        "corpus": {
            "dir": str(recordings_dir),
            "files": [p.name for p in files],
            "games": games,
            "action_rows": action_rows,
            "selection_rows": selection_rows,
            "malformed_lines_skipped": malformed_total,
            "action_rows_with_card_id": action_rows_with_card_id,
            "action_detail_rows": action_detail_rows,
            "schema_note": schema_note,
        },
        "denominator": {
            "total_clauses": total_clauses,
            "total_cards": len(deck_card_ids),
            "by_zone": dict(by_zone),
        },
        "card_level": {
            "deck_cards_total": len(deck_card_ids),
            "on_board_count": len(on_board),
            "on_board": on_board,
            "never_on_board": never_on_board,
            "unexpected_on_board": unexpected_on_board,
            "method": (
                "union of card_id across every action/selection row's "
                "board_p0/board_p1 snapshot -- the TOP card of each battle-"
                "area stack only. Does NOT distinguish played-from-hand vs "
                "digivolved-onto vs entered-by-effect, and 'never on "
                "board' is NOT evidence a card was never played: a Digi-Egg "
                "that hatched and immediately digivolved further is buried "
                "as digivolution material and invisible to a top-card-only "
                "snapshot, and a card whose own effect routes it to "
                "security/trash/hand instead of the battle area (e.g. an "
                "Option that places itself as a security card) can be "
                "played every game and still never appear here."
            ),
        },
        "prompt_level": {
            "distinct_prompts": len(prompt_counts),
            "total_selection_rows": selection_rows,
            "counts": dict(prompt_counts.most_common()),
        },
        "clause_level": {
            "status": "UNKNOWN_FOR_ALL",
            "reason": (
                "Recordings carry no effect-activation events. A card's "
                "presence on the board is not evidence any specific clause "
                "of that card fired (e.g. a card can sit on the board all "
                "game without its [On Deletion] clause ever running). "
                "Every clause is UNKNOWN, not 'not covered' -- this is the "
                "correct, honest output today."
            ),
            "by_status": {"UNKNOWN": total_clauses},
            "unknown_clause_ids": [c["id"] for c in clauses],
            "next_step": (
                "DCGO's player log already emits activation lines (e.g. "
                "'Activate_Optional_Effect_Execute: Siriusmon'); hooking "
                "that into the JSONL recorder is the cheap next step "
                "toward real clause-level coverage. Not built here -- YAGNI "
                "per this task's scope."
            ),
        },
    }


def _print_summary(report: dict, *, file) -> None:
    corpus = report["corpus"]
    denom = report["denominator"]
    print(
        f"Coverage report: {corpus['dir']} ({corpus['games']} games, {len(corpus['files'])} files)",
        file=file,
    )
    print(
        f"DENOMINATOR: {denom['total_clauses']} clauses total across "
        f"{denom['total_cards']} cards  (by zone: "
        + ", ".join(f"{z}={n}" for z, n in denom["by_zone"].items())
        + ")",
        file=file,
    )

    clause_level = report["clause_level"]
    unknown = clause_level["by_status"].get("UNKNOWN", 0)
    total = denom["total_clauses"] or 1
    print(
        f"  clause-level: UNKNOWN={unknown}/{denom['total_clauses']} ({unknown / total:.1%}) "
        "-- not directly measurable from these recordings today",
        file=file,
    )

    card_level = report["card_level"]
    print(
        f"CARD-LEVEL: {card_level['on_board_count']} / {card_level['deck_cards_total']} "
        "deck cards appeared on a battle area at least once",
        file=file,
    )
    if card_level["never_on_board"]:
        print("  never on board: " + ", ".join(card_level["never_on_board"]), file=file)

    prompt_level = report["prompt_level"]
    print(
        f"PROMPT-LEVEL: {prompt_level['distinct_prompts']} distinct selection prompt kinds, "
        f"{prompt_level['total_selection_rows']} selection rows total",
        file=file,
    )
    for prompt, count in prompt_level["counts"].items():
        print(f"    {prompt}: {count}", file=file)

    print(f"NOTE: {corpus['schema_note']}", file=file)


def run(clauses_doc: dict, recordings_dir: Path) -> dict:
    """Library entry point (also used directly by tests)."""
    return build_coverage_report(clauses_doc["clauses"], clauses_doc["cards"], recordings_dir)


def _use_utf8_streams() -> None:
    """Card text is full of full-width brackets etc.; Windows consoles
    default to cp1252 and choke on it. Best-effort, never fatal."""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")
        except (AttributeError, ValueError):
            pass


def main(argv: list[str] | None = None) -> None:
    _use_utf8_streams()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clauses", type=Path, required=True, help="Path to `extract`'s JSON output")
    parser.add_argument(
        "--recordings-dir", type=Path, required=True, help="Directory of DCGO .jsonl recordings (read-only)"
    )
    parser.add_argument("--out", type=Path, help="Write the coverage-report JSON here (default: stdout)")
    parser.add_argument("--quiet", action="store_true", help="Suppress the human-readable summary")
    args = parser.parse_args(argv)

    with open(args.clauses, encoding="utf-8") as f:
        clauses_doc = json.load(f)

    report = run(clauses_doc, args.recordings_dir)

    out_text = json.dumps(report, indent=2, ensure_ascii=False)
    if args.out:
        args.out.write_text(out_text, encoding="utf-8")
        summary_file = sys.stdout
    else:
        print(out_text)
        summary_file = sys.stderr

    if not args.quiet:
        _print_summary(report, file=summary_file)


if __name__ == "__main__":
    main()
