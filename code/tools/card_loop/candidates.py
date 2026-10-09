"""Which meta archetype should the card loop run next?

`python -m tools.card_loop candidates [--top N] [--min-played K] [--json] [--out PATH]`

Ranks deck-library archetypes by how close each is to being fully adjudicated
against the DCGO oracle, weighted by how much it matters in the meta. It is a
read-only report over committed data; it writes `qa/card-loop/candidates.md`
and its JSON twin `candidates.json` and changes nothing else.

Measures (everything per archetype is over its POOL -- the union of its
recorded lists, which is exactly what `plan --archetype` resolves):

* **meta weight** -- sum over the archetype's recorded lists of
  ``(1 if the list top-cut else NON_TOP_CUT_WEIGHT) * 0.5 ** (age / HALF_LIFE_DAYS)``,
  where age is measured from the newest event date in the whole library. A
  top cut counts four times a plain entry; a list loses half its weight every
  180 days. Undated lists take the library's median recency.
* **adjudicated** -- the readiness definition (oracle-readiness design §4.7,
  card-loop design D4/D9): ``confirmed``; ``unreachable`` / ``unavailable``
  with a stated reason; ``diverged`` triaged ``dcgo_quirk`` with a citation.
  Anything else (unmeasured, untriaged or ``ours_wrong`` divergences, verdicts
  invalidated by text drift) is open work.
* **needs implementation** -- no YAML spec, a training-blocking ledger status
  (`NOT_READY_STATUSES`, the deny-list of the gate in `gauntlet.py`), or an id
  that is not in `cards.json` at all. "Missing" therefore means BOTH "no YAML"
  (this) and "no verdict" (open clauses / interactions), kept apart.
* **remaining units** -- cards needing implementation + open clauses + open
  gating interactions (distinct ids: a shared ruling counts once). A card
  without a DCGO script has its open clauses and interactions planned
  ``unavailable`` by the loop (design D3), so they cost no exam units; the
  card is listed as an oracle risk instead. One unit is one loop item, not
  one unit of cost -- implementing a card costs more than examining a clause,
  which is why the tiers below keep the two kinds of work apart.
* **closeness** -- ``1 / (1 + units / CLOSENESS_K)``: 1.0 with nothing left,
  0.5 at 250 remaining units (about one Three-Musketeers-sized campaign).
* **score** -- ``meta weight * closeness``. Monotonic: more meta weight never
  lowers it, more remaining work never raises it.
* **tiers** (after the 2026-08-22 prioritisation spec) -- A: at most
  `TIER_A_MAX_IMPL` cards to implement and none in the core (exam work);
  B: at most `TIER_B_MAX_IMPL` cards to implement, either at most
  `TIER_B_SMALL_IMPL` or with >= `TIER_B_MIN_TOP2_SHARE` of them in two sets
  (a coherent era); C: everything else (large or scattered gaps).
* **era spread** -- the sets of the cards that need implementation, and the
  share held by the two largest.
* **first list** -- remaining units, walking the loop's own greedy
  decklist-completion order (`workset.decklist_completion_order`), until the
  first of the archetype's lists has nothing outstanding.

Clause extraction runs with DCGO disabled (``DIGIMON_DCGO_ROOT=""``) exactly as
the readiness generator does, so the clause denominator is machine-independent;
DCGO is read only for per-card script presence. Binding is done ONCE over the
union of the scored pools (seconds, not the per-archetype rebinding that makes
`exam_index` slow), and every archetype reads the per-card result.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import sys
import time
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Iterable, Mapping, Sequence

from data_paths import CARD_OVERRIDES, CARDS_JSON, DATA_DIR, DECK_LIBRARY, REPO_ROOT
from tools.clause_coverage import archetype as archetype_mod
from tools.clause_coverage.exam_binding import (
    VERDICT_CLASSES,
    bind,
    bind_interactions,
    load_interaction_verdicts,
)

from .config import load_config
from .preflight import _card_effect_dir, dcgo_script_presence, resolve_dcgo_root, yaml_card_ids
from .workset import YAML_CARDS_DIR, _base_sha, decklist_completion_order, load_cards_json

REPORT_VERSION = 1

#: A plain (non-top-cut) entry counts this fraction of a top cut.
NON_TOP_CUT_WEIGHT = 0.25
#: A list's meta weight halves for every this many days of age.
HALF_LIFE_DAYS = 180
#: closeness = 1 / (1 + units / CLOSENESS_K).
CLOSENESS_K = 250

TIER_A_MAX_IMPL = 5
TIER_B_MAX_IMPL = 30
TIER_B_SMALL_IMPL = 10
TIER_B_MIN_TOP2_SHARE = 0.6

#: The training gate's deny-list (`gauntlet._NOT_READY_DSL_STATUSES`); a test
#: pins the two together without importing gauntlet (it needs the engine).
NOT_READY_STATUSES = frozenset({"PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"})
#: Failing verdicts of the legacy hand-written tracker, consulted only for a
#: card the DSL ledger does not list.
LEGACY_FAILING_STATUSES = NOT_READY_STATUSES | {"FAIL", "QA-FAIL"}

DEFAULT_TOP = 60
DEFAULT_MIN_PLAYED = 10
PLAN_COMMANDS = 10
DEFAULT_OUT = REPO_ROOT / "qa" / "card-loop" / "candidates.md"
COMMAND = "python -m tools.card_loop candidates"


@dataclass(frozen=True)
class DataPaths:
    """Every committed input; tests point these at synthetic fixtures."""

    library: Path = DECK_LIBRARY
    yaml_dir: Path = YAML_CARDS_DIR
    verdicts: Path = REPO_ROOT / "qa" / "qa-reports" / "exam-verdicts"
    scenarios: Path = REPO_ROOT / "qa" / "dcgo-exams"
    denominator: Path = DATA_DIR / "interaction_denominator.json"
    dsl_status: Path = REPO_ROOT / "qa" / "qa-reports" / "validated_cards_dsl.json"
    legacy_status: Path = REPO_ROOT / "qa" / "qa-reports" / "validated_cards.json"
    cards_json: Path = CARDS_JSON
    overrides: Path = CARD_OVERRIDES
    official: Path = DATA_DIR / "card_official.json"


# ---------------------------------------------------------------------------
# small pure measures
# ---------------------------------------------------------------------------


def _natural(s: str) -> list:
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", s)]


def set_of(card_id: str) -> str:
    """`BT12-034` -> `BT12`; `P-001` -> `P`."""
    return card_id.split("-", 1)[0]


def parse_event_date(raw) -> date | None:
    """The library's three date spellings: ISO `2026-07-03`, and US
    `M/D/YYYY` / `M/D/YY` (measured: the first field never exceeds 12, the
    second often does). Anything else is None, never a guess."""
    if not raw:
        return None
    s = str(raw).strip()
    try:
        m = re.fullmatch(r"(\d{4})-(\d{1,2})-(\d{1,2})", s)
        if m:
            return date(int(m[1]), int(m[2]), int(m[3]))
        m = re.fullmatch(r"(\d{1,2})/(\d{1,2})/(\d{2}|\d{4})", s)
        if m:
            year = int(m[3])
            return date(year + 2000 if year < 100 else year, int(m[1]), int(m[2]))
    except ValueError:
        return None
    return None


def recency(d: date, ref: date) -> float:
    return 0.5 ** (max(0, (ref - d).days) / HALF_LIFE_DAYS)


def _list_dates(library: Mapping) -> list[date]:
    out = []
    for entry in library.values():
        for dl in entry.get("decklists") or []:
            d = parse_event_date(dl.get("event_date"))
            if d is not None:
                out.append(d)
    return out


def reference_date(library: Mapping) -> date:
    """The newest event in the library (today if nothing is dated)."""
    dates = _list_dates(library)
    return max(dates) if dates else datetime.now(timezone.utc).date()


def undated_recency(library: Mapping, ref: date) -> float:
    """The median recency of the library's dated lists (1.0 if none)."""
    dates = _list_dates(library)
    return statistics.median(recency(d, ref) for d in dates) if dates else 1.0


