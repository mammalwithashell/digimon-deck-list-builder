"""Three-way comparison for Q&A interactions (design D7) and the DCGO
fork-candidate export.

A Q&A exam has three oracles: our engine, DCGO, and the publisher's ruling
(the scenario's ``expect_ruling:`` block). The exam measures two legs -- ours
vs DCGO (the differ) and ours vs the ruling (`exam::assertions::check_ruling`)
-- and the outcome is a pure function of them:

=========  ============  ====================================================
ours=DCGO  ours=ruling   outcome
=========  ============  ====================================================
yes        yes           ``confirmed``
yes        no            ``ours_wrong`` (fix gate); DCGO's divergence from the
                         ruling is logged -> DCGO fork candidate
no         yes           ``dcgo_quirk`` citing ``qa:<Q>`` -- still a
                         terminating call, two-family checked (D5) -> fork
                         candidate
no         no            ``ours_wrong``
=========  ============  ====================================================

Parity with DCGO is the aspiration, so every row where DCGO disagrees with the
ruling (and every triaged ``dcgo_quirk``) is exported as a candidate patch to
our DCGO fork. This module only produces the list; the mod work is separate
and needs a Unity rebuild.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Mapping

from tools.card_loop.interactions import QA_PREFIX
from tools.card_loop.interactions.probes import natural_key

CONFIRMED = "confirmed"
OURS_WRONG = "ours_wrong"
DCGO_QUIRK = "dcgo_quirk"


@dataclass(frozen=True)
class ThreeWay:
    verdict_hint: str              # confirmed | ours_wrong | dcgo_quirk
    citation_kind: str | None      # "qa" (the ruling is the citation) or None
    dcgo_fork_candidate: bool
    #: Ends the item without our engine changing -> both families must agree (D5).
    terminating: bool
    #: What the two legs imply about DCGO vs the ruling: True / False, or None
    #: when they cannot say (ours matches neither: DCGO may or may not match).
    dcgo_matches_ruling: bool | None

    def to_dict(self) -> dict:
        return asdict(self)


_TABLE: dict[tuple[bool, bool], ThreeWay] = {
    (True, True): ThreeWay(CONFIRMED, None, False, False, True),
    (True, False): ThreeWay(OURS_WRONG, "qa", True, False, False),
    (False, True): ThreeWay(DCGO_QUIRK, "qa", True, True, False),
    (False, False): ThreeWay(OURS_WRONG, "qa", False, False, None),
}


def three_way(ours_vs_dcgo_agree: bool, ours_vs_ruling_agree: bool) -> ThreeWay:
    """The D7 table. Both arguments must be real booleans: an unmeasured leg
    is not a disagreement, so the caller must not coerce `None` into one."""
    if not isinstance(ours_vs_dcgo_agree, bool) or not isinstance(ours_vs_ruling_agree, bool):
        raise TypeError(
            "three_way needs both legs measured (bool); an unmeasured leg leaves the "
            "interaction unmeasured, not decided"
        )
    return _TABLE[(ours_vs_dcgo_agree, ours_vs_ruling_agree)]


def citation(interaction_id: str, outcome: ThreeWay) -> str | None:
    """`qa:<Q>` for a Q&A interaction whose outcome cites the ruling."""
    if outcome.citation_kind == "qa" and interaction_id.startswith(QA_PREFIX):
        return interaction_id
    return None


def fork_candidates(results: Iterable[Mapping]) -> list[dict]:
    """DCGO fork-candidate export rows, sorted by interaction id.

    Each input row describes one examined interaction::

        interaction_id   qa:<Q> or a probe id
        card_ids         every card it counts for
        ours_vs_dcgo_agree, ours_vs_ruling_agree   (Q&A exams: both legs)
          -- or --
        verdict_hint     an already-triaged outcome (e.g. a probe triaged
                         `dcgo_quirk` with a rules citation)
        citation         optional; defaults to the interaction id for qa:
        dcgo_observed    what DCGO did (the differ's lead divergence, or the
                         ruling assertion DCGO's state fails)
        expected         what the ruling / rules say should happen
        scenario_path    the exam that showed it

    A row becomes a candidate when DCGO is shown to disagree with the ruling
    (D7 rows 2 and 3) or was triaged `dcgo_quirk`. Rows that cannot say what
    DCGO did versus the ruling are left out rather than guessed.
    """
    out = []
    for r in results:
        iid = r["interaction_id"]
        if "verdict_hint" in r and r.get("ours_vs_dcgo_agree") is None:
            hint = r["verdict_hint"]
            if hint != DCGO_QUIRK:
                continue
            reason = "triaged dcgo_quirk"
            cite = r.get("citation")
        else:
            o = three_way(r["ours_vs_dcgo_agree"], r["ours_vs_ruling_agree"])
            if not o.dcgo_fork_candidate:
                continue
            hint = o.verdict_hint
            reason = ("DCGO contradicts the ruling; our engine follows it" if hint == DCGO_QUIRK
                      else "both engines contradict the ruling; our fix restores it, DCGO keeps it")
            cite = r.get("citation") or citation(iid, o)
        if not cite:
            raise ValueError(f"fork candidate {iid} has no citation (qa:<Q>, PDF section, or DCGO file:line)")
        out.append({
            "interaction_id": iid,
            "card_ids": sorted(r.get("card_ids") or [], key=natural_key),
            "source": "qa" if iid.startswith(QA_PREFIX) else "probe",
            "outcome": hint,
            "citation": cite,
            "reason": reason,
            "dcgo_observed": r.get("dcgo_observed"),
            "expected": r.get("expected"),
            "scenario_path": r.get("scenario_path"),
        })
    return sorted(out, key=lambda row: natural_key(row["interaction_id"]))


def write_fork_candidates(rows: list[dict], path: Path | str) -> None:
    """Deterministic JSON (sorted keys, LF) for the DCGO-fork backlog."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps({"version": 1, "candidates": rows}, indent=2, sort_keys=True) + "\n")
