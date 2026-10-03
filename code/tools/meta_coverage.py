"""Meta coverage: how much of the current tournament meta can the engine play?

This answers the gate in front of RL training: *before spending compute on
agents, how much of the meta does the simulator faithfully run?* Coverage over
the whole ~4,400-card pool is the wrong denominator -- most of the pool never
sees play. The right one is the decks people actually register, weighted by how
often they register them.

Four inputs, joined per card:

- **The meta** -- tournament decklists from ``data/deck_library.json`` inside a
  date window (default: the newest format in ``data/meta_shares.json``). Each
  list is weighted so its archetype carries its real DigiLab field share
  (``--weighting digilab``, the default when the format has a snapshot) or an
  equal 1/n (``--weighting sample``). Archetypes the snapshot does not name share
  the remaining field mass in proportion to their list counts.
- **Implemented** -- a YAML spec under ``code/digimon-engine/cards/`` declares
  the card (``card: <ID>``). ``build.rs`` compiles exactly these into the
  engine's registry, so this is the set ``digimon-engine-cli pool`` prints,
  without needing a cargo build. (The per-card ``.json`` files beside the specs
  are printed card *data*, not implementations.)
- **Tested** -- implemented, referenced by at least one Rust test under
  ``code/digimon-engine/tests/``, and not flagged ``PARTIAL``/``BLOCKED`` in
  ``qa/qa-reports/validated_cards_dsl.json``.
- **DCGO-verified** -- tested, and every clause of the printed text has an exam
  verdict with none ``unmeasured`` or ``diverged``. The clause denominator comes
  from ``tools.clause_coverage.exam_binding.bind`` -- one denominator, one
  producer.

Outputs (``--out-dir``, default ``qa/qa-reports/meta-coverage/``):

- ``latest.md`` -- the human summary (committed).
- ``history.jsonl`` -- one headline row appended per run, skipped when nothing
  changed since the last row, so the figures roll (committed).
- ``latest.json`` -- the full report: headline, per-set, per-archetype, every
  meta card's status, the work queues (``implement_next`` is ordered to make
  the most field share fully playable soonest) and the evaluated launch plan.
  Regenerated in about a second, so it is git-ignored.
- ``dashboard.html`` (``--html``) -- a self-contained dashboard rendering the
  report, built from ``code/tools/meta_coverage_dashboard.html`` (git-ignored).
  ``--artifact-out PATH`` writes the same page without a document skeleton, for
  publishing as a claude.ai Artifact.

The launch plan (``qa/qa-reports/meta-coverage/launch_plan.json``) is
hand-maintained: each gate that names a ``metric`` (a dotted path into
``latest.json``) is re-scored on every run.

Standard library only, matching the rest of ``code/tools/``.

Usage (from the repo root)::

    PYTHONPATH=code python -m tools.meta_coverage                 # current format
    PYTHONPATH=code python -m tools.meta_coverage --trend --html  # + weekly backfill + dashboard
    PYTHONPATH=code python -m tools.meta_coverage --since 2026-07-03 --until 2026-09-03

Refresh the decklists first when the window is thin (``latest.md`` warns):
``python code/tools/meta_loader.py --scrape-dcg-nexus --dcg-nexus-since <date>``.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

DEFAULT_LIBRARY = REPO_ROOT / "data" / "deck_library.json"
DEFAULT_SHARES = REPO_ROOT / "data" / "meta_shares.json"
DEFAULT_CARDS_JSON = REPO_ROOT / "data" / "cards.json"
DEFAULT_TESTED_CARDS = REPO_ROOT / "data" / "tested_cards.json"
DEFAULT_ALIASES = REPO_ROOT / "data" / "archetype_aliases.json"
DEFAULT_CARDS_DIR = REPO_ROOT / "code" / "digimon-engine" / "cards"
DEFAULT_TESTS_DIR = REPO_ROOT / "code" / "digimon-engine" / "tests"
DEFAULT_TRACKER = REPO_ROOT / "qa" / "qa-reports" / "validated_cards_dsl.json"
DEFAULT_VERDICTS = REPO_ROOT / "qa" / "qa-reports" / "exam-verdicts"
DEFAULT_SCENARIOS = REPO_ROOT / "qa" / "dcgo-exams"
DEFAULT_OUT_DIR = REPO_ROOT / "qa" / "qa-reports" / "meta-coverage"
DEFAULT_PLAN = DEFAULT_OUT_DIR / "launch_plan.json"
DASHBOARD_TEMPLATE = Path(__file__).resolve().parent / "meta_coverage_dashboard.html"
DASHBOARD_PLACEHOLDER = "/*__META_COVERAGE_REPORT__*/null"

VERDICTS = ("confirmed", "diverged", "unreachable", "unavailable", "unmeasured")

#: Exclusive readiness stages, worst to best. ``verified`` implies tested
#: implies implemented; ``known_gap`` and ``untested`` are implemented but not
#: tested.
STAGES = ("missing", "known_gap", "untested", "tested", "verified")

KNOWN_GAP_STATUSES = frozenset({"PARTIAL", "BLOCKED"})

#: Ledger verdicts the training deck pool treats as ready -- mirrors
#: ``digimon_gym.agents.gauntlet._TRAINING_READY_DSL_STATUSES``.
TRAINING_READY_STATUSES = frozenset({"IMPLEMENTED", "AUDITED-OK"})

#: ``card_kind`` codes in ``cards.json``.
CARD_KINDS = {0: "Digimon", 1: "Tamer", 2: "Option", 3: "Digi-Egg", 4: "DUAL"}

#: A format window with fewer lists than this is flagged as thin in the report.
THIN_WINDOW_LISTS = 150


# --------------------------------------------------------------------------
# Small parsers
# --------------------------------------------------------------------------

_ISO_DATE = re.compile(r"^(\d{4})-(\d{2})-(\d{2})")
_US_DATE = re.compile(r"^(\d{1,2})/(\d{1,2})/(\d{4})")
_SHORT_ISO = re.compile(r"^(\d{2})-(\d{2})-(\d{2})$")
_CARD_LINE = re.compile(r"""^card:\s*["']?([A-Za-z0-9_-]+)["']?\s*(?:#.*)?$""")
_CARD_ID_TOKEN = re.compile(r"\b([A-Z]{1,3}\d{0,2}-\d{2,3})\b")
_CARD_ID_PARTS = re.compile(r"^([A-Z]+)(\d*)-(\d+)$")


def parse_event_date(raw) -> str | None:
    """Normalise the library's mixed event-date formats to ``YYYY-MM-DD``.

    DCG Nexus and DigiLab write ISO dates; DigimonMeta writes ``M/D/YYYY``; a
    few older imports wrote ``YY-MM-DD``. Anything else is ``None`` -- an
    undated list cannot be placed in a window, so it is excluded rather than
    guessed.
    """
    if not raw:
        return None
    text = str(raw).strip()
    if _ISO_DATE.match(text):
        return text[:10]
    m = _US_DATE.match(text)
    if m:
        return f"{m.group(3)}-{int(m.group(1)):02d}-{int(m.group(2)):02d}"
    m = _SHORT_ISO.match(text)
    if m:
        return f"20{m.group(1)}-{m.group(2)}-{m.group(3)}"
    return None


def normalize_card_id(raw: str, known: set[str] | dict | None = None) -> str:
    """Canonicalise a decklist card id against the ``cards.json`` keys.

    Strips alt-art suffixes (``BT26-081_P1``) and, when the id is unknown, tries
    the zero-padding variants scrapers disagree on (``ST10-4`` / ``ST10-04``,
    ``BT1-87`` / ``BT1-087``). An id that still matches nothing is returned as
    is and reported as ``in_cards_json: false`` -- usually a set we have not
    ingested yet.
    """
    cid = str(raw).strip().upper().split("_", 1)[0]
    if known is None or cid in known:
        return cid
    m = _CARD_ID_PARTS.match(cid)
    if not m:
        return cid
    prefix, set_no, num = m.groups()
    set_variants = {set_no, set_no.lstrip("0")} if set_no else {""}
    for s in set_variants:
        for width in (2, 3):
            candidate = f"{prefix}{s}-{int(num):0{width}d}"
            if candidate in known:
                return candidate
    return cid


def card_set(card_id: str) -> str:
    return card_id.split("-", 1)[0]


# --------------------------------------------------------------------------
# Inputs
# --------------------------------------------------------------------------

@dataclass
class Deck:
    archetype: str
    date: str
    source: str
    placement: str | None
    counts: Counter = field(default_factory=Counter)


def load_decks(library: dict, cards_index: dict) -> list[Deck]:
    """Every dated decklist in the library (any window)."""
    decks: list[Deck] = []
    for name, entry in sorted(library.get("archetypes", {}).items()):
        for dl in entry.get("decklists", []):
            when = parse_event_date(dl.get("event_date"))
            if not when:
                continue
            raw = dl.get("decklist")
            try:
                ids = json.loads(raw) if isinstance(raw, str) else list(raw or [])
            except json.JSONDecodeError:
                continue
            counts = Counter(normalize_card_id(c, cards_index) for c in ids if c)
            if not counts:
                continue
            decks.append(Deck(name, when, dl.get("source") or "?", dl.get("placement"), counts))
    return decks


def scan_implemented(cards_dir: Path) -> dict[str, str]:
    """``{card_id: spec_path}`` for every YAML spec the engine compiles."""
    found: dict[str, str] = {}
    paths = sorted(list(cards_dir.rglob("*.yaml")) + list(cards_dir.rglob("*.yml")))
    for path in paths:
        with path.open(encoding="utf-8") as fh:
            for line in fh:
                m = _CARD_LINE.match(line.rstrip("\n"))
                if m:
                    try:
                        rel = path.relative_to(REPO_ROOT).as_posix()
                    except ValueError:
                        rel = path.as_posix()
                    found.setdefault(m.group(1), rel)
                    break
    return found


def scan_test_refs(tests_dir: Path) -> set[str]:
    """Card ids mentioned anywhere in the engine's Rust test tree."""
    refs: set[str] = set()
    for path in tests_dir.rglob("*.rs"):
        refs.update(_CARD_ID_TOKEN.findall(path.read_text(encoding="utf-8", errors="ignore")))
    return refs


def load_json(path: Path, default=None):
    if not path.exists():
        return default
    return json.loads(path.read_text(encoding="utf-8"))


def load_tracker(path: Path) -> dict[str, dict]:
    data = load_json(path, {}) or {}
    return data.get("cards", {}) if isinstance(data, dict) else {}


def load_verdict_times(verdicts_dir: Path) -> dict[str, tuple[str, str]]:
    """``{clause_id: (verdict, recorded_at)}`` straight from the ledger files.

    ``recorded_at`` is when the verdict was last recorded, not first earned --
    good enough for a burn-up line, and labelled as such in the report.
    """
    out: dict[str, tuple[str, str]] = {}
    if not verdicts_dir.exists():
        return out
    for path in sorted(verdicts_dir.glob("*.json")):
        data = load_json(path, {}) or {}
        for clause_id, row in (data.get("clauses") or {}).items():
            out[clause_id] = (row.get("verdict", "unmeasured"), (row.get("recorded_at") or "")[:10])
    return out


def bind_exam(card_ids: list[str], scenarios_dir: Path, verdicts_dir: Path) -> dict[str, dict]:
    """Per-card clause denominator + verdict counts from ``clause_coverage``.

    Returns ``{card_id: {"total_clauses": N, "by_verdict": {...},
    "clause_ids": [...]}}``.
    """
    code_dir = str(REPO_ROOT / "code")
    if code_dir not in sys.path:
        sys.path.insert(0, code_dir)
    from tools.clause_coverage.exam_binding import bind

    binding = bind(sorted(set(card_ids)), scenarios_dir, verdicts_dir,
                   source_desc="meta_coverage")
    out: dict[str, dict] = {}
    for cid, entry in binding.get("cards", {}).items():
        out[cid] = {
            "total_clauses": entry.get("total_clauses", 0),
            "by_verdict": {v: entry.get("by_verdict", {}).get(v, 0) for v in VERDICTS},
            "clause_ids": [c.get("clause_id") for c in entry.get("clauses", []) if c.get("clause_id")],
        }
    return out


# --------------------------------------------------------------------------
# Weighting
# --------------------------------------------------------------------------

def deck_weights(decks: list[Deck], share_spec: dict | None) -> tuple[list[float], dict]:
    """Weight each list so the window reproduces the field, summing to 1.

    With no snapshot every list weighs 1/n. With a DigiLab snapshot, each named
    deck (or family) carries its printed share spread evenly over its lists; the
    unnamed remainder of the field is spread over every other archetype in
    proportion to its list count. A named deck with no lists in the window
    cannot contribute card composition -- it is reported as unrepresented and
    the weights are renormalised over what is represented.
    """
    n = len(decks)
    if n == 0:
        return [], {"mode": "empty"}
    if not share_spec:
        return [1.0 / n] * n, {"mode": "sample"}

    by_arch = Counter(d.archetype for d in decks)
    entry_of: dict[str, str] = {}
    rows = []
    for entry in share_spec.get("archetypes", []):
        lists = sum(by_arch.get(a, 0) for a in entry.get("library_archetypes", []))
        rows.append({
            "name": entry["name"],
            "share_pct": entry["share_pct"],
            "library_archetypes": entry.get("library_archetypes", []),
            "lists": lists,
        })
        for a in entry.get("library_archetypes", []):
            entry_of[a] = entry["name"]
    share_of = {r["name"]: r["share_pct"] / 100.0 for r in rows}
    lists_of = {r["name"]: r["lists"] for r in rows}
    named_mass = sum(share_of.values())
    other_mass = max(0.0, 1.0 - named_mass)
    other_lists = sum(c for a, c in by_arch.items() if a not in entry_of)

    raw = []
    for d in decks:
        name = entry_of.get(d.archetype)
        if name is not None:
            raw.append(share_of[name] / lists_of[name])
        else:
            raw.append(other_mass / other_lists if other_lists else 0.0)
    total = sum(raw)
    if total <= 0:
        return [1.0 / n] * n, {"mode": "sample", "fallback": "snapshot matched no lists"}
    weights = [w / total for w in raw]
    unrepresented = [r for r in rows if r["lists"] == 0]
    return weights, {
        "mode": "digilab",
        "named_share_pct": round(named_mass * 100, 2),
        "other_share_pct": round(other_mass * 100, 2),
        "represented_share_pct": round(total * 100, 2),
        "rows": rows,
        "unrepresented": [r["name"] for r in unrepresented],
    }


# --------------------------------------------------------------------------
# Status
# --------------------------------------------------------------------------

def is_dcgo_verified(exam: dict | None, prints_text: bool = True) -> bool:
    """Every clause measured, none diverged, at least one confirmed.

    ``unreachable``/``unavailable`` are honest non-verdicts and do not block;
    ``unmeasured`` and ``diverged`` do. Zero clauses counts as verified only for
    a card that prints no text at all -- the extractor can also yield zero
    clauses when its preferred source is empty (an official-DB bundle with no
    ``text_sections``), and an empty denominator is not a verified one.
    """
    if not exam:
        return False
    if exam.get("total_clauses", 0) == 0:
        return not prints_text
    bv = exam.get("by_verdict", {})
    return bv.get("unmeasured", 0) == 0 and bv.get("diverged", 0) == 0 and bv.get("confirmed", 0) > 0


def has_printed_text(record: dict | None) -> bool:
    """Does ``cards.json`` carry any effect, inherited or security text?"""
    rec = record or {}
    return any((rec.get(k) or "").strip() for k in (
        "effect_description_eng", "inherited_effect_description_eng", "security_effect_description_eng"))


def card_stage(implemented: bool, has_test: bool, known_gap: bool, exam: dict | None,
               printed_text: bool = True) -> str:
    if not implemented:
        return "missing"
    if known_gap:
        return "known_gap"
    if not has_test:
        return "untested"
    if is_dcgo_verified(exam, printed_text):
        return "verified"
    return "tested"


def load_alias_map(path: Path) -> dict[str, str]:
    """``{lowercase alias or canonical: canonical}`` from ``archetype_aliases.json``."""
    raw = load_json(path, {}) or {}
    out: dict[str, str] = {}
    for canonical, aliases in raw.items():
        out[canonical.lower()] = canonical
        for alias in aliases or []:
            out[str(alias).lower()] = canonical
    return out


def canonicalize_archetype(name: str, alias_map: dict[str, str]) -> str:
    folded = (name.replace("’", "'").replace("‘", "'")
              .replace("“", '"').replace("”", '"'))
    return alias_map.get(folded.lower(), folded)


def training_gate_archetypes(tracker: dict[str, dict], alias_map: dict[str, str]) -> set[str]:
    """Archetypes the training deck pool admits today (its "Gate 1").

    Mirrors ``digimon_gym.agents.gauntlet._load_fully_implemented_archetypes``
    (which cannot be imported here -- it pulls in numpy, gymnasium and the PyO3
    engine): ledger entries are grouped by their free-form ``archetype`` label,
    canonicalised through the alias map, and a label is admitted when every
    entry under it is ``IMPLEMENTED`` or ``AUDITED-OK``. A library archetype
    whose name matches no admitted label is excluded from training even when
    every card in its lists is implemented -- which is why this is reported
    next to "playable".
    """
    by_label: dict[str, list[str]] = defaultdict(list)
    for row in tracker.values():
        label, status = row.get("archetype"), row.get("status")
        if label and status:
            by_label[canonicalize_archetype(str(label), alias_map)].append(str(status))
    return {label for label, statuses in by_label.items()
            if statuses and all(s in TRAINING_READY_STATUSES for s in statuses)}


def stage_at_least(stage: str, floor: str) -> bool:
    """``tested`` and ``verified`` both count as tested, etc."""
    order = {"missing": 0, "known_gap": 1, "untested": 1, "tested": 2, "verified": 3}
    return order[stage] >= order[floor]


# --------------------------------------------------------------------------
# Work ordering
# --------------------------------------------------------------------------

def unlock_order(weights: list[float], missing_by_deck: list[set[str]],
                 limit: int | None = None) -> list[dict]:
    """Order the missing cards so field share becomes fully playable soonest.

    Greedy on *fractional unlock value*: each not-yet-playable list spreads its
    weight evenly over its remaining missing cards, so a card that is one of the
    last two blockers of a 10%-share deck outranks a card scattered across many
    lists that each still miss fifteen others. Ties break on card id so the
    order is deterministic.
    """
    remaining = [set(m) for m in missing_by_deck]
    playable = sum(w for w, m in zip(weights, remaining) if not m)
    order: list[dict] = []
    while limit is None or len(order) < limit:
        value: dict[str, float] = defaultdict(float)
        for w, m in zip(weights, remaining):
            if m:
                part = w / len(m)
                for c in m:
                    value[c] += part
        if not value:
            break
        best = min(value, key=lambda c: (-value[c], c))
        newly = 0.0
        for i, m in enumerate(remaining):
            if best in m:
                m.discard(best)
                if not m:
                    newly += weights[i]
        playable += newly
        order.append({
            "card_id": best,
            "unlock_value": value[best],
            "newly_playable_pct": newly * 100,
            "cumulative_playable_pct": playable * 100,
        })
    return order


def milestones(order: list[dict], start_pct: float,
               targets=(25, 50, 75, 90, 100)) -> list[dict]:
    """How many cards (in ``order``) until each playable-share target."""
    out = []
    for t in targets:
        if start_pct >= t - 1e-9:
            out.append({"target_pct": t, "cards": 0})
            continue
        hit = next((i + 1 for i, row in enumerate(order)
                    if row["cumulative_playable_pct"] >= t - 1e-6), None)
        out.append({"target_pct": t, "cards": hit})
    return out


# --------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------

def _pct(num: float, den: float) -> float:
    return round(100.0 * num / den, 2) if den else 0.0


def card_meta(cid: str, cards_index: dict) -> dict:
    rec = cards_index.get(cid) or {}
    return {
        "name": rec.get("card_name_eng") or "?",
        "set": card_set(cid),
        "kind": CARD_KINDS.get(rec.get("card_kind"), "?"),
        "level": rec.get("level"),
        "in_cards_json": cid in cards_index,
    }


def build_report(*, decks: list[Deck], weights: list[float], weighting: dict,
                 cards_index: dict, implemented: dict[str, str], test_refs: set[str],
                 tracker: dict[str, dict], exam: dict[str, dict], window: dict,
                 tested_cards_snapshot: list[str] | None = None,
                 alias_map: dict[str, str] | None = None,
                 queue_limit: int = 60) -> dict:
    alias_map = alias_map or {}
    copies: dict[str, float] = defaultdict(float)
    incl: dict[str, float] = defaultdict(float)
    lists: Counter = Counter()
    arch_weight: dict[str, dict[str, float]] = defaultdict(lambda: defaultdict(float))
    for d, w in zip(decks, weights):
        for c, n in d.counts.items():
            copies[c] += w * n
            incl[c] += w
            lists[c] += 1
            arch_weight[c][d.archetype] += w

    def status(cid: str) -> dict:
        impl = cid in implemented and cid in cards_index
        has_test = cid in test_refs
        trow = tracker.get(cid) or {}
        gap = (trow.get("status") or "").upper() in KNOWN_GAP_STATUSES
        ex = exam.get(cid)
        return {
            "implemented": impl,
            "has_test": has_test,
            "known_gap": gap,
            "stage": card_stage(impl, has_test, gap, ex, has_printed_text(cards_index.get(cid))),
            "tracker_status": trow.get("status"),
            "gap_kind": trow.get("gap_kind"),
        }

    statuses = {c: status(c) for c in copies}
    total_copies = sum(copies.values())

    def copies_pct(floor: str) -> float:
        return _pct(sum(v for c, v in copies.items() if stage_at_least(statuses[c]["stage"], floor)),
                    total_copies)

    def unique(floor: str) -> int:
        return sum(1 for c in copies if stage_at_least(statuses[c]["stage"], floor))

    stage_copies = {s: _pct(sum(v for c, v in copies.items() if statuses[c]["stage"] == s), total_copies)
                    for s in STAGES}
    stage_unique = Counter(statuses[c]["stage"] for c in copies)

    # Deck readiness.
    missing_by_deck = [{c for c in d.counts if not statuses[c]["implemented"]} for d in decks]

    def deck_share(floor: str) -> tuple[float, int]:
        share, count = 0.0, 0
        for d, w in zip(decks, weights):
            if all(stage_at_least(statuses[c]["stage"], floor) for c in d.counts):
                share += w
                count += 1
        return share * 100, count

    playable_pct, playable_lists = deck_share("untested")
    tested_deck_pct, tested_lists = deck_share("tested")
    verified_deck_pct, verified_lists = deck_share("verified")

    # What the training deck pool would actually admit today: its archetype
    # gate (ledger labels) AND every card in the registry.
    gate_ready = training_gate_archetypes(tracker, alias_map)

    def passes_gate(archetype: str) -> bool:
        return canonicalize_archetype(archetype, alias_map) in gate_ready

    trainable_pct = sum(w for d, w, m in zip(decks, weights, missing_by_deck)
                        if not m and passes_gate(d.archetype)) * 100
    trainable_lists = sum(1 for d, m in zip(decks, missing_by_deck) if not m and passes_gate(d.archetype))

    # Clauses: each clause counts once per card, weighted by how much of the
    # field plays that card (copies do not change what a clause does).
    clause_totals = Counter()
    field_total = field_confirmed = 0.0
    for c in copies:
        ex = exam.get(c)
        if not ex:
            continue
        clause_totals["total"] += ex["total_clauses"]
        for v in VERDICTS:
            clause_totals[v] += ex["by_verdict"].get(v, 0)
        field_total += incl[c] * ex["total_clauses"]
        field_confirmed += incl[c] * ex["by_verdict"].get("confirmed", 0)

    headline = {
        "decklists": len(decks),
        "meta_cards": len(copies),
        "avg_deck_size": round(total_copies, 2),
        "implemented": {"unique": unique("untested"), "unique_pct": _pct(unique("untested"), len(copies)),
                        "copies_pct": copies_pct("untested")},
        "tested": {"unique": unique("tested"), "unique_pct": _pct(unique("tested"), len(copies)),
                   "copies_pct": copies_pct("tested")},
        "verified": {"unique": unique("verified"), "unique_pct": _pct(unique("verified"), len(copies)),
                     "copies_pct": copies_pct("verified")},
        "stage_copies_pct": stage_copies,
        "stage_unique": {s: stage_unique.get(s, 0) for s in STAGES},
        "decks": {
            "playable_pct": round(playable_pct, 2), "playable_lists": playable_lists,
            "tested_pct": round(tested_deck_pct, 2), "tested_lists": tested_lists,
            "verified_pct": round(verified_deck_pct, 2), "verified_lists": verified_lists,
            "trainable_pct": round(trainable_pct, 2), "trainable_lists": trainable_lists,
            "trainable_gap_pct": round(playable_pct - trainable_pct, 2),
        },
        "clauses": {
            "total": clause_totals["total"],
            "by_verdict": {v: clause_totals[v] for v in VERDICTS},
            "confirmed_pct": _pct(clause_totals["confirmed"], clause_totals["total"]),
            "field_weighted_confirmed_pct": _pct(field_confirmed, field_total),
        },
    }

    # Per set: which releases carry the meta, and how much of each we run.
    set_copies: dict[str, float] = defaultdict(float)
    set_impl_copies: dict[str, float] = defaultdict(float)
    set_cards: dict[str, set] = defaultdict(set)
    for c, v in copies.items():
        s = card_set(c)
        set_copies[s] += v
        set_cards[s].add(c)
        if statuses[c]["implemented"]:
            set_impl_copies[s] += v
    pool_by_set = Counter(card_set(c) for c in cards_index)
    impl_by_set = Counter(card_set(c) for c in implemented if c in cards_index)
    by_set = []
    for s in sorted(set_copies, key=lambda k: -set_copies[k]):
        cards_in = set_cards[s]
        by_set.append({
            "set": s,
            "meta_cards": len(cards_in),
            "meta_cards_implemented": sum(1 for c in cards_in if statuses[c]["implemented"]),
            "share_of_meta_copies_pct": _pct(set_copies[s], total_copies),
            "implemented_copies_pct": _pct(set_impl_copies[s], set_copies[s]),
            "pool_cards": pool_by_set.get(s, 0),
            "pool_implemented": impl_by_set.get(s, 0),
        })

    # Per archetype.
    entry_of = {}
    for row in weighting.get("rows", []):
        for a in row["library_archetypes"]:
            entry_of[a] = row
    arch_idx: dict[str, list[int]] = defaultdict(list)
    for i, d in enumerate(decks):
        arch_idx[d.archetype].append(i)
    archetypes = []
    for arch, idx in arch_idx.items():
        w_sum = sum(weights[i] for i in idx)
        a_copies: dict[str, float] = defaultdict(float)
        a_incl: dict[str, float] = defaultdict(float)
        for i in idx:
            for c, n in decks[i].counts.items():
                a_copies[c] += weights[i] * n
                a_incl[c] += weights[i]
        a_total = sum(a_copies.values())
        miss_counts = [len(missing_by_deck[i]) for i in idx]
        closest = min(idx, key=lambda i: (len(missing_by_deck[i]), decks[i].date))
        a_clause_total = sum(a_incl[c] * exam[c]["total_clauses"] for c in a_incl if c in exam)
        a_clause_conf = sum(a_incl[c] * exam[c]["by_verdict"].get("confirmed", 0) for c in a_incl if c in exam)
        top_missing = sorted((c for c in a_copies if not statuses[c]["implemented"]),
                             key=lambda c: (-a_copies[c], c))[:8]
        row = entry_of.get(arch)
        archetypes.append({
            "archetype": arch,
            "lists": len(idx),
            "field_share_pct": round(w_sum * 100, 2),
            "sample_share_pct": _pct(len(idx), len(decks)),
            "digilab_deck": row["name"] if row else None,
            "digilab_share_pct": row["share_pct"] if row else None,
            "implemented_copies_pct": _pct(sum(v for c, v in a_copies.items()
                                               if stage_at_least(statuses[c]["stage"], "untested")), a_total),
            "tested_copies_pct": _pct(sum(v for c, v in a_copies.items()
                                          if stage_at_least(statuses[c]["stage"], "tested")), a_total),
            "verified_copies_pct": _pct(sum(v for c, v in a_copies.items()
                                            if stage_at_least(statuses[c]["stage"], "verified")), a_total),
            "clause_confirmed_pct": _pct(a_clause_conf, a_clause_total),
            "playable_lists": sum(1 for m in miss_counts if m == 0),
            "training_gate": passes_gate(arch),
            "min_missing": min(miss_counts),
            "closest_list_missing": sorted(missing_by_deck[closest]),
            "top_missing": [{"card_id": c, **card_meta(c, cards_index),
                             "avg_copies": round(a_copies[c] / w_sum, 2) if w_sum else 0.0,
                             "in_lists_pct": round(100 * a_incl[c] / w_sum, 1) if w_sum else 0.0}
                            for c in top_missing],
        })
    archetypes.sort(key=lambda r: (-r["field_share_pct"], r["archetype"]))

    # Every meta card.
    def top_archs(c: str, k: int = 3) -> list[list]:
        total = incl[c]
        ranked = sorted(arch_weight[c].items(), key=lambda kv: (-kv[1], kv[0]))[:k]
        return [[a, round(100 * w / total, 1) if total else 0.0] for a, w in ranked]

    cards = []
    for c in sorted(copies, key=lambda k: (-copies[k], k)):
        st = statuses[c]
        ex = exam.get(c)
        cards.append({
            "card_id": c,
            **card_meta(c, cards_index),
            **{k: st[k] for k in ("stage", "implemented", "has_test", "known_gap", "tracker_status", "gap_kind")},
            "yaml_path": implemented.get(c),
            "weighted_copies": round(copies[c], 3),
            "field_inclusion_pct": round(incl[c] * 100, 2),
            "lists": lists[c],
            "top_archetypes": top_archs(c),
            "clauses": ex["total_clauses"] if ex else None,
            "by_verdict": ex["by_verdict"] if ex else None,
        })
    card_by_id = {row["card_id"]: row for row in cards}

    # Queues.
    full_order = unlock_order(weights, missing_by_deck)
    unlock_curve = [[row["card_id"], round(row["cumulative_playable_pct"], 2)] for row in full_order]
    implement_next = []
    for rank, row in enumerate(full_order[:queue_limit], 1):
        cm = card_by_id[row["card_id"]]
        implement_next.append({
            "rank": rank,
            "card_id": row["card_id"],
            **{k: cm[k] for k in ("name", "set", "kind", "level", "in_cards_json", "weighted_copies",
                                  "field_inclusion_pct", "lists", "top_archetypes", "clauses")},
            "unlock_value_pct": round(row["unlock_value"] * 100, 3),
            "newly_playable_pct": round(row["newly_playable_pct"], 2),
            "cumulative_playable_pct": round(row["cumulative_playable_pct"], 2),
        })
    exam_next = sorted(
        (r for r in cards if r["stage"] in ("tested", "untested") and r["by_verdict"]
         and r["by_verdict"]["unmeasured"] > 0),
        key=lambda r: (-(r["field_inclusion_pct"] * r["by_verdict"]["unmeasured"]), r["card_id"]),
    )[:queue_limit]
    known_gaps = [r for r in cards if r["known_gap"]]
    diverged = [r for r in cards if r["by_verdict"] and r["by_verdict"]["diverged"] > 0]

    pool_impl = {c for c in implemented if c in cards_index}
    pool = {
        "cards_json_total": len(cards_index),
        "implemented": len(pool_impl),
        "implemented_pct": _pct(len(pool_impl), len(cards_index)),
        "tested": sum(1 for c in pool_impl if c in test_refs
                      and (tracker.get(c, {}).get("status") or "").upper() not in KNOWN_GAP_STATUSES),
        "known_gap": sum(1 for c in pool_impl
                         if (tracker.get(c, {}).get("status") or "").upper() in KNOWN_GAP_STATUSES),
        # Needs the binding to cover every implemented card, not just the
        # window's -- main() binds the union.
        "verified": sum(1 for c in pool_impl if c in test_refs
                        and (tracker.get(c, {}).get("status") or "").upper() not in KNOWN_GAP_STATUSES
                        and is_dcgo_verified(exam.get(c), has_printed_text(cards_index.get(c)))),
        "cards_with_confirmed_clause": sum(1 for c in pool_impl if exam.get(c)
                                           and exam[c]["by_verdict"].get("confirmed", 0) > 0),
    }
    if tested_cards_snapshot is not None:
        snap = set(tested_cards_snapshot)
        pool["deck_builder_allowlist"] = len(snap)
        pool["allowlist_missing_implemented"] = sorted(pool_impl - snap)
        pool["allowlist_lag"] = len(pool["allowlist_missing_implemented"])

    warnings = []
    if len(decks) < THIN_WINDOW_LISTS:
        warnings.append(
            f"Only {len(decks)} decklists fall in this window (< {THIN_WINDOW_LISTS}); card-level figures are "
            "a small sample. Refresh data/deck_library.json (meta_loader.py --scrape-dcg-nexus) and re-run."
        )
    unknown = sorted(c for c in copies if c not in cards_index)
    if unknown:
        warnings.append(f"{len(unknown)} meta card id(s) are not in cards.json (un-ingested set?): "
                        + ", ".join(unknown[:12]) + ("..." if len(unknown) > 12 else ""))

    return {
        "window": window,
        "weighting": weighting,
        "headline": headline,
        "pool": pool,
        "by_set": by_set,
        "archetypes": archetypes,
        "implement_next": implement_next,
        "unlock_milestones": milestones(full_order, playable_pct),
        "unlock_total_cards": len(full_order),
        "unlock_curve": unlock_curve,
        "exam_next": exam_next,
        "known_gaps": known_gaps,
        "diverged": diverged,
        "cards": cards,
        "warnings": warnings,
    }


# --------------------------------------------------------------------------
# Launch plan (hand-maintained gates, evaluated against the report)
# --------------------------------------------------------------------------

_SEGMENT = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)(?:\[([^\]]+)\])?$")
_ROW_KEYS = ("set", "archetype", "card_id", "id")