def meta_weight(entry: Mapping, ref: date, undated_recency: float) -> dict:
    """See the module docstring. Counts come from `stats` when present."""
    weight, undated, newest = 0.0, 0, None
    for dl in entry.get("decklists") or []:
        d = parse_event_date(dl.get("event_date"))
        if d is None:
            undated += 1
            r = undated_recency
        else:
            r = recency(d, ref)
            newest = d if newest is None or d > newest else newest
        weight += r * (1.0 if dl.get("is_top_cut") else NON_TOP_CUT_WEIGHT)
    stats = entry.get("stats") or {}
    lists = entry.get("decklists") or []
    return {
        "weight": round(weight, 4),
        "times_played": int(stats.get("times_played", len(lists))),
        "top_cuts": int(stats.get("top_cut_count", sum(1 for dl in lists if dl.get("is_top_cut")))),
        "newest_event": newest.isoformat() if newest else None,
        "undated_lists": undated,
    }


def closeness(units: int) -> float:
    return 1.0 / (1.0 + units / CLOSENESS_K)


def era_spread(card_ids: Iterable[str]) -> dict:
    """Set distribution of `card_ids` and the share of the two largest sets."""
    ids = list(card_ids)
    counts: dict[str, int] = {}
    for c in ids:
        counts[set_of(c)] = counts.get(set_of(c), 0) + 1
    ordered = sorted(counts, key=lambda s: (-counts[s], _natural(s)))
    top2 = ordered[:2]
    return {
        "sets": {s: counts[s] for s in ordered},
        "n_sets": len(ordered),
        "top2": top2,
        "top2_share": (sum(counts[s] for s in top2) / len(ids)) if ids else None,
    }


def tier_of(impl_cards: int, core_impl: int, top2_share: float | None) -> str:
    if impl_cards <= TIER_A_MAX_IMPL and core_impl == 0:
        return "A"
    if impl_cards <= TIER_B_MAX_IMPL and (
        impl_cards <= TIER_B_SMALL_IMPL or (top2_share or 0.0) >= TIER_B_MIN_TOP2_SHARE
    ):
        return "B"
    return "C"


