"""Oracle readiness: which cards RL training may draw.

A card is READY when every printed clause AND every gating interaction of the
card is adjudicated against the DCGO oracle: confirmed; unreachable/unavailable
with a stated reason; or diverged and triaged as a cited DCGO quirk. Everything
else blocks -- an escalation is a human-queue entry, not a verdict, and blocks.
Training reads only the committed artifact this module writes
(`data/oracle_readiness.json`).

The two adjudication units join in `card_blockers` and nowhere else:

* printed clauses -- `exam_binding.bind()`;
* gating interactions (card-loop design D6/D9) -- `exam_binding.bind_interactions()`
  over the committed denominator `data/interaction_denominator.json`: every
  official Q&A ruling that prints the card plus every generated risk probe of
  a PROMOTED family. A shared ruling is one interaction, adjudicated once and
  counted for every card that prints it. A card the denominator never saw is a
  stale denominator, not "no interactions", and blocks (fail-closed).

Usage (from the repo root)::

    PYTHONPATH=code python -m tools.clause_coverage.readiness            # write
    PYTHONPATH=code python -m tools.clause_coverage.readiness --check    # CI drift
    PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit 25
    PYTHONPATH=code python -m tools.clause_coverage.readiness --clause-only --out /tmp/view.json
                                        # report view without the interaction gate (never --check)

Spec: `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.7;
the interaction join: `openspec/changes/add-card-authoring-loop/design.md` D9.
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
DEFAULT_DENOMINATOR = _REPO / "data" / "interaction_denominator.json"

# 2 (2026-10-06): gating interactions join readiness -- `blocking` may name
# interaction ids (`qa:<Q>`, `probe:<clause>:<family>[:neg]`) and the synthetic
# `<id>#denominator#missing`; per card `total_interactions` /
# `by_interaction_verdict`; top-level `gating` and `interactions`.
ARTIFACT_VERSION = 2
GATING_CLAUSES = "clauses"
GATING_INTERACTIONS = "interactions"
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


def interaction_is_adjudicated(interaction: dict) -> bool:
    """One gating interaction cleared (card-loop design D9): the clause rule,
    applied to a row of `exam_binding.bind_interactions()` -- `verdict`,
    `reason`, and for a diverged row the `triage` / `citation` twins the store
    carries flat. `escalated` is a human-queue state, never a verdict: an
    escalated interaction is `unmeasured` (no ending written) or `diverged` with
    an `undetermined` triage in the store, and blocks either way. A stored
    verdict whose text fingerprint drifted is already `unmeasured` here."""
    return clause_is_adjudicated(interaction)


# Printed-text fields of the merged cards.json/overrides record; any of them
# non-blank means the card prints something (mirrors the extractor's fallback).
_TEXT_FIELDS = (
    "effect_description_eng",
    "inherited_effect_description_eng",
    "security_effect_description_eng",
    "xros_req",
)


def vanilla_confirmed(official_entry: dict | None, card_record: dict) -> bool:
    """Two sources agree the card prints no effect, inherited or security text.

    The extractor yields zero clauses whenever the official-mirror entry has no
    text sections -- including a FAILED lookup, whose skeleton entry carries no
    `colors` (e.g. LM-057..LM-062, LM-065, which print effects). So zero clauses
    alone never proves "vanilla": the official entry must be a real lookup
    (non-blank `colors`) AND the merged cards.json/overrides record must carry
    no printed text either.
    """
    if not isinstance(official_entry, dict) or not _has_text(official_entry.get("colors")):
        return False
    if any(_has_text(card_record.get(f)) for f in _TEXT_FIELDS):
        return False
    dual = card_record.get("dual")
    if isinstance(dual, dict) and _has_text((dual.get("option") or {}).get("effect_text")):
        return False
    return True


def card_blockers(card_report: dict) -> list[str]:
    """Every adjudication unit of one card that keeps it out of training:
    clause blockers (sorted), then interaction blockers (sorted).

    THE extension seam of readiness; both units join here and nowhere else.

    1. The card's printed clauses (`exam_binding.bind()["cards"][cid]["clauses"]`),
       judged by `clause_is_adjudicated`, plus one synthetic
       `<id>#extraction#unresolved` unit for a card with zero clauses that
       `build_readiness` could not confirm as vanilla
       (`card_report["vanilla_confirmed"]`, fail-closed when absent).
    2. The card's gating interactions (card-loop design D9), which
       `build_readiness` attaches from `exam_binding.bind_interactions()` as
       `card_report["interactions"]` + `card_report["in_denominator"]`, judged
       by `interaction_is_adjudicated`. A card the denominator never saw
       (`in_denominator` False) gets the synthetic `<id>#denominator#missing`
       unit: the denominator lists every card it considered (`[]` = no
       interactions), so an absent card is a stale denominator, not a clear one.
       A report with no `in_denominator` key was built clause-only (the
       `--clause-only` report view) and contributes no interaction blockers.

    The artifact's `blocking` list, the `ready` status, the training gate and
    `--plan` all consume this list unchanged.
    """
    clauses = card_report.get("clauses", [])
    blocking = [c["clause_id"] for c in clauses if not clause_is_adjudicated(c)]
    if not clauses and card_report.get("vanilla_confirmed") is not True:
        blocking.append(f"{card_report.get('card_id', '?')}#extraction#unresolved")
    blocking.sort()
    joined = card_report.get("in_denominator")
    if joined is None:
        return blocking
    if joined is False:
        blocking.append(f"{card_report.get('card_id', '?')}#denominator#missing")
    blocking.extend(sorted(
        x["interaction_id"] for x in card_report.get("interactions", [])
        if not interaction_is_adjudicated(x)
    ))
    return blocking


def card_status(card_report: dict) -> dict:
    blocking = card_blockers(card_report)
    status = {
        "status": READY if not blocking else NOT_READY,
        "total_clauses": int(card_report.get("total_clauses", 0)),
        "by_verdict": dict(sorted((card_report.get("by_verdict") or {}).items())),
        "blocking": blocking,
    }
    if card_report.get("in_denominator") is not None:
        status["total_interactions"] = int(card_report.get("total_interactions", 0))
        status["by_interaction_verdict"] = dict(sorted(
            (card_report.get("by_interaction_verdict") or {}).items()))
    return status


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


def _default_vanilla_fn() -> Callable[[str], bool]:
    """`vanilla_confirmed` over the same card-data files the extractor reads."""
    from data_paths import CARD_OVERRIDES, CARDS_JSON
    from tools.clause_coverage.card_sources import (
        _merged_card_record,
        load_cards_index,
        load_official_index,
        load_overrides_index,
    )
    from tools.clause_coverage.extract import DEFAULT_OFFICIAL_JSON

    cards_index = load_cards_index(CARDS_JSON)
    overrides_index = load_overrides_index(CARD_OVERRIDES)
    official_index = load_official_index(DEFAULT_OFFICIAL_JSON)

    def is_vanilla(card_id: str) -> bool:
        record, _ = _merged_card_record(card_id, cards_index, overrides_index)
        return vanilla_confirmed(official_index.get(card_id), record)

    return is_vanilla


def build_readiness(
    card_ids: Iterable[str],
    *,
    scenarios_dir: Path,
    verdicts_dir: Path,
    bind_fn: Callable | None = None,
    vanilla_fn: Callable[[str], bool] | None = None,
    denominator_path: Path | str = DEFAULT_DENOMINATOR,
    interactions_fn: Callable | None = None,
    clause_only: bool = False,
) -> dict:
    """`vanilla_fn(card_id)` confirms a zero-clause card prints nothing
    (default: `vanilla_confirmed` over the extractor's card-data files); it is
    consulted only for cards whose extraction yields zero clauses.

    `interactions_fn(ids, denominator_path, verdicts_dir, gating_only=True)`
    (default `exam_binding.bind_interactions`) joins every gating interaction;
    a missing denominator file raises rather than reading as "no interactions".
    `clause_only=True` skips the join for a REPORT view (marked `gating:
    ["clauses"]`); the committed artifact is always the full gate."""
    if bind_fn is None:
        from tools.clause_coverage.exam_binding import bind as bind_fn
    ids = sorted(set(card_ids))
    with _dcgo_disabled():
        report = bind_fn(ids, scenarios_dir, verdicts_dir, source_desc="oracle readiness")
    reports = {cid: dict(report["cards"][cid]) for cid in sorted(report["cards"])}
    for cid, card_report in reports.items():
        if not card_report.get("clauses"):
            if vanilla_fn is None:
                vanilla_fn = _default_vanilla_fn()
            card_report["vanilla_confirmed"] = bool(vanilla_fn(cid))
    bound_i = None
    if not clause_only:
        if interactions_fn is None:
            from tools.clause_coverage.exam_binding import bind_interactions as interactions_fn
        bound_i = interactions_fn(ids, denominator_path, verdicts_dir, gating_only=True)
        for cid, card_report in reports.items():
            irep = bound_i["cards"].get(cid) or {}
            card_report["in_denominator"] = bool(irep.get("in_denominator"))
            card_report["interactions"] = list(irep.get("interactions") or [])
            card_report["total_interactions"] = int(irep.get("total_interactions") or 0)
            card_report["by_interaction_verdict"] = dict(irep.get("by_verdict") or {})
    cards = {cid: card_status(card_report) for cid, card_report in reports.items()}
    summary = Counter(s["status"] for s in cards.values())
    out = {
        "version": ARTIFACT_VERSION,
        "gating": [GATING_CLAUSES] if clause_only else [GATING_CLAUSES, GATING_INTERACTIONS],
        "cards": cards,
        "summary": {READY: summary.get(READY, 0), NOT_READY: summary.get(NOT_READY, 0)},
    }
    if bound_i is not None:
        den = bound_i["denominator"]
        out["interactions"] = {
            "gating_total": int(den["total_interactions"]),
            "by_verdict": dict(sorted(den["by_verdict"].items())),
        }
    return out


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
    parser.add_argument("--denominator", type=Path, default=DEFAULT_DENOMINATOR,
                        help="the gating interaction denominator (card-loop `interactions build`)")
    parser.add_argument("--clause-only", action="store_true",
                        help="REPORT view without the interaction gate; refused with --check")
    parser.add_argument("--check", action="store_true", help="fail if --out is missing or stale")
    parser.add_argument("--plan", action="store_true", help="rank not-ready cards to examine next")
    parser.add_argument("--limit", type=int, default=25)
    parser.add_argument("--json", action="store_true", help="--plan output as JSON")
    args = parser.parse_args(argv)

    if args.check and args.clause_only:
        print("error: --clause-only is a report view; the committed artifact is always the full "
              "gate, so --check never takes it", file=sys.stderr)
        return 2

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
        denominator_path=args.denominator,
        clause_only=args.clause_only,
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
    gate = "+".join(data["gating"])
    tail = ""
    if "interactions" in data:
        i = data["interactions"]
        tail = (f"; gating interactions: {i['gating_total']} "
                f"({i['by_verdict'].get('unmeasured', 0)} unmeasured)")
    print(f"wrote {args.out} [{gate}]: {data['summary'][READY]} ready, "
          f"{data['summary'][NOT_READY]} not ready; "
          f"{data['decklists']['oracle_ready']}/{data['decklists']['total']} library decklists "
          f"have every card oracle-ready{tail}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
