"""Keyword -> optional/mandatory kind, rule section, and PDF pages.

The kind predicts the PROMPT SHAPE, which is the single most error-prone axis
in scenario authoring:

- ``Opt-cost→Mand`` (Evade, Barrier, Alliance, Fragment, Decoy, Armor Purge,
  Digisorption, Overclock, Training) -- DCGO ASKS, then resolves mandatorily,
  so the line needs an ``expect:`` row.
- ``Mandatory`` (Piercing, Draw, De-Digivolve, Retaliation, Fortitude, Mind
  Link, Recovery) -- no prompt at all; an ``expect:`` row here desynchronizes
  the rest of the line.

Reads the COMMITTED, verified derivations in ``docs/digimon-rules/`` -- present
in every worktree -- and points at the exact ``general_rule.pdf`` pages. It
never replaces the manual: source priority puts the PDF first, and a brief is a
routing aid, not a ruling.

Standard library only.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

#: Kinds where the player is asked before the effect resolves.
PROMPTING_KINDS = frozenset({"Optional", "Opt-cost→Mand"})

#: A table row. The keyword cell is captured WHOLE and mined for backticked
#: names afterwards, because it is not always a single token: row 16-3 reads
#: ``| `<Security A. +x / -x>` (was `<Security Attack>`) | ...`` -- two
#: backticked spans plus prose. A pattern demanding exactly one span silently
#: dropped that row, leaving one of the most common keywords in the game
#: impossible to look up.
_ROW = re.compile(
    r"^\|\s*(?P<keyword>[^|]+?)\s*\|\s*(?P<kind>[^|]+?)\s*\|\s*(?P<when>[^|]*?)\s*"
    r"\|\s*(?P<semantics>.*?)\s*\|\s*(?P<rule>[\d-]+)\s*\|\s*$"
)

#: Every backticked span inside a keyword cell.
_BACKTICKED = re.compile(r"`([^`]+)`")


def _normalize(keyword: str) -> str:
    """`<Evade>` / "Evade" / "evade" -> "evade"."""
    return keyword.strip().strip("<>").strip("`").strip().lower()


def load_briefs(semantics_md: Path, rules_index: Path) -> dict[str, dict]:
    """Parse the keyword table and join it to the PDF page index."""
    pages_by_section: dict[str, dict] = {}
    if rules_index.exists():
        index = json.loads(rules_index.read_text(encoding="utf-8"))
        for entry in (index.get("keywords") or {}).values():
            section = entry.get("section")
            if section:
                pages_by_section[section] = entry

    briefs: dict[str, dict] = {}
    for line in semantics_md.read_text(encoding="utf-8").splitlines():
        m = _ROW.match(line)
        if not m:
            continue
        cell = m.group("keyword").strip()
        kind = m.group("kind").strip()
        when = m.group("when").strip()
        semantics = m.group("semantics").strip()
        rule = m.group("rule").strip()
        if cell.lower().strip("`<> ") == "keyword" or set(cell) <= set("-| :"):
            continue  # header / separator row
        # A cell may name the keyword more than once -- 16-3 gives both the
        # current "<Security A. +x / -x>" and the retired "<Security Attack>".
        # Register every one; the first is the canonical label.
        cell_names = _BACKTICKED.findall(cell) or [cell]
        keyword = cell_names[0]
        index_entry = pages_by_section.get(rule, {})
        brief = {
            "keyword": keyword,
            "kind": kind,
            "when": when,
            "semantics": semantics,
            "rule": rule,
            "pdf": index_entry.get("pdf", "general_rule.pdf"),
            "pages": index_entry.get("pages", ""),
            "expects_prompt": kind in PROMPTING_KINDS,
        }
        briefs[_normalize(keyword)] = brief

        # The table cell often carries a value/modifier suffix no card ever
        # prints bare ("<Recovery +x (Deck)>", "<Fragment (X)>", "<Draw x>") --
        # the rules index's curated `names` are the bare callable form a caller
        # actually asks for ("Recovery", "<Fragment", "<Draw"). Register those
        # as aliases onto the SAME brief rather than requiring a caller to know
        # the exact printed suffix, and never let an alias shadow a real row.
        for name in list(cell_names[1:]) + list(index_entry.get("names", [])):
            alias = _normalize(name)
            if alias:
                briefs.setdefault(alias, brief)
    return briefs


def lookup(briefs: dict, keyword: str) -> dict | None:
    """Look a keyword up. Returns ``None`` rather than guessing."""
    return briefs.get(_normalize(keyword))