def adjudicated(row: Mapping) -> bool:
    """A clause or interaction verdict that ENDS the item (readiness §4.7)."""
    verdict = str(row.get("verdict") or "").lower()
    if verdict == "confirmed":
        return True
    if verdict in ("unreachable", "unavailable"):
        return bool(row.get("reason"))
    if verdict == "diverged":
        return row.get("triage") == "dcgo_quirk" and bool(row.get("citation"))
    return False


# ---------------------------------------------------------------------------
# per-card facts
# ---------------------------------------------------------------------------


@dataclass
class CardFacts:
    card_id: str
    known: bool                       # in cards.json
    has_yaml: bool
    status: str | None                # ledger status (DSL ledger first)
    status_blocks: bool
    dcgo_script: bool | None          # None: no usable DCGO checkout
    clause_counts: dict[str, int]
    clause_ids: list[str]
    adjudicated_clauses: set[str]
    interaction_ids: list[str]        # gating only
    adjudicated_interactions: set[str]
    in_denominator: bool = True
    #: diverged clauses that are not adjudicated (untriaged, `ours_wrong`, uncited)
    diverged_open: list[str] = field(default_factory=list)
    #: security slots with no printed source (no official bundle, no card text):
    #: open until someone reads the image, not until the oracle runs
    image_required: list[str] = field(default_factory=list)

    @property
    def needs_impl(self) -> bool:
        return not self.known or not self.has_yaml or self.status_blocks

    @property
    def vanilla(self) -> bool:
        return not self.clause_ids

    @property
    def _planned_unavailable(self) -> bool:
        return self.dcgo_script is False

    @property
    def open_clauses(self) -> list[str]:
        if self._planned_unavailable:
            return []
        return [c for c in self.clause_ids if c not in self.adjudicated_clauses]

    @property
    def open_interactions(self) -> set[str]:
        if self._planned_unavailable:
            return set()
        return {i for i in self.interaction_ids if i not in self.adjudicated_interactions}

    @property
    def planned_unavailable(self) -> dict[str, int]:
        if not self._planned_unavailable:
            return {"clauses": 0, "interactions": 0}
        return {
            "clauses": sum(1 for c in self.clause_ids if c not in self.adjudicated_clauses),
            "interactions": sum(1 for i in self.interaction_ids if i not in self.adjudicated_interactions),
        }

    @property
    def outstanding(self) -> bool:
        return self.needs_impl or bool(self.open_clauses) or bool(self.open_interactions)


def _tracker_cards(path: Path) -> dict:
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}
    return data.get("cards", {}) if isinstance(data, dict) else {}


def load_statuses(dsl_path: Path, legacy_path: Path) -> dict[str, tuple[str, bool]]:
    """`{card: (status, blocks_training)}`. The DSL ledger is the one the
    training gate reads and wins; the legacy tracker fills in only cards the
    DSL ledger does not list."""
    out: dict[str, tuple[str, bool]] = {}
    for cid, entry in _tracker_cards(legacy_path).items():
        status = str((entry or {}).get("status") or "").strip().upper()
        if status:
            out[cid] = (status, status in LEGACY_FAILING_STATUSES)
    for cid, entry in _tracker_cards(dsl_path).items():
        status = str((entry or {}).get("status") or "").strip().upper()
        if status:
            out[cid] = (status, status in NOT_READY_STATUSES)
    return out


@contextmanager
def _extraction_without_dcgo():
    """Readiness §4.7: extract clauses with DCGO disabled (machine-independent)."""
    old = os.environ.get("DIGIMON_DCGO_ROOT")
    os.environ["DIGIMON_DCGO_ROOT"] = ""
    try:
        yield
    finally:
        if old is None:
            os.environ.pop("DIGIMON_DCGO_ROOT", None)
        else:
            os.environ["DIGIMON_DCGO_ROOT"] = old


def _noop(_msg: str) -> None:
    pass


