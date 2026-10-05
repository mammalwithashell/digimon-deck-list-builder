"""The gating interaction denominator (design D6 / spec "interaction-exams").

Per card, the gating interactions are

1. every official Q&A ruling that lists the card (`data/card_qa.json`), as
   ``qa:<Q-number>`` -- ONE interaction per ruling however many cards print it,
   adjudicated once and counted for each of them;
2. every probe the `probes` generator emits from the card's extracted clauses,
   as ``probe:<clause-id>:<family>[:neg]`` -- gating only when the probe's
   family version is PROMOTED (`promotion.json`). An unpromoted family's probes
   are still listed here (so a scenario may name them and be examined) but carry
   ``"gating": false``.

Both inputs are committed data, so the artifact is a pure function of the repo:
no timestamps, sorted keys, LF line endings. ``--check`` rebuilds it in memory
and fails naming every new / removed / changed interaction, mirroring the
readiness generator's drift gate. Model-authored combos (``combo:``) are never
listed: a model-written list is not a stable denominator.

Artifact shape (``version: 1``)::

    {"version": 1,
     "families": {"families@1": {"promoted": true, "families": [...]}},
     "universe": {"mode": "all"} | {"mode": "cards", "source": "<path>"},
     "summary": {...counts...},
     "cards": {card_id: [interaction ids: qa first (Q-number order), then probes]},
     "interactions": {id: {"source": "qa"|"probe", "card_ids": [...], "kind":
         "positive"|"negative", "gating": bool, "text_sha256": "...",
         "q_id"?, "clause_id"?, "family"?, "family_version"?, "why"?}}}

``cards`` lists EVERY card in the universe, including cards with no
interactions (``[]``), so "this card has no gating interactions" (ready on its
clauses alone, spec "Readiness requires adjudicated gating interactions") is
distinguishable from "this card was never considered". ``text_sha256`` is the
drift fingerprint a stored interaction verdict is checked against: the ruling's
question + answer for ``qa:``, the clause text for ``probe:`` (the same hash the
clause verdicts use).

CLI (`python -m tools.card_loop interactions ...` or
`python -m tools.card_loop.interactions ...`)::

    build [--out data/interaction_denominator.json] [--cards-from FILE]
    --check | check                       exit 1 on drift, naming the ids
    promotion-report [--version V] [--verdicts DIR] [--json OUT]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Iterable, Mapping

from tools.card_loop.interactions import PROBE_PREFIX, QA_PREFIX
from tools.card_loop.interactions.probes import (
    FAMILY_VERSIONS,
    NEGATIVE,
    POSITIVE,
    Family,
    generate_probes,
    natural_key,
)

ROOT = Path(__file__).resolve().parents[4]
DEFAULT_OUT = ROOT / "data" / "interaction_denominator.json"
DEFAULT_CARD_QA = ROOT / "data" / "card_qa.json"
DEFAULT_PROMOTION = Path(__file__).resolve().with_name("promotion.json")
DEFAULT_VERDICTS = ROOT / "qa" / "qa-reports" / "exam-verdicts"

ARTIFACT_VERSION = 1

#: Verdict classes that END an interaction (design D4 terminal states minus
#: `escalated`). `diverged` is a finding still in triage; `unmeasured` is nothing.
ADJUDICATED_VERDICTS = frozenset({"confirmed", "unreachable", "unavailable"})


def sha256_text(text: str) -> str:
    normalized = (text or "").replace("\r\n", "\n").replace("\r", "\n")
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()


def ruling_sha256(ruling: Mapping) -> str:
    """Fingerprint of a ruling's wording: an edited answer invalidates its verdict."""
    return sha256_text(f"{ruling.get('question', '')}\n{ruling.get('answer', '')}")


# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------


