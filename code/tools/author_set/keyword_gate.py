"""The DCGO-oracle keyword gate (Phase 2, tasks 3.1 / 3.2 / 3.3).

Detect candidate new keywords in a release set and triage each one:

    covered        -> the engine runs it: a Rust Keyword variant, or DSL vocabulary
                      (the manifest's ``dsl_lowered_keywords``, e.g. <Delay>)
    trait          -> a trait reference (positional "[X] trait" rule, or lexicon)
    name_ref       -> a known card name (or a name stem: "[Sistermon Noir]")
    timing/grammar -> known non-keyword bracket token (ignored)
    auto_ingest    -> absent from the engine, present in the DCGO manifest -> port from C#
    flag_for_human -> absent from BOTH the engine and DCGO -> halt, request direction

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
from .lexicons import name_stems

# Bracket tokens that are timings, not keywords (square-bracket clause leads).
KNOWN_TIMINGS = {
    "on play", "when attacking", "your turn", "all turns", "opponent's turn",
    "main", "counter", "security", "on deletion", "end of attack",
    "when digivolving", "start of your turn", "end of your turn",
    "start of main phase", "end of opponent's turn", "when this digimon is deleted",
    "start of all turns", "end of all turns", "inherited effect", "inherited",
    "when moving", "our turn", "start of your main phase", "rule", "when linking",
}
# "[Start of Opponent's Turn]" / "[Start of Opponent's Main Phase]" (EX5-065,
# LM-020, EX3-024, BT26-027) and every other start/end-of-turn/phase lead.
_TIMING_RE = re.compile(
    r"^(?:start|end)\s+of\s+(?:your|opponent's|all|the)?\s*(?:turns?|main\s+phase)$"
)

# Bracket tokens that are grammar/markers, not keywords.
GRAMMAR = {
    "once per turn", "twice per turn", "digivolve", "free", "trash", "hand", "breeding",
    "hybrid", "x antibody", "ace", "de-digivolve", "dna digivolve",
}

# "[EX2-039 Impmon]": a name qualified by its card number.
_CARD_ID_PREFIX_RE = re.compile(r"^[a-z]+\d*-\d+\s+(.+)$")

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


def _fold(token: str) -> str:
    """Lowercase, trim, and fold the curly apostrophe the printed text mixes in."""
    return token.strip().lower().replace("’", "'")


def _strip_param(token: str) -> str:
    """``App Fusion -4`` -> ``app fusion`` ; ``Recovery +1 (Deck)`` -> ``recovery`` ;
    ``Digi-Burst up to 4`` -> ``digi-burst``."""
    s = _fold(token)
    s = re.sub(r"\s*\(.*?\)\s*$", "", s)                  # trailing parenthetical
    s = re.sub(r"\s*(?:up\s+to\s+)?[+\-]?\d+\s*$", "", s)  # trailing (up to) numeric param
    return s.strip()


def _lexicon_forms(raw: str) -> list[str]:
    """Lexicon keys for a bracket token, most literal first.

    A name or trait can END in what looks like a keyword parameter —
    ``[King Drasil_7D6]``, ``[Shoutmon EX6]``, ``[Ver.3]``,
    ``[Belphemon (X Antibody)]`` — so the unstripped token is looked up before
    the param-stripped one. ``[EX2-039 Impmon]`` also tries the bare name.
    """
    forms = [_fold(raw)]
    m = _CARD_ID_PREFIX_RE.match(forms[0])
    if m:
        forms.append(m.group(1).strip())
    forms.append(_strip_param(raw))
    return list(dict.fromkeys(f for f in forms if f))


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
# Printed variants: "in one of their traits" (BT10-056), "w/[X] in name"
# (BT26-060), and the bare "[Chronomon] text" (BT26-078).
_IN_ITS = r"in\s+(?:(?:any|one)\s+of\s+)?(?:(?:its|their)\s+)?"
_TRAIT_SUFFIX = rf"(?:traits?|{_IN_ITS}traits?)\b"
_NAME_SUFFIX = rf"(?:{_IN_ITS}(?:names?|texts?)|tokens?|texts?)\b"
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
    dsl_keywords: set[str] | None = None,
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
        dsl_keywords: normalized keywords the engine runs through DSL vocabulary
            rather than a ``Keyword`` variant (the manifest's
            ``dsl_lowered_keywords``) — covered, whatever DCGO's class.
    """
    subsystem_keywords = subsystem_keywords or set()
    engine_keywords = set(rust_keywords) | set(dsl_keywords or ()) | DSL_STEP_KEYWORDS
    stems = name_stems(card_names)
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
            if base in KNOWN_TIMINGS or base in GRAMMAR or _TIMING_RE.match(base):
                rep.ignored[base] += 1
                continue
            # 3. known card / token name, or trait, by lexicon — the literal
            #    token first, so a param-shaped tail is not stripped off a name.
            forms = _lexicon_forms(raw)
            name = next((f for f in forms if f in card_names or f in token_names), None)
            if name is not None:
                rep.name_hits[name] += 1
                continue
            trait = next((f for f in forms if f in traits), None)
            if trait is not None:
                rep.trait_hits[trait] += 1
                continue
            # 4. keyword triage against the engine (enum + DSL), then DCGO.
            norm = normalize_keyword(base)
            norm = PRINTED_KEYWORD_ALIASES.get(norm, norm)
            if norm in engine_keywords:
                rep.covered[base] += 1
            elif norm in dcgo_available:
                if norm in subsystem_keywords:
                    rep.auto_ingest_subsystem[base] += 1
                else:
                    rep.auto_ingest[base] += 1
            # 5. last resort before flagging: a card-family name stem
            #    ("[Sistermon Noir]" when the DB holds "Sistermon Noir
            #    (Awakened)"). After keyword triage, so a stem can never hide a
            #    keyword the engine or DCGO knows.
            elif any(f in stems for f in forms):
                rep.name_hits[next(f for f in forms if f in stems)] += 1
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
        dsl_keywords=set(m.get("dsl_lowered_keywords", {})),
        set_prefix=set_prefix,
    )