def resolve_metric(report: dict, path: str):
    """Look up ``headline.decks.playable_pct`` or ``by_set[BT26].meta_cards_implemented``.

    ``name[key]`` selects the row of list ``name`` whose set / archetype / card
    id equals ``key``. Returns ``None`` when any step is missing, so a gate on a
    set that has not appeared in the window reads as unmeasured, not as zero.
    """
    node = report
    for part in path.split("."):
        m = _SEGMENT.match(part)
        if not m or not isinstance(node, dict):
            return None
        node = node.get(m.group(1))
        if m.group(2) is not None:
            if not isinstance(node, list):
                return None
            key = m.group(2)
            node = next((row for row in node if isinstance(row, dict)
                         and any(row.get(k) == key for k in _ROW_KEYS)), None)
        if node is None:
            return None
    return node


def evaluate_gate(gate: dict, report: dict) -> dict:
    out = dict(gate)
    if "metric" not in gate:
        out["met"] = bool(gate.get("met"))
        return out
    value = resolve_metric(report, gate["metric"])
    out["value"] = value
    if not isinstance(value, (int, float)):
        out["met"] = False
        out["progress"] = None
        return out
    target = gate["target"]
    if gate.get("op", ">=") == "<=":
        out["met"] = value <= target
        out["progress"] = 1.0 if out["met"] else None
    else:
        out["met"] = value >= target
        out["progress"] = max(0.0, min(1.0, value / target)) if target else 1.0
    return out


