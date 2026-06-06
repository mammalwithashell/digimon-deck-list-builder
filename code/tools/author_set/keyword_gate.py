"""The DCGO-oracle keyword gate (Phase 2, tasks 3.1 / 3.2 / 3.3).

Detect candidate new keywords in a release set and triage each one:

    covered        -> already a Rust Keyword variant (skip)
    trait          -> a trait reference (positional "[X] trait" rule, or lexicon)
    name_ref       -> a known card name
    timing/grammar -> known non-keyword bracket token (ignored)
    auto_ingest    -> absent from Rust, present in the DCGO manifest -> port from C#
    flag_for_human -> absent from BOTH Rust and DCGO -> halt, request direction

The detector is a set-subtraction; the positional "[X] trait" rule is what
catches trait display-names (``[Aqua]``, ``[Sovereign]``) that are absent from
the type_eng lexicon. The gate errs toward ``flag_for_human`` on ambiguity — a
false flag costs a human glance; a false auto-ingest ships a wrong primitive.
"""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass, field

from .dcgo_manifest import normalize_keyword

# Bracket tokens that are timings, not keywords (square-bracket clause leads).
KNOWN_TIMINGS = {
    "on play", "when attacking", "your turn", "all turns", "opponent's turn",
    "main", "counter", "security", "on deletion", "end of attack",
    "when digivolving", "start of your turn", "end of your turn",
    "start of main phase", "end of opponent's turn", "when this digimon is deleted",
    "start of all turns", "end of all turns", "inherited effect", "inherited",
    "when moving", "our turn", "start of your main phase", "rule", "when linking",
}

# Bracket tokens that are grammar/markers, not keywords.
GRAMMAR = {
    "once per turn", "digivolve", "free", "trash", "hand", "breeding",
    "hybrid", "x antibody", "ace", "de-digivolve", "dna digivolve",
}

VERDICTS = ("covered", "auto_ingest", "flag_for_human", "trait", "name_ref", "ignored")


@dataclass
class KeywordGateReport:
    set_prefix: str = ""
    covered: Counter = field(default_factory=Counter)
    auto_ingest: Counter = field(default_factory=Counter)            # simple flag-style
    auto_ingest_subsystem: Counter = field(default_factory=Counter)  # Link-style — assess first
    flag_for_human: Counter = field(default_factory=Counter)
    trait_hits: Counter = field(default_factory=Counter)
    name_hits: Counter = field(default_factory=Counter)
    ignored: Counter = field(default_factory=Counter)
    # tokens classified as trait via the positional rule but absent from the
    # lexicon -> suggested lexicon additions (task 3.2 "lexicon-miss patching").
    lexicon_misses: Counter = field(default_factory=Counter)

    @property
    def blocks_authoring(self) -> bool:
        # A flagged keyword (nobody implements it) OR a subsystem keyword (DCGO has
        # it but it needs a scheduled engine port, not a cheap auto-ingest) both
        # block a naive full run until resolved.
        return bool(self.flag_for_human) or bool(self.auto_ingest_subsystem)

    def summary(self) -> str:
        return (
            f"[{self.set_prefix}] covered={len(self.covered)} "
            f"auto_ingest={sorted(self.auto_ingest)} "
            f"auto_ingest_subsystem={sorted(self.auto_ingest_subsystem)} "
            f"flag_for_human={sorted(self.flag_for_human)} "
            f"(traits={len(self.trait_hits)}, names={len(self.name_hits)}, "
            f"lexicon_misses={sorted(self.lexicon_misses)})"
        )


def _strip_param(token: str) -> str:
    """``App Fusion -4`` -> ``app fusion`` ; ``Draw 1`` -> ``draw``."""
    s = token.strip().lower()
    s = re.sub(r"\s*[+\-]?\d+\s*$", "", s)        # trailing numeric param
    s = re.sub(r"\s*\(.*?\)\s*$", "", s)          # trailing parenthetical
    return s.strip()