def load_card_qa(path: Path | str) -> dict:
    p = Path(path)
    if not p.exists():
        raise FileNotFoundError(
            f"no Q&A mirror at {p}; scrape it with `python -m tools.scrape_official_qa --all`"
        )
    data = json.loads(p.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or not isinstance(data.get("qa"), dict) \
            or not isinstance(data.get("cards"), dict):
        raise ValueError(f"{p} is not a card_qa.json document (needs `cards` and `qa` maps)")
    return data


def load_promotion(path: Path | str, registry: Mapping[str, tuple[Family, ...]] = FAMILY_VERSIONS) -> dict[str, bool]:
    """`{version: promoted}` for every registry version; an unlisted version is
    UNPROMOTED (the safe default: a family never gates by accident)."""
    p = Path(path)
    raw = json.loads(p.read_text(encoding="utf-8")) if p.exists() else {}
    unknown = sorted(set(raw) - set(registry))
    if unknown:
        raise ValueError(f"{p} names family versions the generator does not define: {unknown}")
    out = {}
    for version in registry:
        entry = raw.get(version) or {}
        promoted = entry.get("promoted", False)
        if not isinstance(promoted, bool):
            raise ValueError(f"{p}: {version}.promoted must be true or false")
        out[version] = promoted
    return out


def read_card_ids(path: Path | str) -> list[str]:
    """A card list: JSON list, JSON `{"cards": [...]}`, or one id per line."""
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        data = None
    if isinstance(data, list):
        ids = data
    elif isinstance(data, dict) and isinstance(data.get("cards"), (list, dict)):
        ids = list(data["cards"])
    else:
        ids = [ln.split("#", 1)[0].strip() for ln in text.splitlines()]
    return sorted({str(i).strip() for i in ids if str(i).strip()}, key=natural_key)


def extract_clauses(card_ids: Iterable[str], *, cards_json: Path | None = None,
                    overrides: Path | None = None, official: Path | None = None) -> list[dict]:
    """Clause records for `card_ids` from committed card data only.

    `dcgo_root=None`: the DCGO checkout (base-repo only, rule 29) decides only
    whether a trailing image-required security slot is emitted, which never
    carries probes -- so skipping it keeps the denominator machine-independent
    without moving any probe id.
    """
    from data_paths import CARD_OVERRIDES, CARDS_JSON
    from tools.clause_coverage.card_sources import (
        extract_card_clauses,
        load_cards_index,
        load_official_index,
        load_overrides_index,
    )
    from tools.clause_coverage.extract import DEFAULT_OFFICIAL_JSON

    cards_index = load_cards_index(cards_json or CARDS_JSON)
    overrides_index = load_overrides_index(overrides or CARD_OVERRIDES)
    official_index = load_official_index(official or DEFAULT_OFFICIAL_JSON)
    out: list[dict] = []
    for cid in card_ids:
        for c in extract_card_clauses(
            cid, cards_index=cards_index, overrides_index=overrides_index,
            official_index=official_index, image_cache={}, dcgo_root=None,
        ):
            out.append(c.to_dict())
    return out


def all_card_ids(cards_json: Path | None, card_qa: Mapping) -> list[str]:
    from data_paths import CARDS_JSON

    data = json.loads(Path(cards_json or CARDS_JSON).read_text(encoding="utf-8"))
    ids = set(data) if isinstance(data, dict) else {c["id"] for c in data}
    ids |= set(card_qa.get("cards", {}))
    for ruling in card_qa.get("qa", {}).values():
        ids |= set(ruling.get("card_ids") or [])
    return sorted(ids, key=natural_key)


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------


def _qa_interactions(card_qa: Mapping, universe: set[str]) -> dict[str, dict]:
    """`qa:<Q>` records for every ruling touching a universe card."""
    rulings = card_qa.get("qa", {})
    cards_by_q: dict[str, set[str]] = {}
    for card_id, q_ids in card_qa.get("cards", {}).items():
        for q in q_ids or []:
            if q not in rulings:
                raise ValueError(
                    f"card_qa.json lists {q!r} for {card_id} but has no `qa[{q!r}]` record"
                )
            cards_by_q.setdefault(q, set()).add(card_id)
    for q, ruling in rulings.items():
        cards_by_q.setdefault(q, set()).update(ruling.get("card_ids") or [])

    out = {}
    for q, card_ids in cards_by_q.items():
        if not card_ids & universe:
            continue
        out[QA_PREFIX + q] = {
            "source": "qa",
            "q_id": q,
            # Every card that prints the ruling, inside the universe or not: the
            # verdict is shared, and the writer files it under each of them.
            "card_ids": sorted(card_ids, key=natural_key),
            "kind": POSITIVE,
            "gating": True,
            "text_sha256": ruling_sha256(rulings[q]),
        }
    return out


def build_denominator(
    card_ids: Iterable[str],
    clauses: Iterable[Mapping],
    card_qa: Mapping,
    promoted: Mapping[str, bool],
    *,
    registry: Mapping[str, tuple[Family, ...]] = FAMILY_VERSIONS,
    universe_desc: Mapping | None = None,
) -> dict:
    """The artifact for `card_ids` (pure function of its arguments)."""
    universe = sorted(set(card_ids), key=natural_key)
    universe_set = set(universe)
    clauses = [c for c in clauses if c.get("card_id") in universe_set]
    text_by_clause = {c["id"]: c.get("text", "") for c in clauses}

    interactions: dict[str, dict] = _qa_interactions(card_qa, universe_set)
    for p in generate_probes(clauses, registry):
        interactions[p.id] = {
            "source": "probe",
            "clause_id": p.clause_id,
            "card_ids": [p.card_id],
            "family": p.family,
            "family_version": p.family_version,
            "kind": p.kind,
            "gating": bool(promoted.get(p.family_version, False)),
            "text_sha256": sha256_text(text_by_clause.get(p.clause_id, "")),
            "why": p.why,
        }

    cards: dict[str, list[str]] = {c: [] for c in universe}
    for iid, rec in interactions.items():
        for c in rec["card_ids"]:
            if c in cards:
                cards[c].append(iid)
    for c, ids in cards.items():
        qa = sorted((i for i in ids if i.startswith(QA_PREFIX)), key=natural_key)
        pr = sorted((i for i in ids if i.startswith(PROBE_PREFIX)), key=natural_key)
        cards[c] = qa + pr

    by_family = Counter(r["family"] for r in interactions.values() if r["source"] == "probe")
    gating = [r for r in interactions.values() if r["gating"]]
    return {
        "version": ARTIFACT_VERSION,
        "families": {
            v: {"promoted": bool(promoted.get(v, False)), "families": [f.name for f in fams]}
            for v, fams in registry.items()
        },
        "universe": dict(universe_desc or {"mode": "all"}),
        "summary": {
            "cards": len(universe),
            "cards_with_interactions": sum(1 for ids in cards.values() if ids),
            "cards_with_gating_interactions": sum(
                1 for ids in cards.values() if any(interactions[i]["gating"] for i in ids)),
            "interactions": len(interactions),
            "gating_interactions": len(gating),
            "by_source": dict(sorted(Counter(r["source"] for r in interactions.values()).items())),
            "by_family": dict(sorted(by_family.items())),
            "negative_probes": sum(1 for r in interactions.values() if r["kind"] == NEGATIVE),
        },
        "cards": cards,
        "interactions": interactions,
    }


def render(artifact: Mapping) -> str:
    """Deterministic text: sorted keys, ASCII, LF, one line per card / interaction
    so a drifted ruling shows up as one changed line in review."""

    def line(v) -> str:
        return json.dumps(v, sort_keys=True, ensure_ascii=True, separators=(", ", ": "))

    out = ["{"]
    keys = sorted(artifact)
    for n, key in enumerate(keys):
        value = artifact[key]
        tail = "," if n < len(keys) - 1 else ""
        if key in ("cards", "interactions") and isinstance(value, dict):
            out.append(f'  "{key}": {{')
            items = sorted(value.items(), key=lambda kv: natural_key(kv[0]))
            for i, (k, v) in enumerate(items):
                comma = "," if i < len(items) - 1 else ""
                out.append(f"    {json.dumps(k)}: {line(v)}{comma}")
            out.append("  }" + tail)
        else:
            out.append(f"  {json.dumps(key)}: {line(value)}{tail}")
    out.append("}")
    return "\n".join(out) + "\n"


def write(artifact: Mapping, path: Path | str) -> None:
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(render(artifact))


def load_denominator(path: Path | str) -> dict:
    p = Path(path)
    if not p.exists():
        raise FileNotFoundError(
            f"interaction denominator not generated: no file at {p}. Build it with "
            "`python -m tools.card_loop interactions build`"
        )
    data = json.loads(p.read_text(encoding="utf-8"))
    if data.get("version") != ARTIFACT_VERSION:
        raise ValueError(f"{p}: unsupported denominator version {data.get('version')!r}")
    return data


# ---------------------------------------------------------------------------
# Drift and promotion
# ---------------------------------------------------------------------------


def diff_denominators(committed: Mapping, fresh: Mapping) -> dict:
    """`{new, removed, changed}` interaction ids, plus card-list changes."""
    old_i, new_i = committed.get("interactions", {}), fresh.get("interactions", {})
    old_c, new_c = committed.get("cards", {}), fresh.get("cards", {})
    return {
        "new": sorted(set(new_i) - set(old_i), key=natural_key),
        "removed": sorted(set(old_i) - set(new_i), key=natural_key),
        "changed": sorted((i for i in set(old_i) & set(new_i) if old_i[i] != new_i[i]), key=natural_key),
        "cards_added": sorted(set(new_c) - set(old_c), key=natural_key),
        "cards_removed": sorted(set(old_c) - set(new_c), key=natural_key),
        "header_changed": sorted(
            k for k in set(committed) | set(fresh)
            if k not in ("cards", "interactions", "summary") and committed.get(k) != fresh.get(k)
        ),
    }


def drift_is_empty(d: Mapping) -> bool:
    return not any(d[k] for k in ("new", "removed", "changed", "cards_added", "cards_removed",
                                  "header_changed"))


def promotion_report(
    artifact: Mapping,
    version: str,
    verdicts: Mapping[str, Mapping] | None = None,
) -> dict:
    """Cards whose gating set would GROW if `version` were promoted.

    Every such card drops out of readiness until the new probes are
    adjudicated, except where a stored verdict already adjudicates them
    (`ADJUDICATED_VERDICTS`) -- those are counted separately.
    """
    fams = artifact.get("families", {})
    if version not in fams:
        raise ValueError(f"unknown family version {version!r}; known: {sorted(fams)}")
    verdicts = verdicts or {}
    growing: dict[str, list[str]] = {}
    already: Counter = Counter()
    for iid, rec in artifact.get("interactions", {}).items():
        if rec.get("family_version") != version or rec.get("gating"):
            continue
        for card in rec["card_ids"]:
            growing.setdefault(card, []).append(iid)
            if str((verdicts.get(iid) or {}).get("verdict", "")).lower() in ADJUDICATED_VERDICTS:
                already[card] += 1
    cards = {c: sorted(ids, key=natural_key) for c, ids in sorted(growing.items(), key=lambda kv: natural_key(kv[0]))}
    would_drop = sorted((c for c, ids in cards.items() if already[c] < len(ids)), key=natural_key)
    return {
        "version": version,
        "promoted": bool(fams[version].get("promoted")),
        "cards_with_new_gating_probes": len(cards),
        "new_gating_probes": sum(len(v) for v in cards.values()),
        "cards_that_would_drop": would_drop,
        "cards": cards,
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _fresh(args, committed: Mapping | None = None) -> dict:
    card_qa = load_card_qa(args.card_qa)
    promoted = load_promotion(args.promotion)
    if args.cards_from:
        card_ids = read_card_ids(args.cards_from)
        # Repo-relative, so the artifact never embeds a machine-specific path.
        src = Path(args.cards_from).resolve()
        try:
            src = src.relative_to(ROOT)
        except ValueError:
            src = Path(src.name)
        universe = {"mode": "cards", "source": src.as_posix()}
    elif committed is not None and (committed.get("universe") or {}).get("mode") == "cards":
        card_ids = sorted(committed.get("cards", {}), key=natural_key)
        universe = dict(committed["universe"])
    else:
        card_ids = all_card_ids(args.cards_json, card_qa)
        universe = {"mode": "all"}
    clauses = extract_clauses(card_ids, cards_json=args.cards_json,
                              overrides=args.overrides, official=args.official)
    return build_denominator(card_ids, clauses, card_qa, promoted, universe_desc=universe)


def _print_ids(label: str, ids: list[str], limit: int = 25) -> None:
    if not ids:
        return
    print(f"  {label} ({len(ids)}):")
    for i in ids[:limit]:
        print(f"    {i}")
    if len(ids) > limit:
        print(f"    ... and {len(ids) - limit} more")


def _summary_line(a: Mapping) -> str:
    s = a["summary"]
    return (f"{s['cards']} cards, {s['interactions']} interactions "
            f"({s['gating_interactions']} gating; by source {s['by_source']}), "
            f"{s['cards_with_gating_interactions']} cards gated by at least one")


def cli(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m tools.card_loop interactions",
        description="Build or --check the gating interaction denominator (design D6).",
    )
    parser.add_argument("command", nargs="?", default="build",
                        choices=("build", "check", "promotion-report"))
    parser.add_argument("--check", action="store_true", help="same as the `check` command")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT,
                        help="the committed artifact (default: %(default)s)")
    parser.add_argument("--card-qa", type=Path, default=DEFAULT_CARD_QA)
    parser.add_argument("--promotion", type=Path, default=DEFAULT_PROMOTION)
    parser.add_argument("--cards-from", type=Path,
                        help="restrict the universe to these card ids (JSON list / {cards} / lines)")
    parser.add_argument("--cards-json", type=Path)
    parser.add_argument("--overrides", type=Path)
    parser.add_argument("--official", type=Path)
    parser.add_argument("--version", dest="family_version",
                        help="promotion-report: the family version (default: every unpromoted one)")
    parser.add_argument("--verdicts", type=Path, default=DEFAULT_VERDICTS,
                        help="promotion-report: exam verdicts, to discount adjudicated probes")
    parser.add_argument("--json", type=Path, help="promotion-report: also write the report here")
    args = parser.parse_args(argv)
    command = "check" if args.check else args.command

    try:
        if command == "build":
            artifact = _fresh(args)
            write(artifact, args.out)
            print(f"interactions: wrote {args.out} -- {_summary_line(artifact)}")
            return 0

        if command == "check":
            try:
                committed = load_denominator(args.out)
            except FileNotFoundError as e:
                print(f"interactions --check: FAIL -- {e}", file=sys.stderr)
                return 1
            fresh = _fresh(args, committed)
            d = diff_denominators(committed, fresh)
            if drift_is_empty(d):
                print(f"interactions --check: OK -- {_summary_line(fresh)}")
                return 0
            print(f"interactions --check: DRIFT -- {args.out} is stale; rebuild with "
                  "`python -m tools.card_loop interactions build` and commit it")
            _print_ids("new interactions", d["new"])
            _print_ids("removed interactions", d["removed"])
            _print_ids("changed interactions", d["changed"])
            _print_ids("cards added", d["cards_added"])
            _print_ids("cards removed", d["cards_removed"])
            _print_ids("header fields changed", d["header_changed"])
            return 1

        # promotion-report
        artifact = load_denominator(args.out)
        versions = ([args.family_version] if args.family_version else
                    [v for v, e in artifact["families"].items() if not e.get("promoted")])
        if not versions:
            print("promotion-report: every family version is already promoted; nothing would change")
            return 0
        from tools.clause_coverage.exam_binding import load_interaction_verdicts

        stored = load_interaction_verdicts(args.verdicts)
        reports = [promotion_report(artifact, v, stored) for v in versions]
        for r in reports:
            print(f"promotion-report {r['version']} (promoted: {r['promoted']}): "
                  f"{r['new_gating_probes']} probes would start gating across "
                  f"{r['cards_with_new_gating_probes']} cards; "
                  f"{len(r['cards_that_would_drop'])} would drop out of readiness")
            _print_ids("cards that would drop", r["cards_that_would_drop"])
        if args.json:
            args.json.write_text(json.dumps(reports, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return 0
    except (FileNotFoundError, ValueError) as e:
        print(f"interactions {command}: error -- {e}", file=sys.stderr)
        return 2