def evaluate_plan(plan: dict | None, report: dict) -> dict | None:
    """Copy of the plan with every gate's current value and met/progress."""
    if not plan:
        return None
    evaluated = dict(plan)
    evaluated["milestones"] = []
    for ms in plan.get("milestones", []):
        row = dict(ms)
        row["gates"] = [evaluate_gate(g, report) for g in ms.get("gates", [])]
        row["gates_met"] = sum(1 for g in row["gates"] if g["met"])
        evaluated["milestones"].append(row)
    return evaluated


# --------------------------------------------------------------------------
# Trend (weekly backfill from git history + the verdict ledger)
# --------------------------------------------------------------------------

def git_first_added(cards_dir: Path) -> dict[str, str] | None:
    """``{card_id: YYYY-MM-DD}`` -- when each spec file first landed.

    ``None`` on a shallow clone (the early history is missing, so every spec
    would look like it appeared at the shallow boundary) or without git.
    """
    try:
        shallow = subprocess.run(["git", "rev-parse", "--is-shallow-repository"], cwd=REPO_ROOT,
                                 capture_output=True, text=True, check=True).stdout.strip()
        if shallow == "true":
            return None
        rel = cards_dir.resolve().relative_to(REPO_ROOT).as_posix()
        log = subprocess.run(["git", "log", "--diff-filter=A", "--name-only", "--format=@%cs", "--", rel],
                             cwd=REPO_ROOT, capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError, ValueError):
        return None
    first: dict[str, str] = {}
    when = None
    for line in log.splitlines():
        if line.startswith("@"):
            when = line[1:]
        elif when and line.endswith((".yaml", ".yml")):
            stem = Path(line).stem
            if stem not in first or when < first[stem]:
                first[stem] = when
    return first