# Bracket classes. Digimon text mixes ASCII square brackets `[Trait]`/`[Timing]`
# with FULLWIDTH angle brackets `＜Keyword＞` (U+FF1C/FF1E) for keywords, plus
# occasional ASCII `<...>`. The scanner must accept all of them or it silently
# misses every ＜Keyword＞ token (BT25 regression).
_OPEN = r"[\[<＜]"
_CLOSE = r"[\]>＞]"
_INNER = r"[^\]>＞]+"
# token followed (within a few chars) by the word "trait" -> trait reference.
# Alternation: a fullwidth ＜…＞ keyword captures everything up to ＞ (so nested
# ASCII brackets like ＜Decode ([Aegiomon])＞ are kept whole, not truncated at
# the inner `]`); ASCII [..]/<..> tokens use the strict no-close-char inner.
_TOKEN_RE = re.compile(rf"＜([^＞]+)＞|[\[<]({_INNER})[\]>]")
_TRAIT_SUFFIX_RE = re.compile(r"^\W*trait", re.IGNORECASE)
# A run of bracketed tokens joined by EXPLICIT list separators (/ , "or" "and"),
# ending in "trait" — e.g. "[Social]/[Tool]/[Game] trait". Every token in the
# run is a trait reference. Consecutive brackets joined only by bare whitespace
# are NOT a list (e.g. "[Link] [Appmon] trait" = the Link keyword + an [Appmon]
# trait clause), so a separator is required between items.
_TRAIT_LIST_RE = re.compile(
    rf"(?:{_OPEN}{_INNER}{_CLOSE}\s*(?:[/,]|\bor\b|\band\b)\s*)+{_OPEN}{_INNER}{_CLOSE}\s*trait\b",
    re.IGNORECASE,
)


def scan_bracket_tokens(text: str) -> list[tuple[str, bool]]:
    """Return ``(raw_token, followed_by_trait)`` for each bracketed token.

    ``followed_by_trait`` is True when the token is immediately followed by the
    word "trait" OR when it sits inside a bracket-list that terminates in
    "trait" (so slash/comma/or-delimited trait enumerations are fully covered).
    """
    text = text or ""
    trait_spans = [m.span() for m in _TRAIT_LIST_RE.finditer(text)]
    out = []
    for m in _TOKEN_RE.finditer(text):
        token = m.group(1) if m.group(1) is not None else m.group(2)
        tail = text[m.end(): m.end() + 12]
        in_list = any(s <= m.start() and m.end() <= e for s, e in trait_spans)
        out.append((token.strip(), in_list or bool(_TRAIT_SUFFIX_RE.match(tail))))
    return out


def triage_set(
    effect_texts,
    *,
    traits: set[str],
    card_names: set[str],
    rust_keywords: set[str],
    dcgo_available: set[str],
    subsystem_keywords: set[str] | None = None,
    set_prefix: str = "",
) -> KeywordGateReport:
    """Scan a set's effect texts and triage every bracket token.

    Args:
        effect_texts: iterable of strings (main + inherited + security per card).
        traits / card_names: complete lexicons (lowercased).
        rust_keywords: normalized Rust ``Keyword`` variants (from the manifest).
        dcgo_available: normalized DCGO registry ∪ core-modeled allowlist.
        subsystem_keywords: normalized DCGO keywords that are Link-style subsystems
            (route to ``auto_ingest_subsystem`` — assess before porting).
    """
    subsystem_keywords = subsystem_keywords or set()
    rep = KeywordGateReport(set_prefix=set_prefix)
    for text in effect_texts:
        for raw, followed_by_trait in scan_bracket_tokens(text):
            base = _strip_param(raw)
            if not base:
                continue
            # 1. positional trait rule — beats everything (catches display-names).
            if followed_by_trait:
                rep.trait_hits[base] += 1
                if base not in traits:
                    rep.lexicon_misses[base] += 1
                continue
            # 2. known non-keyword bracket tokens.
            if base in KNOWN_TIMINGS or base in GRAMMAR:
                rep.ignored[base] += 1
                continue
            # 3. known card name / trait by lexicon.
            if base in card_names:
                rep.name_hits[base] += 1
                continue
            if base in traits:
                rep.trait_hits[base] += 1
                continue
            # 4. keyword triage against Rust enum, then DCGO manifest.
            norm = normalize_keyword(base)
            if norm in rust_keywords:
                rep.covered[base] += 1
            elif norm in dcgo_available:
                if norm in subsystem_keywords:
                    rep.auto_ingest_subsystem[base] += 1
                else:
                    rep.auto_ingest[base] += 1
            else:
                rep.flag_for_human[base] += 1
    return rep


def triage_set_from_artifacts(
    effect_texts,
    *,
    manifest_path: str = "data/dcgo_keyword_manifest.json",
    lexicons_path: str = "data/author_set_lexicons.json",
    set_prefix: str = "",
) -> KeywordGateReport:
    """Convenience loader: pull lexicons + manifest from their checked-in files."""
    import json

    from .lexicons import load_lexicons

    traits, names = load_lexicons(lexicons_path)
    with open(manifest_path, encoding="utf-8") as f:
        m = json.load(f)
    return triage_set(
        effect_texts,
        traits=traits,
        card_names=names,
        rust_keywords=set(m["rust_enum_keywords"]),
        dcgo_available=set(m["dcgo_available_keywords"]),
        subsystem_keywords=set(m.get("subsystem_keywords", [])),
        set_prefix=set_prefix,
    )
