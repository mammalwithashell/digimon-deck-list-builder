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

    Returns ``{"card": str | None, "clause": str | None}``.
    """
    found: dict[str, str | None] = {"card": None, "clause": None}
    for raw_line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n"):
        line = raw_line.lstrip("﻿")
        if not line or line[0] in (" ", "\t", "#", "-"):
            continue
        key, sep, value = line.partition(":")
        if not sep:
            continue
        key = key.strip()
        if key not in found or found[key] is not None:
            continue
        parsed = _unquote(_strip_inline_comment(value.strip())).strip()
        found[key] = parsed or None
    return found


def load_verdict_store(path: Path | str | None) -> dict:
    """Load the verdict store -> ``{clause_id: entry}``.

    Accepts either the **fleet layout** -- a directory of per-card
    ``<CARD-ID>.json`` files, which is what nodes write, because disjoint
    writers must touch disjoint files -- or a **single file**, which fixtures
    and tests still use.

    A missing path is NOT an error (fresh checkout): it yields an empty store,
    and every clause then honestly reports `unmeasured`.

    In the directory branch only, a row whose ``card_id`` does not match the
    file it was found in is rejected with a ``ValueError`` naming both the
    file and the offending card -- the same check the Rust
    ``VerdictStore::load_dir`` (`code/tools/dcgo-harness/src/exam/verdict.rs`)
    performs, so a misfiled row (a bad hand-edit or a bad merge) cannot be
    silently absorbed into a card's clause list by one reader while the other
    refuses it. The single-file branch is exempt: fixtures and tests load
    arbitrarily-named single files on purpose, and the filename there carries
    no claim about which card the contents belong to.
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
                merged[clause_id] = record
        return merged

    return _load_verdict_file(p)


def _load_verdict_file(p: Path) -> dict:
    """One store file -> ``{clause_id: entry}``. Shape errors yield ``{}`."""
    with open(p, encoding="utf-8") as f:
        data = json.load(f)
    clauses = data.get("clauses") if isinstance(data, dict) else None
    if not isinstance(clauses, dict):
        return {}
    return clauses


def _scenario_files(scenarios_dir: Path | str | None) -> list[Path]:
    if not scenarios_dir:
        return []
    d = Path(scenarios_dir)
    if not d.exists():
        return []
    return sorted(p for p in d.rglob("*.yaml") if p.is_file())


def bind(
    card_ids: list[str],
    scenarios_dir: Path | str | None,
    verdicts_path: Path | str | None,
    *,
    source_desc: str | None = None,
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
    orphan_scenarios: list[dict] = []

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
            }
        )

    total_clauses = len(clauses)
    # Invariant: exactly one class per clause, appended in the single loop above.
    assert sum(by_verdict.values()) == total_clauses

    bound_scenario_count = sum(len(v) for v in scenarios_by_clause.values())

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
        },
        "verdicts": {
            "path": str(verdicts_path) if verdicts_path else None,
            "present": bool(verdicts_path) and Path(verdicts_path).exists(),
            "entries": len(store),
            "invalidated": len(invalidated_clause_ids),
            "unrecognized": len(unrecognized_verdicts),
        },
    }