def trend_points(*, all_decks: list[Deck], cards_index: dict, implemented: dict[str, str],
                 first_added: dict[str, str], exam: dict[str, dict],
                 verdict_times: dict[str, tuple[str, str]], start: str, end: str,
                 step_days: int = 7, window_days: int = 42, min_lists: int = 20) -> list[dict]:
    """Coverage of a rolling window of the meta by what was implemented *then*.

    Each point uses the lists dated in ``(D - window_days, D]`` (equal weight --
    field shares only exist per format) and the specs that had landed by ``D``.
    Clause confirmations use the ledger's ``recorded_at``.
    """
    earliest = min(first_added.values()) if first_added else start
    impl_dates = {c: first_added.get(c, earliest) for c in implemented if c in cards_index}
    points = []
    d = date.fromisoformat(start)
    stop = date.fromisoformat(end)
    while d <= stop:
        hi = d.isoformat()
        lo = (d - timedelta(days=window_days)).isoformat()
        window = [k for k in all_decks if lo < k.date <= hi]
        if len(window) >= min_lists:
            done = {c for c, when in impl_dates.items() if when <= hi}
            copies: dict[str, float] = defaultdict(float)
            playable = 0
            for k in window:
                if all(c in done for c in k.counts):
                    playable += 1
                for c, n in k.counts.items():
                    copies[c] += n
            total = sum(copies.values())
            ctotal = cconf = 0
            for c in copies:
                ex = exam.get(c)
                if not ex:
                    continue
                ctotal += ex["total_clauses"]
                cconf += sum(1 for cid in ex["clause_ids"]
                             if verdict_times.get(cid, ("", ""))[0] == "confirmed"
                             and verdict_times[cid][1] and verdict_times[cid][1] <= hi)
            points.append({
                "date": hi,
                "lists": len(window),
                "meta_cards": len(copies),
                "implemented_copies_pct": _pct(sum(v for c, v in copies.items() if c in done), total),
                "implemented_unique_pct": _pct(sum(1 for c in copies if c in done), len(copies)),
                "playable_decks_pct": _pct(playable, len(window)),
                "clause_confirmed_pct": _pct(cconf, ctotal),
                "pool_implemented": len(done),
            })
        d += timedelta(days=step_days)
    return points