def collect_card_facts(card_ids: Iterable[str], paths: DataPaths, dcgo_root: Path | None,
                       *, progress=_noop) -> tuple[dict[str, CardFacts], dict]:
    """One bind + one interaction bind over `card_ids`, joined per card."""
    ids = sorted(set(card_ids), key=_natural)
    cards_index = load_cards_json(paths.cards_json)
    yaml_ids = yaml_card_ids(paths.yaml_dir)
    statuses = load_statuses(paths.dsl_status, paths.legacy_status)

    progress(f"binding clauses for {len(ids)} cards")
    with _extraction_without_dcgo():
        binding = bind(
            ids, paths.scenarios, paths.verdicts,
            source_desc=f"card_loop.candidates ({len(ids)} cards)",
            interaction_denominator=paths.denominator,
            cards_json_path=paths.cards_json, overrides_path=paths.overrides,
            official_json_path=paths.official,
        )
    progress("binding gating interactions")
    ibinding = bind_interactions(ids, paths.denominator, paths.verdicts)
    istore = load_interaction_verdicts(paths.verdicts)

    dcgo_available = dcgo_root is not None and _card_effect_dir(Path(dcgo_root)).is_dir()
    progress(f"DCGO script presence ({dcgo_root if dcgo_available else 'no DCGO checkout'})")
    presence = dcgo_script_presence(ids, dcgo_root) if dcgo_available else {}

    facts: dict[str, CardFacts] = {}
    image_required: list[str] = []
    for cid in ids:
        card = binding["cards"][cid]
        clause_rows = card["clauses"]
        image_required += [r["clause_id"] for r in clause_rows if r.get("source") == "image-required"]
        irows = ibinding["cards"][cid]
        adj_int = set()
        for r in irows["interactions"]:
            row = dict(r)
            if row["verdict"] == "diverged":
                raw = istore.get(row["interaction_id"]) or {}
                row["triage"], row["citation"] = raw.get("triage"), raw.get("citation")
            if adjudicated(row):
                adj_int.add(row["interaction_id"])
        status, blocks = statuses.get(cid, (None, False))
        facts[cid] = CardFacts(
            card_id=cid,
            known=cid in cards_index,
            has_yaml=cid in yaml_ids,
            status=status,
            status_blocks=blocks,
            dcgo_script=(presence.get(cid) is not None) if dcgo_available else None,
            clause_counts=dict(card["by_verdict"]),
            clause_ids=[r["clause_id"] for r in clause_rows],
            adjudicated_clauses={r["clause_id"] for r in clause_rows if adjudicated(r)},
            interaction_ids=[r["interaction_id"] for r in irows["interactions"]],
            adjudicated_interactions=adj_int,
            in_denominator=irows["in_denominator"],
            diverged_open=[r["clause_id"] for r in clause_rows
                           if r["verdict"] == "diverged" and not adjudicated(r)],
            image_required=[r["clause_id"] for r in clause_rows
                            if r.get("source") == "image-required" and not adjudicated(r)],
        )

    notes = {
        "dcgo_available": dcgo_available,
        "dcgo_root": str(dcgo_root) if dcgo_available else None,
        "cards_bound": len(ids),
        "clauses_bound": binding["denominator"]["total_clauses"],
        "clause_verdicts_by_class": binding["denominator"]["by_verdict"],
        "invalidated_clause_ids": list(binding["invalidated_clause_ids"]),
        "image_required_clause_ids": image_required,
        "interaction_verdicts_in_ledger": len(istore),
        "cards_missing_from_interaction_denominator": list(ibinding["cards_missing_from_denominator"]),
    }
    return facts, notes


# ---------------------------------------------------------------------------
# per-archetype score
# ---------------------------------------------------------------------------


def _clause_summary(cards: Sequence[str], facts: Mapping[str, CardFacts]) -> dict:
    out = {v: 0 for v in VERDICT_CLASSES}
    total = adj = open_ = image = 0
    for c in cards:
        f = facts[c]
        for v in VERDICT_CLASSES:
            out[v] += f.clause_counts.get(v, 0)
        total += len(f.clause_ids)
        adj += len(f.adjudicated_clauses)
        open_ += len(f.open_clauses)
        image += len(set(f.image_required) & set(f.open_clauses))
    out.update(total=total, adjudicated=adj, open=open_, planned_unavailable=total - adj - open_,
               open_image_required=image)
    return out


def first_list_units(lists: Sequence[Sequence[str]], facts: Mapping[str, CardFacts]) -> tuple[int, int]:
    """`(units until the first list is ready, lists ready now)`, walking the
    loop's greedy decklist-completion order over the outstanding cards."""
    outstanding = {c for cards in lists for c in cards if facts[c].outstanding}
    ready_now = sum(1 for cards in lists if not set(cards) & outstanding)
    if ready_now or not lists:
        return 0, ready_now
    seen: set[str] = set()
    total = 0
    for card, completes in decklist_completion_order(lists, outstanding):
        f = facts[card]
        new = f.open_interactions - seen
        seen |= new
        total += int(f.needs_impl) + len(f.open_clauses) + len(new)
        if completes:
            break
    return total, 0


def plan_command(name: str) -> str:
    return f'python -m tools.card_loop plan --archetype "{name}"'


def rationale(row: Mapping) -> str:
    m, u, era = row["meta"], row["units"], row["era"]
    meta = f"{m['top_cuts']} top cuts in {m['times_played']} lists (weight {m['weight']:.1f})"
    if u["implement"] == 0:
        impl = "nothing to implement"
    elif row["tier"] == "A":
        impl = f"{u['implement']} straggler card(s) to implement, none in the core"
    else:
        sets = "1 set" if era["n_sets"] == 1 else f"{era['n_sets']} sets"
        impl = (f"{u['implement']} card(s) to implement over {sets} "
                f"({era['top2_share']:.0%} in {'+'.join(era['top2'])})")
    image = row["clauses"]["pool"]["open_image_required"]
    exams = (f"{u['clauses']} clause + {u['interactions']} interaction exams"
             + (f" (incl. {image} image-required security slot(s))" if image else ""))
    first = ("a decklist is ready now" if u["lists_ready_now"]
             else f"first decklist ready after ~{u['first_list']} units")
    return f"{meta}; {impl}; {exams}; {first}"


