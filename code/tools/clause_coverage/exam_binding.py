"""Bind exam scenarios to the clause-coverage denominator.

This is the WORKFLOW-layer join of the DCGO scripted-scenario exam
(`docs/superpowers/specs/2026-08-21-dcgo-scripted-scenario-exam-design.md`):

    extracted clauses   x   authored scenarios   x   stored verdicts
    (the denominator)       (what was authored)      (what was measured)

`bind()` produces a report in which **every** extracted clause appears with
exactly one of the five verdict classes:

    confirmed | diverged | unreachable | unavailable | unmeasured

`unmeasured` is the point. A card must never read as "passed"; it reads as
"8 clauses: 5 confirmed, 1 diverged, 2 unmeasured". `by_verdict` therefore
always sums to `total_clauses` by construction (one class is appended per
clause in a single loop), and the report always carries the full
`unmeasured_clause_ids` list.

Clause identity is NOT invented here: a clause id is
`clause_coverage.models.Clause.id` == ``{card_id}#{zone}#{idx}`` (see
`card_sources.extract_card_clauses`), e.g. ``EX12-073#security#0``.

Three failure modes this module refuses to hide
-----------------------------------------------

1. **Orphan scenarios.** A scenario naming a clause id the extractor does not
   produce (a typo, a stale id after a text re-scrape, a card outside the
   requested scope) would otherwise pass its own assertions while covering
   nothing in the denominator -- an invisible sixth verdict class. Every such
   scenario lands in `orphan_scenarios` with a `kind` and a `reason`; none is
   dropped.
2. **Verdicts whose clause text drifted.** Clause ids are *positional within a
   zone*, so an override or re-scrape that changes a card's text silently
   re-points every later id at a DIFFERENT clause. A stored verdict carrying a
   `text_sha256` that no longer matches the current clause text is invalidated:
   it reports `unmeasured` and its id is listed in `invalidated_clause_ids`.
   (Same rule as the Rust `VerdictStore::get_validated`.)
3. **Unrecognized verdict strings.** A stored value outside the five classes
   degrades to `unmeasured` and is surfaced in `unrecognized_verdicts` rather
   than being coerced into something that reads like a pass.

Verdict-store shape (as written by the Rust `VerdictStore`)::

    {"version": 1, "last_updated": "...",
     "clauses": {"<clause_id>": {"clause_id": ..., "card_id": ...,
                                 "verdict": "confirmed", "text_sha256": ...,
                                 "scenario_path": ..., "reason": ..., ...}}}

**Version 2** (card-loop design D8) adds ``"interactions": {"<interaction_id>":
{"interaction_id", "card_ids", "source", "kind", "verdict", "text_sha256",
...}}`` beside ``clauses``. Both versions load; a file is written as v2 only
when its card carries an interaction verdict (`write_interaction_verdict`), so
the committed v1 files are never rewritten. A shared ruling's verdict is
written into EVERY card file it counts for, and two copies that disagree are
refused on read.

Interaction scenarios (an ``interaction:`` block) bind to
``scenarios.by_interaction``, never to a clause: an id absent from the
committed interaction denominator is an orphan, and with no denominator every
interaction scenario is one (``denominator_not_generated``). ``covers:`` ids
the extractor does not produce are orphans too. `bind_interactions` reports a
card's gating interactions with the same five classes.

A missing verdicts file is NOT an error (fresh checkout): everything reports
`unmeasured`.

Standard library only, matching the rest of `tools/clause_coverage/` -- see
`_parse_scenario_header` for the deliberately tiny YAML front-matter reader.
"""

from __future__ import annotations

import hashlib
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

from tools.clause_coverage.extract import run as extract_run

#: The five verdict classes. `unmeasured` is the default for every clause.
VERDICT_CLASSES: tuple[str, ...] = (
    "confirmed",
    "diverged",
    "unreachable",
    "unavailable",
    "unmeasured",
)

UNMEASURED = "unmeasured"

#: Verdict-store file versions this reader understands (Rust `SUPPORTED_VERSIONS`).
SUPPORTED_STORE_VERSIONS: tuple[int, ...] = (1, 2)