# --------------------------------------------------------------------------
# Rendering
# --------------------------------------------------------------------------

def _fmt_pct(v) -> str:
    return "—" if v is None else f"{v:.1f}%"


def render_markdown(report: dict) -> str:
    h = report["headline"]
    w = report["window"]
    wt = report["weighting"]
    lines = [
        "# Meta coverage",
        "",
        "**Generated — do not hand-edit.** Regenerate with `PYTHONPATH=code python -m tools.meta_coverage`.",
        "",
        f"- Generated: {report['generated_at']} (git `{report.get('git_head') or '?'}`)",
        f"- Window: **{w.get('label')}** — {w['since']} → {w['until']} · {h['decklists']} decklists · "
        f"{h['meta_cards']} distinct cards · sources {', '.join(f'{k} {v}' for k, v in w['sources'].items())}",
        f"- Weighting: **{wt['mode']}**"
        + (f" — DigiLab names {wt['named_share_pct']}% of the field explicitly; the other "
           f"{wt['other_share_pct']}% is spread over unnamed archetypes by list count"
           if wt["mode"] == "digilab" else ""),
    ]
    if wt.get("unrepresented"):
        lines.append(f"- Named by DigiLab but no lists in window: {', '.join(wt['unrepresented'])}")
    for warning in report.get("warnings", []):
        lines.append(f"- ⚠️ {warning}")
    lines += [
        "",
        "## Headline",
        "",
        "| Measure | Implemented | Tested | DCGO-verified |",
        "|---|---|---|---|",
        f"| Share of meta card copies | {_fmt_pct(h['implemented']['copies_pct'])} | "
        f"{_fmt_pct(h['tested']['copies_pct'])} | {_fmt_pct(h['verified']['copies_pct'])} |",
        f"| Distinct meta cards | {h['implemented']['unique']}/{h['meta_cards']} "
        f"({_fmt_pct(h['implemented']['unique_pct'])}) | {h['tested']['unique']}/{h['meta_cards']} "
        f"({_fmt_pct(h['tested']['unique_pct'])}) | {h['verified']['unique']}/{h['meta_cards']} "
        f"({_fmt_pct(h['verified']['unique_pct'])}) |",
        f"| Field share of decks with every card at this level | {_fmt_pct(h['decks']['playable_pct'])} "
        f"({h['decks']['playable_lists']} lists) | {_fmt_pct(h['decks']['tested_pct'])} "
        f"({h['decks']['tested_lists']} lists) | {_fmt_pct(h['decks']['verified_pct'])} "
        f"({h['decks']['verified_lists']} lists) |",
        "",
        f"**Trainable today**: {_fmt_pct(h['decks']['trainable_pct'])} of the field "
        f"({h['decks']['trainable_lists']} lists) — fully playable AND admitted by the training deck pool's "
        "archetype gate (`gauntlet._load_fully_implemented_archetypes`, which matches free-form ledger labels).",
        "",
        f"Clauses on meta cards: {h['clauses']['total']} — "
        + ", ".join(f"{v} {h['clauses']['by_verdict'][v]}" for v in VERDICTS)
        + f" ({_fmt_pct(h['clauses']['confirmed_pct'])} confirmed; "
          f"{_fmt_pct(h['clauses']['field_weighted_confirmed_pct'])} weighted by field play rate).",
        "",
        f"Whole pool: {report['pool']['implemented']}/{report['pool']['cards_json_total']} cards implemented "
        f"({_fmt_pct(report['pool']['implemented_pct'])}), {report['pool']['known_gap']} flagged PARTIAL/BLOCKED, "
        f"{report['pool']['verified']} DCGO-verified.",
    ]
    if report["pool"].get("allowlist_missing_implemented"):
        missing = report["pool"]["allowlist_missing_implemented"]
        lines.append(f"Deck-builder allowlist (`data/tested_cards.json`) lags the engine by {len(missing)} "
                     f"implemented card(s) — regenerate with `python code/tools/build_tested_cards.py`.")
    plan = report.get("plan")
    if plan:
        lines += ["", "## Launch plan", "", f"Goal: {plan.get('goal', '')} (plan updated {plan.get('updated', '?')}; "
                  "edit `qa/qa-reports/meta-coverage/launch_plan.json`)", ""]
        for ms in plan["milestones"]:
            lines.append(f"- **{ms['id']} · {ms['name']}** ({ms.get('status', '?')}) — "
                         f"{ms['gates_met']}/{len(ms['gates'])} gates met")
            for g in ms["gates"]:
                mark = "x" if g["met"] else " "
                if "metric" in g:
                    value = g.get("value")
                    shown = "—" if value is None else (f"{value:g}" if isinstance(value, (int, float)) else str(value))
                    lines.append(f"  - [{mark}] {g['label']}: {shown} (target {g.get('op', '>=')} "
                                 f"{g['target']:g} {g.get('unit', '')})".rstrip())
                else:
                    lines.append(f"  - [{mark}] {g['label']}")
    ms = ", ".join(f"{m['target_pct']}% → {m['cards'] if m['cards'] is not None else 'n/a'} cards"
                   for m in report["unlock_milestones"])
    lines += [
        "",
        f"**Unlock path** (cards to implement, in `implement_next` order, until that much of the field is "
        f"fully playable): {ms}. {report['unlock_total_cards']} missing cards in total.",
        "",
        "## By set",
        "",
        "| Set | Share of meta copies | Meta cards implemented | Implemented copies | Whole set implemented |",
        "|---|---|---|---|---|",
    ]
    for row in report["by_set"][:14]:
        lines.append(f"| {row['set']} | {_fmt_pct(row['share_of_meta_copies_pct'])} | "
                     f"{row['meta_cards_implemented']}/{row['meta_cards']} | "
                     f"{_fmt_pct(row['implemented_copies_pct'])} | {row['pool_implemented']}/{row['pool_cards']} |")
    lines += [
        "",
        "## Archetypes",
        "",
        "| Archetype | Field share | Lists | Implemented | Tested | Verified | Clauses confirmed | "
        "Playable lists | Training gate | Closest list missing |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for row in report["archetypes"][:25]:
        closest = ", ".join(row["closest_list_missing"][:6]) + ("…" if len(row["closest_list_missing"]) > 6 else "")
        lines.append(
            f"| {row['archetype']} | {_fmt_pct(row['field_share_pct'])} | {row['lists']} | "
            f"{_fmt_pct(row['implemented_copies_pct'])} | {_fmt_pct(row['tested_copies_pct'])} | "
            f"{_fmt_pct(row['verified_copies_pct'])} | {_fmt_pct(row['clause_confirmed_pct'])} | "
            f"{row['playable_lists']}/{row['lists']} | {'pass' if row['training_gate'] else 'fail'} | "
            f"{row['min_missing']}" + (f" ({closest})" if closest else "") + " |")
    lines += [
        "",
        "## Implement next",
        "",
        "Ordered to make the most field share fully playable soonest (see `unlock_order`).",
        "",
        "| # | Card | Name | Kind | Copies/deck | Field play | Clauses | Cumulative playable | Played in |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for row in report["implement_next"][:30]:
        arch = ", ".join(f"{a} {p:.0f}%" for a, p in row["top_archetypes"])
        flag = "" if row["in_cards_json"] else " ⚠️ not ingested"
        lines.append(f"| {row['rank']} | {row['card_id']}{flag} | {row['name']} | {row['kind']} | "
                     f"{row['weighted_copies']:.2f} | {_fmt_pct(row['field_inclusion_pct'])} | "
                     f"{row['clauses'] if row['clauses'] is not None else '?'} | "
                     f"{_fmt_pct(row['cumulative_playable_pct'])} | {arch} |")
    lines += [
        "",
        "## Exam next (implemented, clauses unmeasured)",
        "",
        "| Card | Name | Field play | Unmeasured | Confirmed | Clauses |",
        "|---|---|---|---|---|---|",
    ]
    for row in report["exam_next"][:25]:
        bv = row["by_verdict"]
        lines.append(f"| {row['card_id']} | {row['name']} | {_fmt_pct(row['field_inclusion_pct'])} | "
                     f"{bv['unmeasured']} | {bv['confirmed']} | {row['clauses']} |")
    if report["known_gaps"]:
        lines += ["", "## Known gaps in the meta (PARTIAL / BLOCKED)", "",
                  "| Card | Name | Status | Gap kind | Field play |", "|---|---|---|---|---|"]
        for row in report["known_gaps"]:
            lines.append(f"| {row['card_id']} | {row['name']} | {row['tracker_status']} | "
                         f"{row['gap_kind'] or '—'} | {_fmt_pct(row['field_inclusion_pct'])} |")
    if report.get("trend"):
        lines += ["", "## Trend (rolling window, coverage by what was implemented at the time)", "",
                  "| Date | Lists | Implemented copies | Playable decks | Clauses confirmed | Pool implemented |",
                  "|---|---|---|---|---|---|"]
        for p in report["trend"][-16:]:
            lines.append(f"| {p['date']} | {p['lists']} | {_fmt_pct(p['implemented_copies_pct'])} | "
                         f"{_fmt_pct(p['playable_decks_pct'])} | {_fmt_pct(p['clause_confirmed_pct'])} | "
                         f"{p['pool_implemented']} |")
    lines += [
        "",
        "## Definitions",
        "",
        "- **Implemented**: a YAML spec under `code/digimon-engine/cards/` declares the card (what `build.rs` "
        "compiles into the registry).",
        "- **Tested**: implemented, referenced by a Rust test under `code/digimon-engine/tests/`, and not flagged "
        "PARTIAL/BLOCKED in `validated_cards_dsl.json`.",
        "- **DCGO-verified**: tested, and every printed clause has an exam verdict with none `unmeasured` or "
        "`diverged` (`tools.clause_coverage` supplies the denominator).",
        "- **Field share / copies**: each decklist is weighted so its archetype carries its DigiLab share "
        "(`data/meta_shares.json`); copies are expected copies per deck in the field.",
        "",
    ]
    return "\n".join(lines)


def slim_for_dashboard(report: dict, card_limit: int = 150) -> dict:
    slim = dict(report)
    slim["cards"] = report["cards"][:card_limit]
    return slim


def render_dashboard(report: dict, template_path: Path = DASHBOARD_TEMPLATE, *, standalone: bool = True) -> str:
    """Fill the template with the report.

    The template is a page *body* (it is also published as a claude.ai
    Artifact, whose host supplies the document skeleton). ``standalone`` wraps
    it in a doctype + head so the local file renders in standards mode.
    """
    template = template_path.read_text(encoding="utf-8")
    if DASHBOARD_PLACEHOLDER not in template:
        raise ValueError(f"{template_path}: missing placeholder {DASHBOARD_PLACEHOLDER!r}")
    payload = json.dumps(slim_for_dashboard(report), separators=(",", ":")).replace("</", "<\\/")
    page = template.replace(DASHBOARD_PLACEHOLDER, payload)
    if not standalone:
        return page
    return ('<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
            '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n'
            '</head>\n<body>\n' + page + '\n</body>\n</html>\n')


def history_row(report: dict) -> dict:
    h = report["headline"]
    return {
        "generated_at": report["generated_at"],
        "git_head": report.get("git_head"),
        "label": report["window"].get("label"),
        "since": report["window"]["since"],
        "until": report["window"]["until"],
        "weighting": report["weighting"]["mode"],
        "decklists": h["decklists"],
        "meta_cards": h["meta_cards"],
        "implemented_copies_pct": h["implemented"]["copies_pct"],
        "tested_copies_pct": h["tested"]["copies_pct"],
        "verified_copies_pct": h["verified"]["copies_pct"],
        "implemented_unique_pct": h["implemented"]["unique_pct"],
        "playable_decks_pct": h["decks"]["playable_pct"],
        "trainable_decks_pct": h["decks"]["trainable_pct"],
        "clause_confirmed_pct": h["clauses"]["confirmed_pct"],
        "pool_implemented": report["pool"]["implemented"],
    }


def append_history(path: Path, row: dict) -> bool:
    """Append ``row`` unless it repeats the last row's measurement.

    Re-running on the same commit and window adds nothing to a rolling record,
    so only ``generated_at`` may differ for a row to be skipped. Returns whether
    a row was written.
    """
    if path.exists():
        lines = [ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()]
        if lines:
            try:
                last = json.loads(lines[-1])
            except json.JSONDecodeError:
                last = None
            if last is not None:
                strip = lambda r: {k: v for k, v in r.items() if k != "generated_at"}  # noqa: E731
                if strip(last) == strip(row):
                    return False
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(row) + "\n")
    return True


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------

def _git_head() -> str | None:
    try:
        return subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=REPO_ROOT,
                              capture_output=True, text=True, check=True).stdout.strip() or None
    except (OSError, subprocess.CalledProcessError):
        return None


