"""Guard: an admissible DSL card authors every printed clause.

Silphymon (BT24-037) was admissible for training while its printed
"[All Turns] [Once Per Turn] When this Digimon would leave the battle area
other than by your effects, you may play ..." clause was missing from its
YAML twice over (effect box and inherited box). A scan of the admissible pool
found more cards with whole printed boxes unauthored. This guard runs the
printed-clause denominator (``tools.clause_coverage.extract``, official bundle
first) against every admissible DSL card's YAML through
``tools.clause_coverage.authored_match`` and fails on a printed clause with no
authored counterpart.

A flagged card is fixed one of two ways:
  * author the missing clause (with its behavioural test), or
  * record a failed verdict (``PARTIAL``) for it in
    ``qa/qa-reports/validated_cards_dsl.json`` and regenerate
    ``data/card_admissibility.json`` so the training gate drops it
    (CLAUDE.md rule 34).

The matcher is a heuristic. A card it flags although every printed clause is
authored goes in ``REVIEWED_AUTHORED`` with the reason, after a person read the
card face / bundle and the YAML.
"""

from __future__ import annotations

import functools
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "code"))

from tools.clause_coverage.authored_match import load_specs, unmatched_clauses  # noqa: E402
from tools.clause_coverage.extract import run as extract_run  # noqa: E402

# card -> (zone, printed-text prefix, why that flagged printed clause IS
# authored), reviewed 2026-11-22. Each entry exempts exactly the one clause it
# names, so a later regression on another clause of the same card still fails.
REVIEWED_AUTHORED: dict[str, tuple[str, str, str]] = {
    "BT1-087": (
        "inherited",
        "Play this card without paying its memory cost.",
        "Tamer: the bundle files its [Security] play under both Inherited and "
        "Security; the face prints it once (security box), authored `on_security`.",
    ),
    "BT13-007": (
        "inherited",
        "When an Option card with the [Royal Knight] trait is placed in the battle area",
        "Inherited [Breeding] Royal Knight Option memory clause is authored "
        "`scope: face_up` + in_breeding on purpose (see the YAML comment: breeding "
        "dispatch skips inherited-flagged effects).",
    ),
    "BT20-083": (
        "security",
        "When your security stack is removed from, by suspending this Digimon",
        "The bundle repeats the {Breeding} inherited clause under Security; "
        "authored once, `scope: inherited` on_own_security_removed.",
    ),
    "EX8-048": (
        "security",
        "When effects trash this card from a [Mineral] or [Rock] trait Digimon",
        "Card face prints one inherited box; the bundle repeats it under "
        "Security; authored `scope: inherited` on_digivolution_card_trashed.",
    ),
    "P-247": (
        "effect",
        "By trashing 1 [Dark Animal], [Shaman], [Undead] or [TS] trait card",
        "Digi-Egg with no bundle: cards.json splits its inherited "
        "[When Attacking] box into effect text; authored `scope: inherited`.",
    ),
    "ST3-04": (
        "effect",
        "When an opponent's Digimon is deleted by dropping to 0 DP, gain 1 memory.",
        "Card face prints the clause only in the inherited box; the bundle "
        "repeats it as Effect; authored `scope: inherited`.",
    ),
    "EX12-059": (
        "inherited",
        "Ace Overflow",
        "ACE Overflow <-4> is authored via card-level `ace_overflow: -4` metadata "
        "in EX12-059.yaml and verified in ex12_059.rs.",
    ),
}


def _is_reviewed(card_id: str, clause: dict) -> bool:
    entry = REVIEWED_AUTHORED.get(card_id)
    return bool(entry) and clause["zone"] == entry[0] and clause["text"].startswith(entry[1])


def _admissible_dsl_cards() -> list[str]:
    adm = json.loads((ROOT / "data" / "card_admissibility.json").read_text(encoding="utf-8"))
    return sorted(cid for cid, row in adm["cards"].items() if row.get("admissible") and row.get("dsl"))


@functools.lru_cache(maxsize=1)
def _flagged() -> dict[str, list[dict]]:
    specs = load_specs(ROOT / "code" / "digimon-engine" / "cards")
    ids = [cid for cid in _admissible_dsl_cards() if cid in specs]
    result = extract_run(ids, "printed-clause-authoring guard")
    printed: dict[str, list[dict]] = {}
    for clause in result["clauses"]:
        printed.setdefault(clause["card_id"], []).append(clause)
    out = {}
    for cid in ids:
        missing = unmatched_clauses(printed.get(cid, []), specs[cid][1])
        if missing:
            out[cid] = missing
    return out


def test_admissible_dsl_cards_author_every_printed_clause():
    flagged = _flagged()
    unreviewed = {}
    for cid, missing in flagged.items():
        left = [c for c in missing if not _is_reviewed(cid, c)]
        if left:
            unreviewed[cid] = left
    lines = [
        f"{cid}: {c['zone']} {c.get('timings') or ''} {c['text'][:120]!r}"
        for cid, missing in sorted(unreviewed.items())
        for c in missing
    ]
    assert not unreviewed, (
        "admissible cards with a printed clause that has no authored YAML clause "
        "(author it, or record a PARTIAL verdict and regenerate admissibility):\n"
        + "\n".join(lines)
    )


@pytest.mark.parametrize("card_id", sorted(REVIEWED_AUTHORED))
def test_reviewed_entries_are_still_admissible_dsl_cards(card_id):
    # A reviewed exception for a card that is no longer admissible is dead
    # weight; drop it so the list stays a list of live decisions.
    assert card_id in set(_admissible_dsl_cards())


@pytest.mark.parametrize("card_id", sorted(REVIEWED_AUTHORED))
def test_reviewed_entries_still_name_a_flagged_clause(card_id):
    # An exemption the matcher no longer needs (or one whose zone/text no
    # longer matches the flagged clause) is dead weight; drop or fix it.
    clauses = _flagged().get(card_id, [])
    assert any(_is_reviewed(card_id, c) for c in clauses), (card_id, clauses)