#: Field order the Rust `InteractionVerdict` serializes in; the Python writer
#: emits the same order so both writers produce the same bytes.
_INTERACTION_FIELD_ORDER: tuple[str, ...] = (
    "interaction_id", "card_ids", "source", "kind", "verdict", "text_sha256",
    "scenario_path", "reason", "dcgo_build", "job_id", "recorded_at",
)

#: The committed interaction denominator (`tools.card_loop.interactions`).
DEFAULT_INTERACTION_DENOMINATOR = (
    Path(__file__).resolve().parents[3] / "data" / "interaction_denominator.json"
)


def clause_text_sha256(text: str) -> str:
    """Stable content hash of a clause's printed text.

    Line endings are normalized to ``\\n`` first: the same clause text read on
    Windows and Linux must hash identically, or every verdict would invalidate
    itself on the other platform. A hash MISmatch degrades a verdict to
    `unmeasured` (never to a pass), so the failure direction is safe.
    """
    normalized = (text or "").replace("\r\n", "\n").replace("\r", "\n")
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()


def _strip_inline_comment(value: str) -> str:
    """Drop a YAML trailing comment, honoring YAML's rule that ``#`` only
    starts a comment when it is at the start of the line or preceded by
    whitespace.

    This matters here more than usual: a clause id CONTAINS ``#``
    (``EX12-073#effect#0``). A naive ``value.split("#")[0]`` would truncate
    every clause id to a bare card id and turn the whole binding into nonsense.
    """
    if value.startswith("#"):
        return ""
    out = value
    for marker in (" #", "\t#"):
        idx = out.find(marker)
        if idx != -1:
            out = out[:idx]
    return out.strip()