def score_archetype(name: str, entry: Mapping, facts: Mapping[str, CardFacts], meta: dict,
                    *, core_fraction: float) -> dict:
    lists = archetype_mod.decklists(entry)
    pool = sorted(archetype_mod.list_frequency(lists), key=_natural)
    core_info = archetype_mod.core_of_lists(lists, core_fraction)
    core = sorted(core_info["cards"], key=_natural)

    needs_impl = [c for c in pool if facts[c].needs_impl]
    core_impl = [c for c in core if facts[c].needs_impl]

    gating: set[str] = set()
    adj: set[str] = set()
    open_: set[str] = set()
    for c in pool:
        f = facts[c]
        gating.update(f.interaction_ids)
        adj.update(f.adjudicated_interactions)
        open_ |= f.open_interactions
    adj &= gating
    open_ -= adj

    clauses_pool = _clause_summary(pool, facts)
    first, ready_now = first_list_units(lists, facts)
    units = {
        "implement": len(needs_impl),
        "clauses": clauses_pool["open"],
        "interactions": len(open_),
    }
    units["total"] = sum(units.values())
    units["first_list"] = first
    units["lists_ready_now"] = ready_now

    era = era_spread(needs_impl)
    dcgo_known = any(facts[c].dcgo_script is not None for c in pool)
    row = {
        "archetype": name,
        "tier": tier_of(len(needs_impl), len(core_impl), era["top2_share"]),
        "score": round(meta["weight"] * closeness(units["total"]), 4),
        "closeness": round(closeness(units["total"]), 4),
        "meta": meta,
        "lists": len(lists),
        "pool": len(pool),
        "core": len(core),
        "core_threshold": core_info["threshold"],
        "cards": {
            "with_yaml": sum(1 for c in pool if facts[c].has_yaml),
            "without_yaml": sum(1 for c in pool if not facts[c].has_yaml),
            "blocked_status": [c for c in pool if facts[c].status_blocks],
            "unknown_ids": [c for c in pool if not facts[c].known],
            "no_dcgo_script": ([c for c in pool if facts[c].dcgo_script is False and not facts[c].vanilla]
                               if dcgo_known else None),
            "no_dcgo_script_vanilla": (sum(1 for c in pool if facts[c].dcgo_script is False and facts[c].vanilla)
                                       if dcgo_known else None),
            "needs_impl": len(needs_impl),
            "core_needs_impl": len(core_impl),
            "needs_impl_ids": needs_impl,
        },
        "clauses": {"pool": clauses_pool, "core": _clause_summary(core, facts)},
        "interactions": {
            "total": len(gating),
            "adjudicated": len(adj),
            "open": len(open_),
            "planned_unavailable": len(gating) - len(adj) - len(open_),
        },
        "units": units,
        "era": era,
        "plan_command": plan_command(name),
    }
    row["rationale"] = rationale(row)
    return row


# ---------------------------------------------------------------------------
# the report
# ---------------------------------------------------------------------------


def build_report(paths: DataPaths, *, dcgo_root: Path | None, top: int, min_played: int,
                 core_fraction: float, command: str, base_sha: str | None,
                 progress=_noop) -> dict:
    library = archetype_mod.load_archetypes(paths.library)
    ref = reference_date(library)
    undated = undated_recency(library, ref)

    metas = {name: meta_weight(e, ref, undated) for name, e in library.items()}
    eligible = [n for n, e in library.items()
                if archetype_mod.decklists(e) and metas[n]["times_played"] >= min_played]
    eligible.sort(key=lambda n: (-metas[n]["weight"], n))
    chosen = eligible[:top]
    progress(f"{len(eligible)} of {len(library)} archetypes have >= {min_played} lists; "
             f"scoring the top {len(chosen)} by meta weight")

    union: set[str] = set()
    for n in chosen:
        union.update(archetype_mod.list_frequency(archetype_mod.decklists(library[n])))
    facts, notes = collect_card_facts(union, paths, dcgo_root, progress=progress)

    progress("scoring")
    rows = [score_archetype(n, library[n], facts, metas[n], core_fraction=core_fraction)
            for n in chosen]
    rows.sort(key=lambda r: (-r["score"], -r["meta"]["weight"], r["archetype"]))
    for i, r in enumerate(rows, 1):
        r["rank"] = i

    scored_cards = {c for n in chosen for c in archetype_mod.list_frequency(archetype_mod.decklists(library[n]))}
    open_interactions = set().union(*(facts[c].open_interactions for c in scored_cards)) if scored_cards else set()
    open_units = (sum(1 for c in scored_cards if facts[c].needs_impl)
                  + sum(len(facts[c].open_clauses) for c in scored_cards) + len(open_interactions))
    notes.update(
        unknown_ids=sorted((c for c in scored_cards if not facts[c].known), key=_natural),
        no_dcgo_script_with_text=(sorted((c for c in scored_cards if facts[c].dcgo_script is False
                                          and not facts[c].vanilla), key=_natural)
                                  if notes["dcgo_available"] else None),
        diverged_awaiting_triage=sorted(
            (cid for c in scored_cards for cid in facts[c].diverged_open), key=_natural),
        union_open_units={
            "implement": sum(1 for c in scored_cards if facts[c].needs_impl),
            "clauses": sum(len(facts[c].open_clauses) for c in scored_cards),
            "interactions": len(open_interactions),
            "total": open_units,
        },
    )

    return {
        "version": REPORT_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "base_sha": base_sha,
        "command": command,
        "parameters": {
            "top": top, "min_played": min_played, "core_fraction": core_fraction,
            "non_top_cut_weight": NON_TOP_CUT_WEIGHT, "half_life_days": HALF_LIFE_DAYS,
            "closeness_k": CLOSENESS_K,
            "tier_a_max_impl": TIER_A_MAX_IMPL, "tier_b_max_impl": TIER_B_MAX_IMPL,
            "tier_b_small_impl": TIER_B_SMALL_IMPL, "tier_b_min_top2_share": TIER_B_MIN_TOP2_SHARE,
            "not_ready_statuses": sorted(NOT_READY_STATUSES),
        },
        "reference_date": ref.isoformat(),
        "undated_recency": round(undated, 4),
        "library": {"archetypes": len(library), "eligible": len(eligible), "scored": len(chosen)},
        "notes": notes,
        "candidates": rows,
    }