def pick_format(shares: dict, name: str | None, until: str) -> tuple[str | None, dict | None]:
    formats = (shares or {}).get("formats", {})
    if name:
        if name not in formats:
            raise SystemExit(f"format {name!r} not in meta_shares.json (have: {', '.join(sorted(formats))})")
        return name, formats[name]
    live = [(spec["start"], key) for key, spec in formats.items() if spec.get("start", "9999") <= until]
    if not live:
        return None, None
    key = max(live)[1]
    return key, formats[key]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--shares", type=Path, default=DEFAULT_SHARES)
    parser.add_argument("--cards-json", type=Path, default=DEFAULT_CARDS_JSON)
    parser.add_argument("--tested-cards", type=Path, default=DEFAULT_TESTED_CARDS)
    parser.add_argument("--aliases", type=Path, default=DEFAULT_ALIASES)
    parser.add_argument("--cards-dir", type=Path, default=DEFAULT_CARDS_DIR)
    parser.add_argument("--tests-dir", type=Path, default=DEFAULT_TESTS_DIR)
    parser.add_argument("--tracker", type=Path, default=DEFAULT_TRACKER)
    parser.add_argument("--verdicts", type=Path, default=DEFAULT_VERDICTS)
    parser.add_argument("--scenarios", type=Path, default=DEFAULT_SCENARIOS)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--plan", type=Path, default=DEFAULT_PLAN,
                        help="hand-maintained launch plan whose gates are evaluated against the report")
    parser.add_argument("--format", dest="format_name",
                        help="format key in meta_shares.json (default: newest that has started)")
    parser.add_argument("--since", help="window start YYYY-MM-DD (overrides the format window)")
    parser.add_argument("--until", help="window end YYYY-MM-DD (default: newest list in the library)")
    parser.add_argument("--window-days", type=int, help="window = the N days ending at --until")
    parser.add_argument("--weighting", choices=("auto", "digilab", "sample"), default="auto",
                        help="auto = digilab when the window is a format window with a snapshot")
    parser.add_argument("--trend", action="store_true", help="add a weekly backfill (needs full git history)")
    parser.add_argument("--trend-start", default="2026-04-01")
    parser.add_argument("--trend-window-days", type=int, default=42)
    parser.add_argument("--html", action="store_true", help="also write dashboard.html")
    parser.add_argument("--artifact-out", type=Path,
                        help="also write the dashboard as a body-only page (for publishing as a claude.ai Artifact)")
    parser.add_argument("--no-history", action="store_true", help="do not append to history.jsonl")
    args = parser.parse_args(argv)

    # cards.json carries a few synthetic engine fixtures (TEST-020..) -- not cards.
    cards_index = {k: v for k, v in (load_json(args.cards_json, {}) or {}).items() if not k.startswith("TEST")}
    library = load_json(args.library, {}) or {}
    shares = load_json(args.shares, {}) or {}
    all_decks = load_decks(library, cards_index)
    if not all_decks:
        raise SystemExit(f"{args.library}: no dated decklists")
    newest = max(d.date for d in all_decks)
    until = args.until or newest

    fmt_key, fmt_spec = (None, None)
    if not args.since and not args.window_days:
        fmt_key, fmt_spec = pick_format(shares, args.format_name, until)
    elif args.format_name:
        fmt_key, fmt_spec = pick_format(shares, args.format_name, until)
    if args.window_days:
        since = (date.fromisoformat(until) - timedelta(days=args.window_days - 1)).isoformat()
    elif args.since:
        since = args.since
    elif fmt_spec:
        since = fmt_spec["start"]
    else:
        since = (date.fromisoformat(until) - timedelta(days=41)).isoformat()

    decks = [d for d in all_decks if since <= d.date <= until]
    if not decks:
        raise SystemExit(f"no decklists between {since} and {until}")
    use_shares = (args.weighting == "digilab"
                  or (args.weighting == "auto" and fmt_spec is not None and not args.since and not args.window_days))
    weights, weighting = deck_weights(decks, fmt_spec if use_shares and fmt_spec else None)
    if use_shares and fmt_spec:
        weighting["source_url"] = fmt_spec.get("source_url")
        weighting["source_window"] = fmt_spec.get("source_window")
        weighting["field"] = fmt_spec.get("field")

    implemented = scan_implemented(args.cards_dir)
    test_refs = scan_test_refs(args.tests_dir)
    tracker = load_tracker(args.tracker)
    alias_map = load_alias_map(args.aliases)
    tested_snapshot = (load_json(args.tested_cards, {}) or {}).get("card_ids")

    exam_cards = {c for d in decks for c in d.counts} | {c for c in implemented if c in cards_index}
    trend_decks: list[Deck] = []
    if args.trend:
        # The first point's window reaches back before --trend-start.
        lookback = (date.fromisoformat(args.trend_start) - timedelta(days=args.trend_window_days)).isoformat()
        trend_decks = [d for d in all_decks if d.date > lookback]
        exam_cards |= {c for d in trend_decks for c in d.counts}
    exam = bind_exam(sorted(exam_cards), args.scenarios, args.verdicts)

    window = {
        "label": fmt_key if fmt_spec and not args.since and not args.window_days else f"{since}..{until}",
        "format": fmt_key,
        "since": since,
        "until": until,
        "library_generated_at": library.get("generated_at"),
        "newest_list": newest,
        "sources": dict(Counter(d.source for d in decks).most_common()),
    }
    report = build_report(decks=decks, weights=weights, weighting=weighting, cards_index=cards_index,
                          implemented=implemented, test_refs=test_refs, tracker=tracker, exam=exam,
                          window=window, tested_cards_snapshot=tested_snapshot, alias_map=alias_map)
    report["generated_at"] = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    report["git_head"] = _git_head()
    report["calendar"] = shares.get("calendar", [])

    if args.trend:
        first_added = git_first_added(args.cards_dir)
        if first_added is None:
            report["warnings"].append("Trend skipped: shallow clone or no git (run `git fetch --unshallow`).")
        else:
            report["trend"] = trend_points(all_decks=trend_decks, cards_index=cards_index,
                                           implemented=implemented, first_added=first_added, exam=exam,
                                           verdict_times=load_verdict_times(args.verdicts),
                                           start=args.trend_start, end=until,
                                           window_days=args.trend_window_days)
            report["trend_window_days"] = args.trend_window_days
    report["plan"] = evaluate_plan(load_json(args.plan), report)

    args.out_dir.mkdir(parents=True, exist_ok=True)
    (args.out_dir / "latest.json").write_text(json.dumps(report, indent=1) + "\n", encoding="utf-8")
    (args.out_dir / "latest.md").write_text(render_markdown(report), encoding="utf-8")
    if not args.no_history:
        append_history(args.out_dir / "history.jsonl", history_row(report))
    if args.html:
        (args.out_dir / "dashboard.html").write_text(render_dashboard(report), encoding="utf-8")
    if args.artifact_out:
        args.artifact_out.parent.mkdir(parents=True, exist_ok=True)
        args.artifact_out.write_text(render_dashboard(report, standalone=False), encoding="utf-8")

    h = report["headline"]
    print(f"meta coverage [{window['label']}] {h['decklists']} lists, {h['meta_cards']} cards: "
          f"implemented {h['implemented']['copies_pct']:.1f}% of copies, tested {h['tested']['copies_pct']:.1f}%, "
          f"verified {h['verified']['copies_pct']:.1f}%; fully playable decks {h['decks']['playable_pct']:.1f}% "
          f"of the field -> {args.out_dir}")
    for warning in report["warnings"]:
        print(f"warning: {warning}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
