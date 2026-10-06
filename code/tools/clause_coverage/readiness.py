"""Oracle readiness: which cards RL training may draw.

A card is READY when every printed clause is adjudicated against the DCGO
oracle: confirmed; unreachable/unavailable with a stated reason; or diverged
and triaged as a cited DCGO quirk. Everything else blocks. Training reads only
the committed artifact this module writes (`data/oracle_readiness.json`).

Readiness today is clause-only. The ONE place a further adjudication input
joins it (card-loop group 7: every gating interaction of the card, from
`exam_binding.bind_interactions`) is `card_blockers` -- see its docstring.

Usage (from the repo root)::

    PYTHONPATH=code python -m tools.clause_coverage.readiness            # write
    PYTHONPATH=code python -m tools.clause_coverage.readiness --check    # CI drift
    PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit 25

Spec: `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.7.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import Counter
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterable, Iterator

_CODE = Path(__file__).resolve().parents[2]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))
_REPO = _CODE.parent

DEFAULT_LIBRARY = _REPO / "data" / "deck_library.json"
DEFAULT_OUT = _REPO / "data" / "oracle_readiness.json"
DEFAULT_TESTED = _REPO / "data" / "tested_cards.json"
DEFAULT_LEDGER = _REPO / "qa" / "qa-reports" / "validated_cards_dsl.json"
DEFAULT_VERDICTS = _REPO / "qa" / "qa-reports" / "exam-verdicts"
DEFAULT_SCENARIOS = _REPO / "qa" / "dcgo-exams"

ARTIFACT_VERSION = 1
READY = "ready"
NOT_READY = "not_ready"
# Mirror of gauntlet._NOT_READY_DSL_STATUSES (gauntlet needs the engine binding,
# which this stdlib tool must not import).
LEDGER_NOT_READY = frozenset({"PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"})
# Mirror of the engine's `deck_tools::is_card_id`, which `parse_tts` -- the
# gauntlet's decklist parser -- applies: anything else in a decklist is an
# export header or junk and is never looked up by the gate.
_CARD_ID = re.compile(r"^[A-Z]{1,3}[0-9]*-[0-9]+$")


def _has_text(value) -> bool:
    return bool((value or "").strip())


def clause_is_adjudicated(clause: dict) -> bool:
    """One printed clause cleared against the oracle (spec §4.7)."""
    verdict = clause.get("verdict")
    if verdict == "confirmed":
        return True
    if verdict in ("unreachable", "unavailable"):
        return _has_text(clause.get("reason"))
    if verdict == "diverged":
        return clause.get("triage") == "dcgo_quirk" and _has_text(clause.get("citation"))
    return False


def card_blockers(card_report: dict) -> list[str]:
    """Every adjudication unit of one card that keeps it out of training, sorted.

    THE extension seam of readiness. Today the units are the card's printed
    clauses (`exam_binding.bind()["cards"][cid]["clauses"]`), judged by
    `clause_is_adjudicated`. When the gating-interaction join lands (card-loop
    group 7, design D9), it joins HERE and nowhere else: `build_readiness`
    attaches the card's `bind_interactions()` rows to `card_report`, and this
    function appends the ids of the ones that are not adjudicated. The
    artifact's `blocking` list, the `ready` status, the training gate and
    `--plan` all consume this list unchanged.
    """
    return sorted(
        c["clause_id"] for c in card_report.get("clauses", []) if not clause_is_adjudicated(c)
    )


def card_status(card_report: dict) -> dict:
    blocking = card_blockers(card_report)
    return {
        "status": READY if not blocking else NOT_READY,
        "total_clauses": int(card_report.get("total_clauses", 0)),
        "by_verdict": dict(sorted((card_report.get("by_verdict") or {}).items())),
        "blocking": blocking,
    }


def library_decklists(library_path: Path) -> list[list[str]]:
    """Every decklist of the deck library as flat card ids, as the gauntlet reads them."""
    return [ids for _, ids in library_archetype_decklists(library_path)]


def library_archetype_decklists(library_path: Path) -> list[tuple[str, list[str]]]:
    """`library_decklists`, each paired with its (library) archetype name."""
    lib = json.loads(Path(library_path).read_text(encoding="utf-8"))["archetypes"]
    out: list[tuple[str, list[str]]] = []
    for archetype, entry in lib.items():
        for dl in entry.get("decklists", []):
            raw = dl.get("decklist")
            if raw:
                try:
                    ids = json.loads(raw)
                except (json.JSONDecodeError, TypeError):
                    continue
                if not isinstance(ids, list):
                    continue
                ids = [x for x in ids if isinstance(x, str) and _CARD_ID.match(x)]
            else:
                ids = [str(x) for x in dl.get("card_ids", [])]
            if ids:
                out.append((str(archetype), ids))
    return out


@contextmanager
def _dcgo_disabled() -> Iterator[None]:
    """Extract without consulting a local DCGO checkout, then restore the env.

    The artifact must be identical on a machine with the base-repo DCGO and in
    CI without it (spec §4.7 "Determinism"). Scoped, so an in-process caller's
    environment is left exactly as it was.
    """
    previous = os.environ.get("DIGIMON_DCGO_ROOT")
    os.environ["DIGIMON_DCGO_ROOT"] = ""
    try:
        yield
    finally:
        if previous is None:
            os.environ.pop("DIGIMON_DCGO_ROOT", None)
        else:
            os.environ["DIGIMON_DCGO_ROOT"] = previous


def build_readiness(
    card_ids: Iterable[str],
    *,
    scenarios_dir: Path,
    verdicts_dir: Path,
    bind_fn: Callable | None = None,
) -> dict:
    if bind_fn is None:
        from tools.clause_coverage.exam_binding import bind as bind_fn
    ids = sorted(set(card_ids))
    with _dcgo_disabled():
        report = bind_fn(ids, scenarios_dir, verdicts_dir, source_desc="oracle readiness")
    cards = {cid: card_status(report["cards"][cid]) for cid in sorted(report["cards"])}
    summary = Counter(s["status"] for s in cards.values())
    return {
        "version": ARTIFACT_VERSION,
        "cards": cards,
        "summary": {READY: summary.get(READY, 0), NOT_READY: summary.get(NOT_READY, 0)},
    }


def render(data: dict) -> str:
    return json.dumps(data, indent=2, sort_keys=True) + "\n"


def _is_ready(cards: dict[str, dict], card_id: str) -> bool:
    return cards.get(card_id, {}).get("status") == READY


def decklist_summary(decklists: list[list[str]], cards: dict[str, dict]) -> dict:
    """How many library decklists the ORACLE gate alone admits.

    The gauntlet additionally requires every card to be registered and free of
    a not-ready ledger verdict, so its admitted count is at most `oracle_ready`.
    """
    return {
        "total": len(decklists),
        "oracle_ready": sum(1 for deck in decklists if all(_is_ready(cards, c) for c in deck)),
    }


def rank_plan(
    decklists: list[list[str]],
    cards: dict[str, dict],
    eligible: Callable[[str], bool],
    limit: int,
    *,
    archetypes: list[str] | None = None,
) -> dict:
    """Which not-ready cards to examine next: greedy decklist completion (spec §4.8).

    Only decklists every card of which passes the OTHER gates (`eligible`:
    registered, no not-ready ledger verdict) are considered. The ordering is
    `tools.card_loop.workset.decklist_completion_order` -- the repo's single
    implementation of this ranking: the card completing the most otherwise-ready
    decklists first, tie-broken by how many considered decklists contain it, then
    by card id. `archetypes`, parallel to `decklists`, names each list's
    archetype so a pick reports the archetype(s) it unblocks.
    """
    from tools.card_loop.workset import decklist_completion_order

    labels = archetypes if archetypes is not None else [None] * len(decklists)
    considered: list[tuple[str | None, set[str]]] = []
    for label, deck in zip(labels, decklists):
        unique = set(deck)
        if all(eligible(c) for c in unique):
            considered.append((label, unique))
    contains: Counter = Counter()
    archetypes_of: dict[str, set[str]] = {}
    for label, deck in considered:
        for c in deck:
            contains[c] += 1
            if label is not None:
                archetypes_of.setdefault(c, set()).add(label)
    outstanding = {c for _, deck in considered for c in deck if not _is_ready(cards, c)}
    ready_now = sum(1 for _, deck in considered if not (deck & outstanding))
    order = decklist_completion_order([deck for _, deck in considered], outstanding)
    picks = [
        {
            "card_id": card_id,
            "decklists_completed": completed,
            "decklists_containing": contains[card_id],
            "blocking": cards.get(card_id, {}).get("blocking", [f"{card_id} (not in artifact)"]),
            "archetypes": sorted(archetypes_of.get(card_id, ())),
        }
        for card_id, completed in order[: max(limit, 0)]
    ]
    return {
        "decklists_ready_now": ready_now,
        "decklists_considered": len(considered),
        "picks": picks,
    }


def _eligibility(tested_path: Path, ledger_path: Path, cards: dict[str, dict]) -> Callable[[str], bool]:
    """The gauntlet's OTHER gates, approximated without the engine binding.

    Registered = in `tested_cards.json` (the registry snapshot) or vanilla (no
    printed clause); not-ready = a ledger verdict in `LEDGER_NOT_READY`.
    """
    tested = set(json.loads(tested_path.read_text(encoding="utf-8")).get("card_ids", []))
    ledger = json.loads(ledger_path.read_text(encoding="utf-8")).get("cards", {})
    not_ready = {
        cid for cid, e in ledger.items()
        if str(e.get("status") or "").strip().upper() in LEDGER_NOT_READY
    }

    def eligible(card_id: str) -> bool:
        playable = card_id in tested or cards.get(card_id, {}).get("total_clauses", 1) == 0
        return playable and card_id not in not_ready

    return eligible


_REGENERATE = "PYTHONPATH=code python -m tools.clause_coverage.readiness"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Oracle readiness artifact and exam plan")
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--verdicts", type=Path, default=DEFAULT_VERDICTS)
    parser.add_argument("--scenarios", type=Path, default=DEFAULT_SCENARIOS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--tested", type=Path, default=DEFAULT_TESTED)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--check", action="store_true", help="fail if --out is missing or stale")
    parser.add_argument("--plan", action="store_true", help="rank not-ready cards to examine next")
    parser.add_argument("--limit", type=int, default=25)
    parser.add_argument("--json", action="store_true", help="--plan output as JSON")
    args = parser.parse_args(argv)

    labelled = library_archetype_decklists(args.library)
    decklists = [ids for _, ids in labelled]

    if args.plan:
        if not args.out.exists():
            print(f"error: {args.out} is missing; run `{_REGENERATE}`", file=sys.stderr)
            return 1
        data = json.loads(args.out.read_text(encoding="utf-8"))
        plan = rank_plan(
            decklists,
            data["cards"],
            _eligibility(args.tested, args.ledger, data["cards"]),
            args.limit,
            archetypes=[name for name, _ in labelled],
        )
        if args.json:
            print(json.dumps(plan, indent=2, sort_keys=True))
        else:
            print(f"decklists considered (registered, ledger-ready): {plan['decklists_considered']}, "
                  f"oracle-ready now: {plan['decklists_ready_now']}")
            for i, p in enumerate(plan["picks"], 1):
                print(f"{i:>3}. {p['card_id']:<10} completes {p['decklists_completed']:>4}  "
                      f"in {p['decklists_containing']:>4} decklists  "
                      f"[{'; '.join(p['archetypes'])}]  "
                      f"outstanding: {', '.join(p['blocking'])}")
        return 0

    data = build_readiness(
        {c for deck in decklists for c in deck},
        scenarios_dir=args.scenarios,
        verdicts_dir=args.verdicts,
    )
    data["decklists"] = decklist_summary(decklists, data["cards"])
    text = render(data)
    if args.check:
        if not args.out.exists():
            print(f"error: {args.out} is missing; run `{_REGENERATE}`", file=sys.stderr)
            return 1
        if args.out.read_text(encoding="utf-8") != text:
            print(f"error: {args.out} is stale; run `{_REGENERATE}`", file=sys.stderr)
            return 1
        print(f"ok: {args.out} is up to date")
        return 0
    args.out.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {args.out}: {data['summary'][READY]} ready, "
          f"{data['summary'][NOT_READY]} not ready; "
          f"{data['decklists']['oracle_ready']}/{data['decklists']['total']} library decklists "
          "have every card oracle-ready")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
