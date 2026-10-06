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
    lib = json.loads(Path(library_path).read_text(encoding="utf-8"))["archetypes"]
    out: list[list[str]] = []
    for entry in lib.values():
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
                out.append(ids)
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