# ---------------------------------------------------------------------------
# rendering
# ---------------------------------------------------------------------------

TIER_TITLES = {
    "A": "Tier A — implemented; the remaining work is adjudication",
    "B": "Tier B — a few cards to implement, or a gap concentrated in one era",
    "C": "Tier C — large or scattered implementation gaps",
}


def _cell(text) -> str:
    return str(text).replace("|", "\\|").replace("\n", " ")


def _ids(ids: Sequence[str] | None, limit: int = 12) -> str:
    if not ids:
        return "none"
    head = ", ".join(ids[:limit])
    return head + (f", ... (+{len(ids) - limit})" if len(ids) > limit else "")


def _set_counts(clause_ids: Sequence[str]) -> str:
    sets = era_spread([c.split("#", 1)[0] for c in clause_ids])["sets"]
    return ", ".join(f"{s} {n}" for s, n in sets.items()) or "none"


def _clause_cell(s: Mapping) -> str:
    return (f"{s['adjudicated']}/{s['total']} "
            f"({s['confirmed']}/{s['diverged']}/{s['unmeasured']})")


def _era_cell(era: Mapping) -> str:
    if not era["n_sets"]:
        return "—"
    sets = "1 set" if era["n_sets"] == 1 else f"{era['n_sets']} sets"
    return f"{sets}; {era['top2_share']:.0%} in {'+'.join(era['top2'])}"


def _tier_table(rows: Sequence[Mapping]) -> list[str]:
    head = ("| # | Archetype | Score | Meta weight (top cuts / lists) | Pool / core | "
            "No YAML | Blocked status | No DCGO script | Clauses, pool: adj/total (conf/div/unm) | "
            "Clauses, core | Gating interactions adj/total | Units left: impl + clauses + int. = total | "
            "First list ready after | Era of the implementation work |")
    out = [head, "|" + "---|" * 14]
    for r in rows:
        c, u, m = r["cards"], r["units"], r["meta"]
        no_dcgo = "?" if c["no_dcgo_script"] is None else len(c["no_dcgo_script"])
        first = "ready now" if u["lists_ready_now"] else u["first_list"]
        out.append(
            f"| {r['rank']} | {_cell(r['archetype'])} | {r['score']:.2f} | "
            f"{m['weight']:.1f} ({m['top_cuts']} / {m['times_played']}) | {r['pool']} / {r['core']} | "
            f"{c['without_yaml']} | {len(c['blocked_status'])} | {no_dcgo} | "
            f"{_clause_cell(r['clauses']['pool'])} | {_clause_cell(r['clauses']['core'])} | "
            f"{r['interactions']['adjudicated']}/{r['interactions']['total']} | "
            f"{u['implement']} + {u['clauses']} + {u['interactions']} = {u['total']} | "
            f"{first} | {_era_cell(r['era'])} |"
        )
    return out