def _unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _parse_scenario_header(text: str) -> dict:
    """Read the top-level scalar keys ``card:`` and ``clause:`` out of a
    scenario YAML, WITHOUT a YAML dependency.

    `tools/clause_coverage/` is standard-library only, and the two keys this
    binding needs are top-level scalars written by our own drafter/authors --
    so a targeted reader is preferable to adding pyyaml to the package's
    dependency surface. The scope is deliberately narrow:

    - a key must start at column 0 (so a nested ``clause:`` inside ``steps:``
      cannot be mistaken for the document's own),
    - the first occurrence of each key wins,
    - surrounding quotes are stripped, trailing comments removed per the
      whitespace rule above.

    Anything else in the file (nested maps, flow sequences, block scalars) is
    ignored -- this reads a header, it does not validate a scenario. Full
    scenario validation is the Rust ``Scenario::validate``'s job. A scenario
    whose header this reader cannot find is reported as an orphan, never
    silently skipped, so the narrow scope cannot quietly shrink coverage.

    Two non-scalar keys are read as well, in the forms our authors write:
    ``covers:`` (a flow list ``[a, b]`` or a block list of ``- a`` lines) and
    ``interaction:`` (a flow map ``{ id: ..., source: ..., kind: ... }`` or a
    block map of indented ``key: value`` lines). A form this reader does not
    understand yields an EMPTY list / map rather than nothing, so the caller
    reports the scenario instead of mistaking it for a legacy one.

    Returns ``{"card": str | None, "clause": str | None,
    "covers": list[str] | None, "interaction": dict | None}``.
    """
    found: dict = {"card": None, "clause": None, "covers": None, "interaction": None}
    lines = [ln.lstrip("﻿") for ln in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    for n, line in enumerate(lines):
        if not line or line[0] in (" ", "\t", "#", "-"):
            continue
        key, sep, value = line.partition(":")
        if not sep:
            continue
        key = key.strip()
        if key not in found or found[key] is not None:
            continue
        value = _strip_inline_comment(value.strip())
        if key == "covers":
            found[key] = _read_list(value, lines[n + 1:])
        elif key == "interaction":
            found[key] = _read_map(value, lines[n + 1:])
        else:
            parsed = _unquote(value).strip()
            found[key] = parsed or None
    return found


def _indented_block(following: list[str]) -> list[str]:
    """The indented lines directly under a top-level key."""
    block = []
    for ln in following:
        if ln and ln[0] not in (" ", "\t"):
            break
        if ln.strip() and not ln.strip().startswith("#"):
            block.append(ln.strip())
    return block


def _read_list(value: str, following: list[str]) -> list[str]:
    if value.startswith("["):
        inner = value[1:value.rfind("]")] if "]" in value else ""
        return [_unquote(v.strip()) for v in inner.split(",") if v.strip()]
    if value:
        return []  # a scalar where a list belongs: reported by the caller
    return [
        _unquote(_strip_inline_comment(ln[1:].strip()))
        for ln in _indented_block(following)
        if ln.startswith("-")
    ]


def _read_map(value: str, following: list[str]) -> dict:
    if value.startswith("{"):
        inner = value[1:value.rfind("}")] if "}" in value else ""
        pairs = inner.split(",")
    elif value:
        return {}
    else:
        pairs = [_strip_inline_comment(ln) for ln in _indented_block(following)]
    out = {}
    for pair in pairs:
        k, sep, v = pair.partition(":")
        if sep and k.strip():
            out[k.strip()] = _unquote(v.strip())
    return out


def load_verdict_store(path: Path | str | None) -> dict:
    """Load the verdict store -> ``{clause_id: entry}``.

    Accepts either the **fleet layout** -- a directory of per-card
    ``<CARD-ID>.json`` files, which is what nodes write, because disjoint
    writers must touch disjoint files -- or a **single file**, which fixtures
    and tests still use.

    A missing path is NOT an error (fresh checkout): it yields an empty store,
    and every clause then honestly reports `unmeasured`.

    In the directory branch only, two checks mirror the Rust
    ``VerdictStore::load_dir`` / ``VerdictStore::from_json``
    (`code/tools/dcgo-harness/src/exam/verdict.rs`), so a misfiled or
    mismatched row cannot be silently absorbed into a card's clause list by
    one reader while the other refuses it:

    1. A row whose ``card_id`` does not match the file it was found in is
       rejected with a ``ValueError`` naming both the file and the offending
       card.
    2. A row whose embedded ``clause_id`` disagrees with the dict key it is
       filed under is rejected with a ``ValueError`` naming the file, the
       key, and the disagreeing ``clause_id`` -- an empty/missing embedded
       ``clause_id`` is tolerated (the key is treated as authoritative),
       matching the Rust reader's tolerance.

    The single-file branch is exempt from both checks: fixtures and tests
    load arbitrarily-named single files on purpose, and the filename there
    carries no claim about which card the contents belong to.
    """
    if not path:
        return {}
    p = Path(path)
    if not p.exists():
        return {}

    if p.is_dir():
        merged: dict = {}
        for f in sorted(p.glob("*.json")):
            expected_card = f.stem
            for clause_id, record in _load_verdict_file(f).items():
                actual_card = record.get("card_id")
                if actual_card != expected_card:
                    raise ValueError(
                        f"verdict file {f} holds a verdict for card {actual_card!r} "
                        f"(clause {clause_id!r}); each file holds exactly one card's "
                        "verdicts"
                    )
                embedded_clause_id = record.get("clause_id")
                if embedded_clause_id and embedded_clause_id != clause_id:
                    raise ValueError(
                        f"verdict file {f} key {clause_id!r} disagrees with its "
                        f"clause_id {embedded_clause_id!r}"
                    )
                merged[clause_id] = record
        return merged

    return _load_verdict_file(p)


def _read_store_file(p: Path) -> dict:
    """One store file, version-checked. Accepts v1 and v2 (the Rust
    `VerdictStore::from_json` rules): an unknown version, or a v1 file carrying
    `interactions`, raises rather than being half-read."""
    with open(p, encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        return {}
    version = data.get("version", 1)
    if version not in SUPPORTED_STORE_VERSIONS:
        raise ValueError(
            f"verdict file {p} is version {version!r}; this reader understands "
            f"{list(SUPPORTED_STORE_VERSIONS)}"
        )
    if version == 1 and data.get("interactions"):
        raise ValueError(f"verdict file {p} is version 1 but carries `interactions` (that is v2)")
    return data


def _load_verdict_file(p: Path) -> dict:
    """One store file -> ``{clause_id: entry}``. Shape errors yield ``{}``."""
    clauses = _read_store_file(p).get("clauses")
    if not isinstance(clauses, dict):
        return {}
    return clauses


def load_interaction_verdicts(path: Path | str | None) -> dict:
    """Load the v2 ``interactions`` maps -> ``{interaction_id: entry}``.

    Same layouts as `load_verdict_store` (a directory of per-card files, or
    one file). In the directory branch, mirroring the Rust ``load_dir``:

    1. a row must list the file's card in ``card_ids`` (a card's file only
       holds interactions that count for it);
    2. an embedded ``interaction_id`` must agree with its key;
    3. the copies of one shared interaction in several card files must be
       identical -- they are written together, so a difference is a partial
       write or a bad merge, and picking one would hide it.

    A missing path is an empty store; v1 files simply contribute nothing.
    """
    if not path:
        return {}
    p = Path(path)
    if not p.exists():
        return {}
    if not p.is_dir():
        return dict(_read_store_file(p).get("interactions") or {})

    merged: dict = {}
    origin: dict[str, Path] = {}
    for f in sorted(p.glob("*.json")):
        card = f.stem
        for iid, record in (_read_store_file(f).get("interactions") or {}).items():
            if card not in (record.get("card_ids") or []):
                raise ValueError(
                    f"verdict file {f} holds interaction {iid!r}, which counts for "
                    f"{record.get('card_ids')!r} but not for {card!r}"
                )
            embedded = record.get("interaction_id")
            if embedded and embedded != iid:
                raise ValueError(f"verdict file {f} key {iid!r} disagrees with its interaction_id {embedded!r}")
            if iid in merged:
                if merged[iid] != record:
                    raise ValueError(
                        f"interaction {iid!r} has disagreeing copies in {origin[iid]} and {f}; a "
                        "shared verdict must be written identically into every listed card's file"
                    )
                continue
            merged[iid] = record
            origin[iid] = f
    return merged


def _ordered_interaction(record: dict) -> dict:
    head = {k: record[k] for k in _INTERACTION_FIELD_ORDER if k in record and record[k] is not None}
    extra = {k: record[k] for k in sorted(record) if k not in _INTERACTION_FIELD_ORDER}
    return {**head, **extra}


def write_interaction_verdict(verdicts_dir: Path | str, record: dict) -> list[Path]:
    """Write one (possibly shared) interaction verdict into EVERY card file it
    counts for, leaving each file's other verdicts untouched.

    The record carries the Rust `InteractionVerdict` fields -- required:
    ``interaction_id``, ``card_ids`` (non-empty), ``source``, ``kind``,
    ``verdict``, ``text_sha256`` (the denominator's fingerprint; ``""`` for a
    combo) and ``recorded_at``; anything else (``reason``, a triage block,
    ``produced_by``) is carried as-is. Each touched file becomes
    ``version: 2``; files of cards the interaction does not count for are
    never opened. Returns the paths written, so the caller commits them
    together (design D8).
    """
    missing = [k for k in ("interaction_id", "source", "kind", "text_sha256", "recorded_at")
               if record.get(k) is None or (k != "text_sha256" and not record.get(k))]
    if missing:
        raise ValueError(f"an interaction verdict needs {missing}")
    iid = record["interaction_id"]
    cards = record.get("card_ids") or []
    if not cards:
        raise ValueError(f"interaction verdict {iid!r} names no card to file it under")
    verdict = str(record.get("verdict", "")).lower()
    if verdict not in VERDICT_CLASSES or verdict == UNMEASURED:
        raise ValueError(f"interaction verdict {verdict!r} is not one of the four recordable classes")
    row = _ordered_interaction({**record, "card_ids": list(cards), "verdict": verdict})

    d = Path(verdicts_dir)
    d.mkdir(parents=True, exist_ok=True)
    written = []
    for card in cards:
        f = d / f"{card}.json"
        data = _read_store_file(f) if f.exists() else {}
        clauses = data.get("clauses") or {}
        interactions = dict(data.get("interactions") or {})
        interactions[iid] = row
        stamps = [r.get("recorded_at") or "" for r in clauses.values()]
        stamps += [r.get("recorded_at") or "" for r in interactions.values()]
        out = {
            "version": 2,
            "last_updated": max(stamps),
            "clauses": clauses,
            "interactions": {k: interactions[k] for k in sorted(interactions)},
        }
        with open(f, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
        written.append(f)
    return written


def _scenario_files(scenarios_dir: Path | str | None) -> list[Path]:
    if not scenarios_dir:
        return []
    d = Path(scenarios_dir)
    if not d.exists():
        return []
    return sorted(p for p in d.rglob("*.yaml") if p.is_file())


def load_interaction_denominator(path: Path | str | None) -> dict | None:
    """The committed interaction denominator's ``interactions`` map, or
    ``None`` when it was never generated. Read directly (not through
    `tools.card_loop`) so this package keeps no dependency on the loop."""
    if not path:
        return None
    p = Path(path)
    if not p.exists():
        return None
    data = json.loads(p.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or data.get("version") != 1:
        raise ValueError(f"{p} is not a version-1 interaction denominator")
    return data


def _bind_interaction_scenario(
    path_str: str,
    header: dict,
    denominator: dict | None,
    scenarios_by_interaction: dict[str, list[str]],
    orphan_scenarios: list[dict],
) -> None:
    block = header["interaction"] or {}
    iid = block.get("id") or ""
    source = block.get("source") or ""

    def orphan(kind: str, reason: str) -> None:
        orphan_scenarios.append({
            "path": path_str, "card": header["card"], "clause": header["clause"],
            "interaction": iid or None, "kind": kind, "reason": reason,
        })

    if not iid or source not in ("qa", "probe", "combo") or not iid.startswith(f"{source}:"):
        orphan("malformed_interaction",
               f"interaction block {block!r} needs an `id` carrying its `source:` prefix")
        return
    if source == "combo":
        # Model-authored combos never gate; they are examined, not denominated.
        scenarios_by_interaction.setdefault(iid, []).append(path_str)
        return
    if denominator is None:
        orphan("denominator_not_generated",
               f"interaction {iid!r} cannot be bound: the interaction denominator was not "
               "generated (`python -m tools.card_loop interactions build`)")
        return
    if iid not in denominator.get("interactions", {}):
        orphan("orphan_interaction",
               f"interaction {iid!r} is not in the interaction denominator -- it covers "
               "nothing that gates any card")
        return
    scenarios_by_interaction.setdefault(iid, []).append(path_str)


def bind(
    card_ids: list[str],
    scenarios_dir: Path | str | None,
    verdicts_path: Path | str | None,
    *,
    source_desc: str | None = None,
    interaction_denominator: Path | str | None = DEFAULT_INTERACTION_DENOMINATOR,
    **extract_kwargs,
) -> dict:
    """Join the clause denominator, the authored scenarios, and the verdicts.

    Returns::

        {"generated_at": ..., "cards": {card_id: {...}},
         "denominator": {"total_clauses": N, "total_cards": M,
                         "by_verdict": {<all five classes>}, "by_zone": {...}},
         "unmeasured_clause_ids": [...], "invalidated_clause_ids": [...],
         "orphan_scenarios": [...], "unrecognized_verdicts": [...],
         "scenarios": {...}, "verdicts": {...}}

    `denominator.by_verdict` always contains all five classes and always sums
    to `total_clauses`.
    """
    seen: set[str] = set()
    ordered_cards = [c for c in card_ids if not (c in seen or seen.add(c))]

    extracted = extract_run(
        ordered_cards,
        source_desc or f"exam_binding.bind ({len(ordered_cards)} cards)",
        **extract_kwargs,
    )
    clauses = extracted["clauses"]
    clauses_by_id = {c["id"]: c for c in clauses}
    in_scope = set(ordered_cards)

    # --- scenarios -------------------------------------------------------
    scenario_paths = _scenario_files(scenarios_dir)
    scenarios_by_clause: dict[str, list[str]] = {}
    scenarios_by_interaction: dict[str, list[str]] = {}
    orphan_scenarios: list[dict] = []
    denominator = None  # loaded on the first interaction scenario only

    for path in scenario_paths:
        path_str = str(path)
        try:
            header = _parse_scenario_header(path.read_text(encoding="utf-8"))
        except OSError as exc:  # unreadable file is a finding, not a silent skip
            orphan_scenarios.append(
                {
                    "path": path_str,
                    "card": None,
                    "clause": None,
                    "kind": "unreadable",
                    "reason": f"could not read scenario file: {exc}",
                }
            )
            continue

        card = header["card"]
        clause_id = header["clause"]

        if not clause_id:
            orphan_scenarios.append(
                {
                    "path": path_str,
                    "card": card,
                    "clause": None,
                    "kind": "missing_key",
                    "reason": (
                        "no top-level 'clause:' key -- this scenario covers "
                        "nothing in the denominator"
                    ),
                }
            )
            continue

        # `covers:` -- every extra covered clause must be one the extractor
        # produces (when its card is in scope); judged like `clause:` below.
        covers = header["covers"]
        extra_covered: list[str] = []
        if covers is not None:
            if not covers or clause_id not in covers:
                orphan_scenarios.append({
                    "path": path_str, "card": card, "clause": clause_id,
                    "kind": "malformed_covers",
                    "reason": "`covers:` must be a non-empty list that includes `clause:`",
                })
            for cid in covers:
                if cid == clause_id:
                    continue
                if cid in clauses_by_id:
                    extra_covered.append(cid)
                elif cid.split("#", 1)[0] in in_scope:
                    orphan_scenarios.append({
                        "path": path_str, "card": card, "clause": cid,
                        "kind": "unknown_covered_clause",
                        "reason": f"covered clause {cid!r} is not produced by the extractor -- "
                                  "it covers NOTHING in the denominator",
                    })

        # An interaction exam binds to its interaction, never to a clause: a
        # passing negative probe proves a clause did NOT fire.
        if header["interaction"] is not None:
            if denominator is None:
                denominator = load_interaction_denominator(interaction_denominator) or {}
            _bind_interaction_scenario(
                path_str, header, denominator or None, scenarios_by_interaction, orphan_scenarios
            )
            continue

        for cid in extra_covered:
            scenarios_by_clause.setdefault(cid, []).append(path_str)

        if clause_id in clauses_by_id:
            declared = clauses_by_id[clause_id]["card_id"]
            if card and card != declared:
                # Binds fine, but the header contradicts itself; say so.
                orphan_scenarios.append(
                    {
                        "path": path_str,
                        "card": card,
                        "clause": clause_id,
                        "kind": "card_clause_mismatch",
                        "reason": (
                            f"'card: {card}' disagrees with the clause id's card {declared!r}"
                        ),
                    }
                )
            scenarios_by_clause.setdefault(clause_id, []).append(path_str)
            continue

        clause_card = clause_id.split("#", 1)[0]
        if card is not None and card not in in_scope and clause_card not in in_scope:
            kind = "out_of_scope_card"
            reason = (
                f"card {card!r} is not among the {len(in_scope)} cards this bind was "
                "asked about -- not counted against this denominator"
            )
        else:
            kind = "unknown_clause_id"
            reason = (
                f"clause id {clause_id!r} is not produced by the extractor for "
                f"{clause_card!r} -- it covers NOTHING in the denominator"
            )
        orphan_scenarios.append(
            {
                "path": path_str,
                "card": card,
                "clause": clause_id,
                "kind": kind,
                "reason": reason,
            }
        )

    # --- verdicts --------------------------------------------------------
    store = load_verdict_store(verdicts_path)
    unrecognized_verdicts: list[dict] = []
    invalidated_clause_ids: list[str] = []
    unmeasured_clause_ids: list[str] = []
    by_verdict: Counter = Counter({v: 0 for v in VERDICT_CLASSES})

    def _empty_card(cid: str) -> dict:
        return {
            "card_id": cid,
            "total_clauses": 0,
            "by_verdict": {v: 0 for v in VERDICT_CLASSES},
            "clauses": [],
        }

    # Pre-seed every requested card so a card with ZERO extracted clauses still
    # appears in the report rather than vanishing from it.
    cards_report: dict[str, dict] = {cid: _empty_card(cid) for cid in ordered_cards}

    for clause in clauses:
        clause_id = clause["id"]
        entry = store.get(clause_id) or {}
        raw_verdict = entry.get("verdict")
        verdict = str(raw_verdict).strip().lower() if raw_verdict is not None else UNMEASURED
        invalidated = False
        reason = entry.get("reason")

        if raw_verdict is not None and verdict not in VERDICT_CLASSES:
            unrecognized_verdicts.append({"clause_id": clause_id, "stored_verdict": raw_verdict})
            verdict = UNMEASURED
            reason = f"stored verdict {raw_verdict!r} is not one of {list(VERDICT_CLASSES)}"
        elif verdict != UNMEASURED:
            stored_sha = entry.get("text_sha256")
            if stored_sha and stored_sha != clause_text_sha256(clause.get("text", "")):
                invalidated = True
                invalidated_clause_ids.append(clause_id)
                reason = (
                    f"stored {verdict!r} verdict invalidated: the clause text changed "
                    "since it was recorded (text_sha256 mismatch), so this positional "
                    "id may now point at a different clause"
                )
                verdict = UNMEASURED

        if verdict == UNMEASURED:
            unmeasured_clause_ids.append(clause_id)

        by_verdict[verdict] += 1

        bucket = cards_report.setdefault(clause["card_id"], _empty_card(clause["card_id"]))
        bucket["total_clauses"] += 1
        bucket["by_verdict"][verdict] += 1
        bucket["clauses"].append(
            {
                "clause_id": clause_id,
                "zone": clause["zone"],
                "label": clause.get("label", ""),
                "kind": clause.get("kind", "untimed"),
                "text": clause.get("text", ""),
                "source": clause.get("source"),
                "verdict": verdict,
                "invalidated": invalidated,
                "reason": reason,
                "scenarios": scenarios_by_clause.get(clause_id, []),
                "recorded_at": entry.get("recorded_at"),
                "dcgo_build": entry.get("dcgo_build"),
                "job_id": entry.get("job_id"),
                # Triage adjudicates a divergence; only the FINAL verdict counts, so
                # a row downgraded to `unmeasured` (text drift, unrecognized stored
                # verdict) must not carry a stale adjudication.
                "triage": entry.get("triage") if verdict == "diverged" else None,
                "citation": entry.get("citation") if verdict == "diverged" else None,
            }
        )

    total_clauses = len(clauses)
    # Invariant: exactly one class per clause, appended in the single loop above.
    assert sum(by_verdict.values()) == total_clauses

    # Distinct files: a `covers:` scenario binds under several clauses but is
    # still one scenario.
    bound_scenario_count = len(
        {p for v in scenarios_by_clause.values() for p in v}
        | {p for v in scenarios_by_interaction.values() for p in v}
    )

    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "cards": cards_report,
        "denominator": {
            "total_clauses": total_clauses,
            "total_cards": len(ordered_cards),
            "by_verdict": {v: by_verdict[v] for v in VERDICT_CLASSES},
            "by_zone": dict(Counter(c["zone"] for c in clauses)),
        },
        "unmeasured_clause_ids": unmeasured_clause_ids,
        "invalidated_clause_ids": invalidated_clause_ids,
        "orphan_scenarios": orphan_scenarios,
        "unrecognized_verdicts": unrecognized_verdicts,
        "scenarios": {
            "dir": str(scenarios_dir) if scenarios_dir else None,
            "files_found": len(scenario_paths),
            "bound": bound_scenario_count,
            "orphaned": len(orphan_scenarios),
            "clauses_with_a_scenario": len(scenarios_by_clause),
            "by_clause": scenarios_by_clause,
            "interactions_with_a_scenario": len(scenarios_by_interaction),
            "by_interaction": scenarios_by_interaction,
        },
        "verdicts": {
            "path": str(verdicts_path) if verdicts_path else None,
            "present": bool(verdicts_path) and Path(verdicts_path).exists(),
            "entries": len(store),
            "invalidated": len(invalidated_clause_ids),
            "unrecognized": len(unrecognized_verdicts),
        },
    }


def bind_interactions(
    card_ids: list[str],
    denominator_path: Path | str | None,
    verdicts_path: Path | str | None,
    *,
    gating_only: bool = True,
) -> dict:
    """Every card's (gating) interactions x the stored interaction verdicts.

    The interaction analogue of `bind`'s verdict half -- the join readiness
    needs (design D9). Every interaction lands in exactly one of the five
    classes; a stored verdict whose ``text_sha256`` no longer matches the
    denominator's fingerprint (an edited ruling, a reworded clause) is
    invalidated to `unmeasured`. A shared ruling is ONE interaction: it counts
    once in ``denominator`` and once under each card that prints it.

    Raises ``FileNotFoundError`` when the denominator was never generated --
    reporting "0 interactions" would read as every card being clear.
    """
    denominator = load_interaction_denominator(denominator_path)
    if denominator is None:
        raise FileNotFoundError(
            f"interaction denominator not generated: no file at {denominator_path} "
            "(`python -m tools.card_loop interactions build`)"
        )
    rows = denominator.get("interactions", {})
    per_card_ids = denominator.get("cards", {})
    store = load_interaction_verdicts(verdicts_path)

    classified: dict[str, dict] = {}
    unrecognized: list[dict] = []

    def classify(iid: str) -> dict:
        if iid in classified:
            return classified[iid]
        entry = store.get(iid) or {}
        raw = entry.get("verdict")
        verdict = str(raw).strip().lower() if raw is not None else UNMEASURED
        invalidated = False
        reason = entry.get("reason")
        if raw is not None and verdict not in VERDICT_CLASSES:
            unrecognized.append({"interaction_id": iid, "stored_verdict": raw})
            verdict, reason = UNMEASURED, f"stored verdict {raw!r} is not one of {list(VERDICT_CLASSES)}"
        elif verdict != UNMEASURED and entry.get("text_sha256") != rows[iid].get("text_sha256"):
            invalidated, verdict = True, UNMEASURED
            reason = ("stored verdict invalidated: the ruling / clause text changed since it "
                      "was recorded (text_sha256 mismatch)")
        classified[iid] = {
            "interaction_id": iid,
            "source": rows[iid].get("source"),
            "kind": rows[iid].get("kind"),
            "gating": bool(rows[iid].get("gating")),
            "verdict": verdict,
            "invalidated": invalidated,
            "reason": reason,
            # The clause row's twins (`verdict-set --triage/--citation` writes
            # them flat on the interaction record); readiness' D9 rule reads them.
            "triage": entry.get("triage") if verdict == "diverged" else None,
            "citation": entry.get("citation") if verdict == "diverged" else None,
            "scenario_path": entry.get("scenario_path"),
        }
        return classified[iid]

    cards: dict[str, dict] = {}
    seen: set[str] = set()
    for cid in dict.fromkeys(card_ids):
        ids = [i for i in per_card_ids.get(cid, []) if not gating_only or rows[i].get("gating")]
        items = [classify(i) for i in ids]
        seen.update(ids)
        cards[cid] = {
            "card_id": cid,
            "in_denominator": cid in per_card_ids,
            "total_interactions": len(items),
            "by_verdict": {v: sum(1 for x in items if x["verdict"] == v) for v in VERDICT_CLASSES},
            "interactions": items,
        }

    distinct = [classified[i] for i in sorted(seen)]
    by_verdict = Counter({v: 0 for v in VERDICT_CLASSES})
    by_verdict.update(x["verdict"] for x in distinct)
    assert sum(by_verdict.values()) == len(distinct)
    return {
        "cards": cards,
        "denominator": {
            "total_interactions": len(distinct),
            "by_verdict": {v: by_verdict[v] for v in VERDICT_CLASSES},
            "gating_only": gating_only,
        },
        "cards_missing_from_denominator": [c for c, r in cards.items() if not r["in_denominator"]],
        "unmeasured_interaction_ids": [x["interaction_id"] for x in distinct if x["verdict"] == UNMEASURED],
        "invalidated_interaction_ids": [x["interaction_id"] for x in distinct if x["invalidated"]],
        "unrecognized_verdicts": unrecognized,
    }
