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

# Printed keyword spellings whose Rust `Keyword` variant is named differently.
# `_strip_param` has already dropped the numeric param ("Draw 2" -> "draw",
# "Security A. +1" -> "security a."), so both Security A. signs land on the
# same token; the enum carries both variants, either proves coverage.
PRINTED_KEYWORD_ALIASES = {
    "draw": "drawx",
    "securitya": "securityattackplus",
}

# §16 keywords that are one-shot effect verbs rather than standing abilities:
# the engine has no `Keyword` variant because the DSL lowers them as steps.
DSL_STEP_KEYWORDS = {
    "recovery",  # <Recovery +x (Deck)> -> DSL step `recover`
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
    """``App Fusion -4`` -> ``app fusion`` ; ``Recovery +1 (Deck)`` -> ``recovery``."""
    s = token.strip().lower()
    s = re.sub(r"\s*\(.*?\)\s*$", "", s)          # trailing parenthetical
    s = re.sub(r"\s*[+\-]?\d+\s*$", "", s)        # trailing numeric param
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
# A bracketed token is a *reference* (not a keyword) when the words right after
# it say what it refers to: "[X] trait", "[X] in (any of) its traits" -> a trait;
# "[X] in its name" / "[X] in its text" / "[X] Token" -> a card or token name.
# The same holds for every item of a list run joined by EXPLICIT separators
# (/ , "or" "and" ", other than") that ends in such a suffix — e.g.
# "[Social]/[Tool]/[Game] trait" or "[Huckmon] or [Sistermon] in its text".
# Consecutive brackets joined only by bare whitespace are NOT a list (e.g.
# "[Link] [Appmon] trait" = the Link keyword + an [Appmon] trait clause).
_IN_ITS = r"in\s+(?:any\s+of\s+)?(?:its|their)\s+"
_TRAIT_SUFFIX = rf"(?:traits?|{_IN_ITS}traits?)\b"
_NAME_SUFFIX = rf"(?:{_IN_ITS}(?:names?|texts?)|tokens?)\b"
_LIST_SEP = r"(?:,\s*other\s+than|[/,]|\bor\b|\band\b)"


def _suffix_re(suffix: str) -> re.Pattern:
    return re.compile(rf"^[\W_]*{suffix}", re.IGNORECASE)


def _list_re(suffix: str) -> re.Pattern:
    return re.compile(
        rf"(?:{_OPEN}{_INNER}{_CLOSE}\s*{_LIST_SEP}\s*)+{_OPEN}{_INNER}{_CLOSE}[,\s]*{suffix}",
        re.IGNORECASE,
    )


_REF_KINDS = (
    ("trait", _suffix_re(_TRAIT_SUFFIX), _list_re(_TRAIT_SUFFIX)),
    ("name", _suffix_re(_NAME_SUFFIX), _list_re(_NAME_SUFFIX)),
)


def scan_bracket_refs(text: str) -> list[tuple[str, str | None]]:
    """Return ``(raw_token, ref_kind)`` for each bracketed token.

    ``ref_kind`` is ``"trait"`` or ``"name"`` when the token's position marks it
    as a reference (see the suffix rules above), else ``None``.
    """
    text = text or ""
    spans = [(kind, [m.span() for m in lst.finditer(text)]) for kind, _, lst in _REF_KINDS]
    out = []
    for m in _TOKEN_RE.finditer(text):
        token = m.group(1) if m.group(1) is not None else m.group(2)
        tail = text[m.end(): m.end() + 24]
        kind = None
        for (k, suf, _), (_, sp) in zip(_REF_KINDS, spans):
            if suf.match(tail) or any(s <= m.start() and m.end() <= e for s, e in sp):
                kind = k
                break
        out.append((token.strip(), kind))
    return out


def scan_bracket_tokens(text: str) -> list[tuple[str, bool]]:
    """Return ``(raw_token, followed_by_trait)`` for each bracketed token."""
    return [(tok, kind == "trait") for tok, kind in scan_bracket_refs(text)]


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
    effect_texts = list(effect_texts)
    # A token is named in the set's own text ("play 1 [X] Token"); its other
    # mentions ("if you don't have [X]") carry no positional suffix.
    token_names = {
        _strip_param(m.group(1))
        for text in effect_texts
        for m in re.finditer(rf"{_OPEN}({_INNER}){_CLOSE}\s*tokens?\b", text or "", re.IGNORECASE)
    }
    for text in effect_texts:
        for raw, ref_kind in scan_bracket_refs(text):
            base = _strip_param(raw)
            if not base:
                continue
            # 1. positional reference rules — beat everything (catch display-
            #    names and token names that no lexicon carries).
            if ref_kind == "trait":
                rep.trait_hits[base] += 1
                if base not in traits:
                    rep.lexicon_misses[base] += 1
                continue
            if ref_kind == "name":
                rep.name_hits[base] += 1
                continue
            # 2. known non-keyword bracket tokens.
            if base in KNOWN_TIMINGS or base in GRAMMAR:
                rep.ignored[base] += 1
                continue
            # 3. known card / token name, or trait, by lexicon.
            if base in card_names or base in token_names:
                rep.name_hits[base] += 1
                continue
            if base in traits:
                rep.trait_hits[base] += 1
                continue
            # 4. keyword triage against Rust enum, then DCGO manifest.
            norm = normalize_keyword(base)
            norm = PRINTED_KEYWORD_ALIASES.get(norm, norm)
            if norm in rust_keywords or norm in DSL_STEP_KEYWORDS:
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