def render_markdown(report: Mapping) -> str:
    p, lib, notes = report["parameters"], report["library"], report["notes"]
    rows = report["candidates"]
    sha = f" on `{report['base_sha'][:12]}`" if report.get("base_sha") else ""
    out = [
        "# Card-loop candidates: which archetype to run next",
        "",
        "<!-- GENERATED FILE (tools/card_loop/candidates.py): do not edit by hand; regenerate. -->",
        "",
        f"Generated by `{report['command']}` (from the repo root, with `PYTHONPATH=code`) at "
        f"{report['generated_at']}{sha}. Regenerate with the same command; the JSON twin "
        "(`candidates.json`, written beside this file) carries every number plus the per-archetype "
        "list of cards to implement.",
        "",
        f"Scored the top {lib['scored']} of the {lib['eligible']} deck-library archetypes with at least "
        f"{p['min_played']} recorded lists ({lib['archetypes']} archetypes in the library), chosen by "
        "meta weight. A prioritisation aid: every column is a measurement; the ranking is the formula "
        "below, not a verdict.",
        "",
        f"## Run next (top {min(PLAN_COMMANDS, len(rows))} by score)",
        "",
        "| # | Archetype | Tier | Score | Meta weight | Units left (impl + clauses + int.) | "
        "First list ready after |",
        "|---|---|---|---|---|---|---|",
    ]
    for r in rows[:PLAN_COMMANDS]:
        u = r["units"]
        first = "ready now" if u["lists_ready_now"] else f"{u['first_list']} units"
        out.append(f"| {r['rank']} | {_cell(r['archetype'])} | {r['tier']} | {r['score']:.2f} | "
                   f"{r['meta']['weight']:.1f} | {u['implement']} + {u['clauses']} + "
                   f"{u['interactions']} = {u['total']} | {first} |")
    out.append("")
    for r in rows[:PLAN_COMMANDS]:
        out += [f"{r['rank']}. **{r['archetype']}** (tier {r['tier']}, score {r['score']:.2f}): "
                f"{r['rationale']}.",
                "",
                "    ```",
                f"    {r['plan_command']}",
                "    ```",
                ""]

    out += [
        "## How to read this",
        "",
        f"- **Meta weight** = sum over the archetype's recorded lists of (1 for a top cut, "
        f"{p['non_top_cut_weight']} otherwise) × 0.5^(age / {p['half_life_days']} days). Age is "
        f"measured from the newest event in the library ({report['reference_date']}); undated "
        f"lists take the library's median recency ({report['undated_recency']}).",
        "- **Adjudicated** (readiness design §4.7, loop design D4/D9): `confirmed`; `unreachable` / "
        "`unavailable` with a stated reason; `diverged` triaged `dcgo_quirk` with a citation. "
        "Untriaged or `ours_wrong` divergences, verdicts invalidated by text drift and unmeasured "
        "items are open.",
        "- **Needs implementation**: no YAML spec, a training-blocking ledger status "
        f"({', '.join(p['not_ready_statuses'])}; the deny-list of the gate in `gauntlet.py`), or an "
        "id missing from `cards.json`. \"Missing\" is kept two ways: no YAML (implementation) and no "
        "verdict (open clauses and interactions).",
        "- **Units left** = cards needing implementation + open clauses + open gating interactions "
        "(distinct ids, so a shared ruling counts once). A card with no DCGO script has its open "
        "clauses and interactions planned `unavailable` by the loop, so they cost no exam units; "
        "it is listed as an oracle risk instead. A unit is one loop item, not one unit of cost.",
        f"- **Closeness** = 1 / (1 + units / {p['closeness_k']}): 1.0 with nothing left, 0.5 at "
        f"{p['closeness_k']} units (about one Three-Musketeers-sized campaign). **Score** = meta "
        "weight × closeness; more meta weight never lowers it, more work never raises it.",
        f"- **Tiers**: A = at most {p['tier_a_max_impl']} cards to implement and none in the core; "
        f"B = at most {p['tier_b_max_impl']}, and either at most {p['tier_b_small_impl']} or at "
        f"least {p['tier_b_min_top2_share']:.0%} of them in two sets; C = the rest.",
        f"- **Core** = cards in at least {p['core_fraction']:.0%} of the archetype's lists (the "
        "loop's `core_fraction`). **Era** = the sets of the cards that need implementation and the "
        "share of the two largest. **First list** = units spent, in the loop's own greedy "
        "decklist-completion order, before the first list has nothing outstanding.",
        "- Clause columns read `adjudicated/total (confirmed/diverged/unmeasured)`. Clause extraction "
        "runs with DCGO disabled, as the readiness generator does; DCGO is read only for script "
        "presence.",
        "",
    ]

    for tier in ("A", "B", "C"):
        tier_rows = [r for r in rows if r["tier"] == tier]
        out += [f"## {TIER_TITLES[tier]}", ""]
        if tier_rows:
            out += _tier_table(tier_rows)
        else:
            out.append("None in the scored set.")
        out.append("")

    union = notes["union_open_units"]
    share = union["interactions"] / union["total"] if union["total"] else 0.0
    out += [
        "## Data notes",
        "",
        f"- Across the union of the scored pools ({notes['cards_bound']} cards, "
        f"{notes['clauses_bound']} clauses) the open work is {union['implement']} cards to "
        f"implement + {union['clauses']} clauses + {union['interactions']} gating interactions = "
        f"{union['total']} units; interactions are {share:.0%} of it. Interaction verdicts in the "
        f"ledger: {notes['interaction_verdicts_in_ledger']}.",
        f"- Diverged clauses awaiting triage (open): {len(notes['diverged_awaiting_triage'])} "
        f"({_ids(notes['diverged_awaiting_triage'])}).",
        f"- Clause verdicts invalidated by text drift: {len(notes['invalidated_clause_ids'])} "
        f"({_ids(notes['invalidated_clause_ids'])}).",
        f"- Image-required clauses (a security slot with no official bundle and no card text; "
        f"extraction has DCGO disabled, as readiness will): {len(notes['image_required_clause_ids'])}, "
        f"by set {_set_counts(notes['image_required_clause_ids'])}. They stay open until the official "
        "mirror covers the card or someone reads the image.",
        f"- Decklist ids not in `cards.json` (counted as implementation work): "
        f"{_ids(notes['unknown_ids'])}.",
    ]
    if notes["dcgo_available"]:
        out.append(f"- Cards that print text but have no DCGO script (oracle blind spots; their exams "
                   f"are planned `unavailable`): {len(notes['no_dcgo_script_with_text'])} "
                   f"({_ids(notes['no_dcgo_script_with_text'], 30)}). Presence is by file name "
                   "(`<SET>/<Colour>/<ID_>.cs`).")
    else:
        out.append("- No DCGO checkout was found, so script presence is unknown (`?`) and no exam "
                   "work was discounted as planned `unavailable`.")
    if notes["cards_missing_from_interaction_denominator"]:
        out.append(f"- Cards absent from the interaction denominator: "
                   f"{_ids(notes['cards_missing_from_interaction_denominator'])}.")
    out.append("")
    return "\n".join(out)


