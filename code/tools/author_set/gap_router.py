"""Flag-for-human path for the keyword gate (Phase 2, task 5).

When the keyword gate flags a keyword that neither the Rust engine nor DCGO
implements, the run must HALT: the keyword cannot be auto-ingested (no DCGO
behavioral spec to port), so a human supplies context + direction. This module:

- ``cards_using_keyword`` — find which set cards depend on a flagged keyword, so
  they can be excluded from mass-implement (task 5.2).
- ``route_flagged_keyword`` — record the gap in ``docs/RUST_ENGINE_GAPS.md`` and
  drop a ``.claude/plans/`` stub requesting direction (task 5.1). Idempotent:
  re-running does not duplicate an existing entry.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass

from .dcgo_manifest import normalize_keyword
from .keyword_gate import _strip_param, scan_bracket_tokens


@dataclass
class FlaggedKeyword:
    keyword: str                 # base form, e.g. "unchained"
    set_prefix: str
    affected_cards: list[str]


def cards_using_keyword(keyword: str, set_cards: dict) -> list[str]:
    """Return sorted card IDs whose printed text references ``keyword``.

    Matches on the normalized base form, so ``[App Fusion -4]`` and
    ``[App Fusion -6]`` both count toward ``app fusion``.
    """
    target = normalize_keyword(_strip_param(keyword))
    hits = []
    for cid, c in set_cards.items():
        text = " ".join(str(c.get(k, "") or "") for k in
                        ("effect_description_eng", "inherited_effect_description_eng",
                         "security_effect_description_eng"))
        # Reuse the gate's fullwidth-aware scanner (＜…＞ keyword tokens) so this
        # stays consistent with detection — a stale ASCII-only regex here misses
        # every ＜Keyword＞ and reports zero affected cards.
        for raw, _followed_by_trait in scan_bracket_tokens(text):
            if normalize_keyword(_strip_param(raw)) == target:
                hits.append(cid)
                break
    return sorted(hits)


# Per-subsystem DCGO call markers. A card *uses* a subsystem keyword only if its
# DCGO C# actually invokes the subsystem's machinery — NOT merely if its printed
# text mentions the bracket token (a Digimon that references "[TS] cards with
# [Link]" or grants nothing is not a Link card). Keyed by normalized keyword.
SUBSYSTEM_DCGO_MARKERS: dict[str, list[str]] = {
    "link": [
        r"LinkCard\s*\(", r"\.LinkCard\b", r"CanLink\b", r"AddLinkRequirement",
        r"ILinkCard", r"IsLinked", r"LinkEffect", r"ChangeLinkCost", r"ChangeLinkMax",
        r"SetIsLinkedEffect", r"AddSelfLinkConditionStaticEffect",
        r"GrantedReduceLinkCostClass", r"applinkdp",
    ],
}


def _dcgo_path_for(card_id: str, dcgo_root: str, set_prefix: str) -> str | None:
    """Locate a card's DCGO C# file: ``{root}/Assets/Scripts/CardEffect/{SET}/*/{ID_}.cs``.

    DCGO filenames use underscores (``BT25-075`` -> ``BT25_075.cs``) and live under
    a per-colour subdir, so the colour is globbed.
    """
    base = os.path.join(dcgo_root, "Assets", "Scripts", "CardEffect", set_prefix.upper())
    if not os.path.isdir(base):
        return None
    fname = f"{card_id.replace('-', '_')}.cs"
    for color in os.listdir(base):
        path = os.path.join(base, color, fname)
        if os.path.isfile(path):
            return path
    return None


def cards_using_subsystem_keyword(
    keyword: str,
    set_cards: dict,
    *,
    dcgo_root: str,
    set_prefix: str,
    text_fallback: bool = True,
) -> list[str]:
    """Return cards that ACTUALLY use a subsystem keyword, per DCGO C# (not text).

    This is the accurate per-card exclusion for ``auto_ingest_subsystem`` keywords
    (Link-style). The text-based :func:`cards_using_keyword` over-flags any card
    whose printed text mentions ``[Link]`` — including archetype members that
    merely *reference* link Options — which wrongly excludes the whole slice (BT25
    Aegiomon: 1 of 18 cards actually links, but all 18 mention it). Here a card
    counts only if its DCGO C# invokes the subsystem's machinery.

    Fallbacks (conservative — prefer over-flagging to silently authoring on a
    missing primitive):
    - DCGO file missing for a card -> fall back to the text mention.
    - keyword has no marker set -> degrade entirely to :func:`cards_using_keyword`.
    """
    norm = normalize_keyword(_strip_param(keyword))
    markers = SUBSYSTEM_DCGO_MARKERS.get(norm)
    if not markers:
        return cards_using_keyword(keyword, set_cards)
    pat = re.compile("|".join(markers))
    text_hits = set(cards_using_keyword(keyword, set_cards)) if text_fallback else set()
    hits: list[str] = []
    for cid in set_cards:
        path = _dcgo_path_for(cid, dcgo_root, set_prefix)
        if path is None:
            if cid in text_hits:
                hits.append(cid)
            continue
        try:
            src = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            if cid in text_hits:
                hits.append(cid)
            continue
        if pat.search(src):
            hits.append(cid)
    return sorted(hits)


def blocked_cards(flagged: list[FlaggedKeyword]) -> set[str]:
    """Union of all cards depending on any flagged keyword (excluded from authoring)."""
    out: set[str] = set()
    for f in flagged:
        out.update(f.affected_cards)
    return out


def _slugify(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", s.strip().lower()).strip("-")


def route_flagged_keyword(
    flagged: FlaggedKeyword,
    *,
    gaps_path: str = "docs/RUST_ENGINE_GAPS.md",
    plans_dir: str = ".claude/plans",
) -> dict:
    """Record a flagged keyword as an engine gap + plan stub. Idempotent.

    Returns ``{"gap_appended": bool, "plan_path": str, "plan_written": bool}``.
    """
    marker = f"<!-- author-set-flag:{flagged.set_prefix}:{flagged.keyword} -->"
    gap_appended = False
    if os.path.exists(gaps_path):
        existing = open(gaps_path, encoding="utf-8").read()
    else:
        existing = "# Rust Engine Gaps\n"
    if marker not in existing:
        entry = (
            f"\n## [{flagged.set_prefix}] new keyword: `{flagged.keyword}` "
            f"(flag-for-human) {marker}\n"
            f"- Surfaced by `/author-set {flagged.set_prefix}` keyword gate.\n"
            f"- NOT implemented in the Rust engine AND NOT found in the DCGO "
            f"keyword manifest — there is no battle-tested behavioral spec to "
            f"auto-port. Requires human context + direction.\n"
            f"- Affected {flagged.set_prefix} cards "
            f"({len(flagged.affected_cards)}): {', '.join(flagged.affected_cards) or '-'}\n"
            f"- These cards are EXCLUDED from mass-implementation until the "
            f"keyword is resolved.\n"
        )
        with open(gaps_path, "a", encoding="utf-8") as f:
            f.write(entry)
        gap_appended = True

    os.makedirs(plans_dir, exist_ok=True)
    plan_path = os.path.join(
        plans_dir, f"author-set-{_slugify(flagged.set_prefix)}-{_slugify(flagged.keyword)}.md"
    )
    plan_written = False
    if not os.path.exists(plan_path):
        with open(plan_path, "w", encoding="utf-8") as f:
            f.write(
                f"# Flagged keyword: `{flagged.keyword}` ({flagged.set_prefix})\n\n"
                f"`/author-set {flagged.set_prefix}` halted: this keyword is in "
                f"neither the Rust `Keyword` enum nor the DCGO keyword manifest.\n\n"
                f"## Needed from you\n"
                f"- What does `{flagged.keyword}` do (rules text / intended behavior)?\n"
                f"- Is there a DCGO or other reference implementation to port from?\n"
                f"- Is it a true keyword primitive, a trait, or card-specific behavior?\n\n"
                f"## Affected {flagged.set_prefix} cards\n"
                + "".join(f"- {c}\n" for c in flagged.affected_cards)
            )
        plan_written = True
    return {"gap_appended": gap_appended, "plan_path": plan_path, "plan_written": plan_written}
