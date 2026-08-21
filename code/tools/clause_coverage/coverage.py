"""`coverage` entry point: clause list + a DCGO recording corpus -> what fired.

This is the second half of the clause-coverage pipeline (see the package
README). Usage::

    PYTHONPATH=code python -m tools.clause_coverage.coverage \\
        --clauses clauses.json --recordings-dir D:\\dcgo-build\\vb-corpus3 --out coverage.json

Four measurement tiers, from most to least reliable given today's
recording schema (`docs/DCGO_RECORDING_SCHEMA.md`):

- **card-level**: which deck cards ever appeared on a battle area, measured
  from the union of `board_p0`/`board_p1` snapshots carried on every
  `action`/`selection` row. Reliable and always present.
- **prompt-level**: which `selection.prompt` kinds fired, and how often.
  Reliable and always present.
- **clause-level**: DCGO commit `8c4f98cb6` added `effect_activation` rows
  (`card_id` + printed `effect_description` + `executed`) -- this is now
  REAL measured coverage, not a placeholder. Each clause is reported as:
  `FIRED` (matched an activation with `executed: true`), `OFFERED_ONLY`
  (matched, but every match had `executed: false` -- offered and
  declined, not "never fired"), `NOT_FIRED` (no activation matched, and
  the clause is of an observable kind), or `UNOBSERVABLE` (the clause's
  KIND is a confirmed structural blind spot of the hook -- keyword
  clauses, and digivolve/Use-Req. cost-line clauses -- so absence of
  evidence is NOT evidence of non-firing; see
  `activation_match.is_blind_spot_kind`). A separate, permanently
  unidentifiable gap (25 DCGO cards using `SetIsBackgroundProcess(true)`
  bypass the hook regardless of clause kind) is called out in the report
  text rather than silently folded into any status, since which of a
  corpus's cards are among those 25 cannot be determined from the
  recordings alone.
  Matching itself (`activation_match.match_activation_row`) is fuzzy
  (normalized similarity ratio), not exact-string, because
  `effect_description` and this package's extracted clause text come from
  two independent pipelines that agree on content but not byte-for-byte
  -- see that module's docstring. Any activation that cannot be matched to
  ANY clause on its card is a genuine signal (a missing card, an
  unresolved `image-required` clause, or a real denominator gap) and is
  surfaced loudly in `clause_level.unmatched_activations`, never dropped.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

from tools.clause_coverage.activation_match import (
    is_blind_spot_kind,
    match_activation_row,
)


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
    activation_rows: list[dict] = []
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
            elif row_type == "effect_activation":
                activation_rows.append(row)
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
            "effect_activation_rows": len(activation_rows),
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
        "clause_level": _build_clause_level(clauses, activation_rows),
    }


def _effective_executed(row: dict) -> bool | None:
    """`executed` if present; a mandatory effect (`is_optional: false`)
    with `executed` absent is always `true` by the recorder's own gate
    (`UseOptional || !IsOptional` -- see `docs/DCGO_RECORDING_SCHEMA.md`).
    Otherwise unknown -- the row matched a clause but tells us nothing
    about whether it ran, so it must not count as FIRED or OFFERED_ONLY
    evidence either way."""
    executed = row.get("executed")
    if executed is not None:
        return executed
    if row.get("is_optional") is False:
        return True
    return None


def _build_clause_level(clauses: list[dict], activation_rows: list[dict]) -> dict:
    """Real, measured clause-level coverage from `effect_activation` rows.

    Every clause in `clauses` gets EXACTLY one of FIRED / OFFERED_ONLY /
    NOT_FIRED / UNOBSERVABLE -- the sum of `by_status.values()` always
    equals `len(clauses)` by construction (one status is appended per
    clause in the single loop below), never leaving a clause silently
    uncounted.
    """
    clauses_by_card: dict[str, list[dict]] = defaultdict(list)
    for c in clauses:
        clauses_by_card[c["card_id"]].append(c)

    # clause id -> list of effective-executed bools from every activation
    # row that matched it (None-executed matches are excluded -- see
    # `_effective_executed`).
    evidence: dict[str, list[bool]] = defaultdict(list)

    executed_true = 0
    executed_false = 0
    executed_unknown = 0
    missing_fields = 0

    # (card_id, effect_description) -> row occurrence count / example spans.
    unmatched_counts: Counter = Counter()
    unmatched_spans_by_key: dict[tuple[str, str], list[dict]] = {}

    for row in activation_rows:
        executed = row.get("executed")
        if executed is True:
            executed_true += 1
        elif executed is False:
            executed_false += 1
        else:
            executed_unknown += 1

        card_id = row.get("card_id")
        description = row.get("effect_description")
        if not card_id or not description:
            missing_fields += 1
            continue

        outcome = match_activation_row(description, clauses_by_card.get(card_id, []))

        effective = _effective_executed(row)
        if effective is not None:
            for clause_id in outcome.matched_clause_ids:
                evidence[clause_id].append(effective)

        if outcome.unmatched_spans:
            key = (card_id, description)
            unmatched_counts[key] += 1
            if key not in unmatched_spans_by_key:
                unmatched_spans_by_key[key] = [
                    {
                        "kind": span.kind,
                        "timings": span.timings,
                        "keyword": span.keyword,
                        "text": span.text,
                    }
                    for span in outcome.unmatched_spans
                ]

    clause_ids_by_status: dict[str, list[str]] = {
        "FIRED": [],
        "OFFERED_ONLY": [],
        "NOT_FIRED": [],
        "UNOBSERVABLE": [],
    }
    unobservable_breakdown: Counter = Counter()

    for c in clauses:
        execs = evidence.get(c["id"], [])
        if any(execs):
            status = "FIRED"
        elif execs:  # non-empty, every match was executed=False
            status = "OFFERED_ONLY"
        elif is_blind_spot_kind(c):
            status = "UNOBSERVABLE"
            unobservable_breakdown["keyword_kind" if c["kind"] == "keyword" else "cost_line_kind"] += 1
        else:
            status = "NOT_FIRED"
        clause_ids_by_status[status].append(c["id"])

    by_status = {status: len(ids) for status, ids in clause_ids_by_status.items()}

    unmatched_activations = [
        {
            "card_id": card_id,
            "effect_description": description,
            "occurrences": unmatched_counts[(card_id, description)],
            "unmatched_spans": unmatched_spans_by_key[(card_id, description)],
        }
        for (card_id, description) in sorted(
            unmatched_counts, key=lambda k: (-unmatched_counts[k], k[0], k[1])
        )
    ]

    return {
        "total_clauses": len(clauses),
        "by_status": by_status,
        "clause_ids_by_status": clause_ids_by_status,
        "unobservable_breakdown": dict(unobservable_breakdown),
        "activation_rows": {
            "total": len(activation_rows),
            "executed_true": executed_true,
            "executed_false": executed_false,
            "executed_unknown": executed_unknown,
            "missing_card_id_or_description": missing_fields,
        },
        "unmatched_activation_pair_count": len(unmatched_counts),
        "unmatched_activation_row_count": sum(unmatched_counts.values()),
        "unmatched_activations": unmatched_activations,
        "method": (
            "Each effect_activation row's effect_description is split with "
            "the same timing/keyword clause splitter used to build the "
            "denominator, then each resulting span is fuzzy-matched "
            "(normalized similarity ratio) against this card's clauses -- "
            "see tools.clause_coverage.activation_match for the full rule. "
            "FIRED requires at least one matched activation with "
            "executed=true; OFFERED_ONLY means every matched activation had "
            "executed=false (offered, then declined -- NOT 'never fired'); "
            "NOT_FIRED means no activation matched anywhere in this corpus "
            "AND the clause is of a kind this hook can observe."
        ),
        "known_limitations": (
            "UNOBSERVABLE clauses (keyword-kind, and Digivolve/DNA "
            "Digivolve/Use Req./other cost-line timing clauses) are a "
            "confirmed structural blind spot of the effect_activation hook "
            "-- it sits in the card-effect activation path and these are "
            "never dispatched through it, so absence of evidence there is "
            "NOT evidence of non-firing (never call it NOT_FIRED). Real "
            "matched evidence for one of these clauses, when this corpus "
            "happens to catch it, still wins and reports FIRED/"
            "OFFERED_ONLY -- the KIND-based rule is only the fallback when "
            "no such evidence exists. Separately, and NOT reflected in any "
            "per-clause status here: 25 DCGO cards register "
            "SetIsBackgroundProcess(true) and bypass this hook entirely "
            "regardless of clause kind; which of a given corpus's cards "
            "are among those 25 cannot be determined from recordings "
            "alone, so any NOT_FIRED clause below could in principle "
            "belong to one of them and simply be unobserved rather than "
            "genuinely never fired."
        ),
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
    by_status = clause_level["by_status"]
    total = clause_level["total_clauses"] or 1
    print(
        "  clause-level: "
        + " ".join(f"{status}={n}/{clause_level['total_clauses']} ({n / total:.1%})" for status, n in by_status.items()),
        file=file,
    )
    act = clause_level["activation_rows"]
    print(
        f"  effect_activation rows: {act['total']} "
        f"(executed=true: {act['executed_true']}, executed=false: {act['executed_false']})",
        file=file,
    )
    if clause_level["unmatched_activation_pair_count"]:
        print(
            f"  UNMATCHED ACTIVATIONS: {clause_level['unmatched_activation_pair_count']} distinct "
            f"(card_id, effect_description) pair(s), {clause_level['unmatched_activation_row_count']} row(s) "
            "-- possible denominator gap, see clause_level.unmatched_activations",
            file=file,
        )
        for entry in clause_level["unmatched_activations"]:
            print(
                f"    {entry['card_id']} x{entry['occurrences']}: {entry['effect_description']!r}",
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
