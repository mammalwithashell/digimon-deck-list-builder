"""Printed clause -> authored YAML clause matcher.

Finds printed clauses (the ``extract`` denominator: official bundle first)
that have no authored counterpart in a card's YAML spec. It exists because a
card can be registered, lowering-covered and admissible for training while a
whole printed box is missing from its spec: Silphymon (BT24-037) shipped with
neither copy of its printed "[All Turns] [Once Per Turn] When this Digimon
would leave the battle area ..." clause (effect box and inherited box), and
Imperialdramon: Dragon Mode (BT20-076) with nothing but its keyword.

The match is a heuristic, not a proof of faithfulness:

* a printed clause in the inherited box needs an unused ``scope: inherited``
  clause; a link box clause a ``scope: linked`` (or inherited) clause; a
  ``[Security]`` clause an ``on_security`` / ``scope: security`` clause;
* a timing ("On Play", "When Attacking", ...) needs a clause whose ``when``
  carries a matching timing; a shared timing ("[On Play][When Digivolving]")
  may reuse an already matched clause;
* a keyword needs a ``grant_keyword`` naming it (or the keyword anywhere in
  the spec);
* a clause with only generic timings ([All Turns], [Your Turn], ...) takes
  any unused clause of its zone.

So it catches a MISSING clause, never a WRONG one. A flagged card is either a
real gap (implement it, or record a failed verdict so the admissibility gate
drops it) or a reviewed false positive listed by the guard
``code/tests/test_printed_clause_authoring.py`` with its reason.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Iterable

import yaml

TIMING_SIGNALS: dict[str, tuple[str, ...]] = {
    "On Play": ("on_play", "when_playing_this", "on_enter_field_anyone"),
    "When Digivolving": ("when_digivolving", "on_digivolve", "on_enter_field_anyone", "on_dna_digivolve"),
    "When Attacking": ("when_attacking", "on_attack"),
    "On Deletion": ("on_deletion", "on_any_deletion"),
    "End of Attack": ("end_of_attack",),
    "Main": ("main_from_hand", "main_on_field", "main", "main_from_trash"),
    "Security": ("on_security",),
    "Counter": ("counter",),
    "Start of Your Turn": ("start_of_your_turn",),
    "End of Your Turn": ("end_of_your_turn", "end_of_your_next_turn"),
    "Start of Your Main Phase": ("start_of_your_main_phase",),
    "End of Opponent's Turn": ("end_of_opponents_turn",),
    "Start of Opponent's Main Phase": ("start_of_opponents_main_phase",),
    "Start of Opponent's Turn": ("start_of_opponents_turn",),
    "When Linking": ("when_linked",),
}

# Printed timings that are digivolution / play conditions, not effect clauses:
# they are authored as alt_paths / special-play blocks.
CONDITION_TIMINGS = frozenset({
    "Digivolve", "DNA Digivolve", "DigiXros", "App Fusion", "Assembly", "Burst Digivolve",
    "Link", "Rule", "Special Digivolution Condition", "Mode Change", "Blast Digivolve",
    "Armor Purge", "Ace", "Digisorption", "Overflow", "Arts Digivolve",
})

_EMPTY_TEXT = frozenset({"", "-", "ー", "—", "{Hand}", "{Security}"})


def _norm(s: str) -> str:
    return re.sub(r"[^a-z]", "", s.lower())


def load_spec_clauses(spec: dict[str, Any]) -> list[dict[str, Any]]:
    clauses = [c for c in (spec.get("effects") or []) if isinstance(c, dict)]
    dual = spec.get("dual")
    if isinstance(dual, dict):
        for side in dual.values():
            if isinstance(side, dict):
                clauses += [c for c in (side.get("effects") or []) if isinstance(c, dict)]
    return clauses


def _has_use_requirement(spec: dict[str, Any]) -> bool:
    if spec.get("use_requirement"):
        return True
    dual = spec.get("dual")
    return isinstance(dual, dict) and any(
        isinstance(side, dict) and side.get("use_requirement") for side in dual.values()
    )


def _signals(c: dict[str, Any]) -> set[str]:
    out: set[str] = set()
    for key in ("when", "timing"):
        w = c.get(key)
        if w:
            out.update(w if isinstance(w, list) else [w])
    if c.get("kind"):
        out.add("kind:" + str(c["kind"]))
    if c.get("trigger"):
        out.add("trigger:" + str(c["trigger"]))
        out.add(str(c["trigger"]))  # a `kind: delay` clause names its timing here
    return {str(x) for x in out}


def _is_link_card(clauses: list[dict[str, Any]]) -> bool:
    return any(
        c.get("scope") == "linked" or c.get("kind") in ("link_requirement", "link_condition")
        for c in clauses
    )


def _candidates(zone: str, timings: set[str], text: str, clauses: list[dict[str, Any]]) -> list[int]:
    out = []
    # A link box's effect is extracted into the effect (or inherited) zone and
    # authored `scope: linked` (or, older idiom, `scope: inherited`).
    link_card = _is_link_card(clauses)
    linky = "When Linking" in text or re.match(r"^(DP\s*\+|\+\d+ ?DP)", text) is not None
    for i, c in enumerate(clauses):
        scope = c.get("scope")
        sig = _signals(c)
        if zone == "inherited":
            ok = scope == "inherited" or (link_card and scope == "linked")
        elif zone == "security":
            ok = scope == "security" or "on_security" in sig
        elif zone == "link":
            ok = scope in ("linked", "inherited")
        else:
            # `scope: security` = a printed {Security} static/timed effect.
            ok = scope not in ("inherited", "linked") and "on_security" not in sig
            ok = ok or ((linky or link_card) and scope in ("linked", "inherited"))
            ok = ok or ("Security" in timings and (scope in ("security", "inherited") or "on_security" in sig))
        if ok:
            out.append(i)
    return out


def unmatched_clauses(printed: Iterable[dict[str, Any]], spec: dict[str, Any]) -> list[dict[str, Any]]:
    """Printed clauses of one card with no authored counterpart in ``spec``."""
    clauses = load_spec_clauses(spec)
    spec_text = _norm(json.dumps(spec, ensure_ascii=False, default=str))
    has_into_path = any(
        isinstance(p, dict) and p.get("direction") == "into" for p in (spec.get("alt_paths") or [])
    )
    used = [False] * len(clauses)
    missing = []
    for p in printed:
        if p.get("source") == "image-required":
            continue
        timings = set(p.get("timings") or [])
        text = (p.get("text") or "").strip()
        if text in _EMPTY_TEXT or text.startswith(("DigiXros", "Assembly", "Inherited Effect")):
            continue
        # A quoted granted effect ("... gains "[X] ..." until ...") is split by
        # the extractor into a fragment; its host clause carries it.
        if (
            text.count('"') % 2 == 1
            or text.count(")") > text.count("(")
            or text[:1].islower()
            or re.fullmatch(r"([\[{][^\]}]+[\]}]\s*)+", text)
        ):
            continue
        if p.get("kind") == "timing" and timings & CONDITION_TIMINGS:
            continue
        if p.get("label") in ("Special Digivolution Condition", "Rule"):
            continue
        if "Use Req." in timings and _has_use_requirement(spec):
            continue
        if has_into_path and "digivolve into" in text.lower():
            continue
        if p.get("kind") == "untimed" and "digivolve" in text.lower() and spec.get("alt_paths"):
            continue
        zone = p.get("zone") or "effect"
        if "When Linking" in text:
            timings = timings | {"When Linking"}
        cands = _candidates(zone, timings, text, clauses)

        if p.get("kind") == "keyword" and p.get("keyword"):
            kw = _norm(p["keyword"])[:5]
            hit = next(
                (i for i in cands if not used[i] and kw and kw in _norm(json.dumps(clauses[i], default=str))),
                None,
            )
            if hit is None:
                if kw and kw in spec_text:
                    continue
                missing.append(p)
                continue
            used[hit] = True
            continue

        want: set[str] = set()
        for t in timings:
            want.update(TIMING_SIGNALS.get(t, ()))
        hit = None
        if want:
            hit = next((i for i in cands if not used[i] and _signals(clauses[i]) & want), None)
            if hit is None and any(_signals(clauses[i]) & want for i in cands):
                hit = -1  # a shared multi-timing clause already matched
        else:
            hit = next((i for i in cands if not used[i]), None)
        if hit is None:
            missing.append(p)
        elif hit >= 0:
            used[hit] = True
    return missing


def load_specs(cards_dir: Path) -> dict[str, tuple[Path, dict[str, Any]]]:
    out = {}
    for f in sorted(Path(cards_dir).glob("*/*.yaml")):
        spec = yaml.safe_load(f.read_text(encoding="utf-8"))
        if isinstance(spec, dict) and spec.get("card"):
            out[str(spec["card"])] = (f, spec)
    return out