def _summary(report: Mapping, limit: int = PLAN_COMMANDS) -> str:
    lines = [f"{'#':>3}  T  {'score':>6}  {'meta':>6}  {'units':>6}  archetype"]
    for r in report["candidates"][:limit]:
        lines.append(f"{r['rank']:>3}  {r['tier']}  {r['score']:6.2f}  {r['meta']['weight']:6.1f}  "
                     f"{r['units']['total']:6d}  {r['archetype']}")
    return "\n".join(lines)


def _echo(argv: Sequence[str]) -> str:
    parts = [a if re.fullmatch(r"[\w./:\\=-]+", a) else f'"{a}"' for a in argv]
    return " ".join([COMMAND, *parts])


def cli_candidates(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop candidates ...` -- exit 0, or 2 on bad input."""
    argv = sys.argv[1:] if argv is None else list(argv)
    parser = argparse.ArgumentParser(
        prog=COMMAND,
        description="Rank meta archetypes for the next card-loop run: meta weight x closeness "
                    "to full adjudication. Writes Markdown + a JSON twin.",
    )
    parser.add_argument("--top", type=int, default=DEFAULT_TOP,
                        help=f"score the N eligible archetypes with the most meta weight (default {DEFAULT_TOP})")
    parser.add_argument("--min-played", type=int, default=DEFAULT_MIN_PLAYED,
                        help=f"only archetypes with at least K recorded lists (default {DEFAULT_MIN_PLAYED})")
    parser.add_argument("--json", action="store_true", help="print the JSON report instead of the summary")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT,
                        help="Markdown path; the JSON is written beside it with a .json suffix "
                             "(default qa/card-loop/candidates.md)")
    parser.add_argument("--config", help="loop config TOML (for core_fraction)")
    d = parser.add_argument_group("data paths (defaults: committed repo data)")
    d.add_argument("--library", type=Path, default=None, help="deck_library.json")
    d.add_argument("--verdicts", type=Path, default=None, help="exam verdict store directory")
    d.add_argument("--denominator", type=Path, default=None, help="interaction_denominator.json")
    d.add_argument("--dcgo-root", type=Path, default=None,
                   help="DCGO checkout (default: the base repo's, rule 29)")
    args = parser.parse_args(argv)
    if args.top < 1 or args.min_played < 0:
        print("candidates: --top must be >= 1 and --min-played >= 0", file=sys.stderr)
        return 2

    started = time.monotonic()

    def progress(msg: str) -> None:
        print(f"candidates: [{time.monotonic() - started:6.1f}s] {msg}", file=sys.stderr, flush=True)

    defaults = DataPaths()
    paths = DataPaths(
        library=args.library or defaults.library,
        verdicts=args.verdicts or defaults.verdicts,
        denominator=args.denominator or defaults.denominator,
    )
    try:
        config = load_config(args.config)
        report = build_report(
            paths, dcgo_root=resolve_dcgo_root(args.dcgo_root), top=args.top,
            min_played=args.min_played, core_fraction=config.core_fraction,
            command=_echo(argv), base_sha=_base_sha(), progress=progress,
        )
    except (OSError, ValueError, LookupError) as e:
        print(f"candidates: {e}", file=sys.stderr)
        return 2

    md_path = Path(args.out)
    json_path = md_path.with_suffix(".json")
    md_path.parent.mkdir(parents=True, exist_ok=True)
    with open(md_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(render_markdown(report))
    with open(json_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    progress(f"wrote {md_path} and {json_path.name}")

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(_summary(report))
    return 0
