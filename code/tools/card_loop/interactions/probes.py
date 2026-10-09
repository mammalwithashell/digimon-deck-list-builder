"""Risk-probe generator, `families@1` (design D6).

A probe is a generated interaction exam: "this clause, examined from THIS risk
angle". The generator is a pure, rule-based function of a clause's extracted
record (`tools.clause_coverage.models.Clause`: `id`, `card_id`, `kind`,
`timings`, `keyword`, `text`), so the same clause extraction always yields the
same probes and the denominator built from them is reproducible.

Probe ids: ``probe:<clause-id>:<family>`` and ``probe:<clause-id>:<family>:neg``
for negative probes (the clause must NOT fire / must NOT reach). One probe per
(clause, family, polarity) -- the family names the risk, the clause names where.

The eight families of `families@1`:

==================== ======== ==================================================
family               polarity fires on
==================== ======== ==================================================
optional_decline     positive "may", "up to N", an optional cost ("By trashing
                              ...,"), an Optional / Opt-cost->Mand keyword, a
                              `<Delay>` / `<Blast Digivolve>` / Arts timing --
                              the decline path is offered and honoured
scope                negative "this Digimon", "N of your/their ...", "all ...
                              Digimon", "any/each of ..." -- a non-target is
                              unaffected
once_per_turn_multi  positive `[Once Per Turn]` -- two copies in play
would_replacement    positive "when/if ... would", a would-keyword (Evade,
                              Barrier, Decode, ...)
granted_keyword      positive "gain(s)/get(s) <Keyword>" -- the granted keyword
                              carries its triggers
leave_play           positive `[On Deletion]`, "when ... leaves / is deleted",
                              Decode / Fortitude / Retaliation / ... (post-trash
                              reads, CLAUDE.md rule 25)
immunity             positive "isn't affected by", "effects don't affect",
                              `<Progress>` (printed or granted)
timing_gate          negative `[Your Turn]` / `[Opponent's Turn]` / start-of-
                              turn timings -- does not fire on the wrong turn
==================== ======== ==================================================

Text rules read the clause with its parenthetical reminder text REMOVED, so a
granted keyword's reminder ("gains <Rush> (This Digimon can attack ...)") does
not masquerade as the clause's own scope or optionality. A *keyword* clause is
classified from a frozen keyword table instead of its reminder text, because
the bundle often prints the keyword with no reminder at all -- the same card
must yield the same probes whichever source the text came from. Only a keyword
the table does not know (a keyword newer than the table) falls back to its
unwrapped reminder text.

A family version is FROZEN once committed: changing what a family fires on
changes the gating denominator, so it is a new version (`families@2`) that
gates only after explicit promotion (see `denominator.py`).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Callable, Iterable, Mapping

from tools.card_loop.interactions import PROBE_PREFIX

FAMILY_VERSION = "families@1"

POSITIVE = "positive"
NEGATIVE = "negative"

# ---------------------------------------------------------------------------
# Frozen tables (families@1). Derived from docs/digimon-rules/keyword-semantics.md
# (general_rule.pdf Ver.3.6 §16); `test_card_loop_interactions_probes.py` guards
# the derivable ones against that table so the two cannot drift silently.
# ---------------------------------------------------------------------------

#: Kind `Optional` or `Opt-cost->Mand` in the §16 table.
_TABLE_OPTIONAL_KEYWORDS = frozenset({
    "digisorption", "digi-burst", "blitz", "delay", "decoy", "armor purge", "save",
    "material save", "evade", "raid", "alliance", "barrier", "blast digivolve",
    "partition", "scapegoat", "vortex", "overclock", "decode", "fragment", "execute",
    "training", "blast dna digivolve",
})
#: Deliberate additions beyond the table's kind column:
#:  - blocker: kind "Persistent", but its semantics are "may suspend to redirect"
#:    -- not blocking is a decline the engine must offer;
#:  - ascension: newer than Ver.3.6; "you may place this card as the top
#:    security card".
_EXTRA_OPTIONAL_KEYWORDS = frozenset({"blocker", "ascension"})
OPTIONAL_KEYWORDS = _TABLE_OPTIONAL_KEYWORDS | _EXTRA_OPTIONAL_KEYWORDS

#: The §16 table's "when" column says "would ..." (immediate replacement).
WOULD_KEYWORDS = frozenset({
    "decoy", "armor purge", "partition", "evade", "barrier", "scapegoat", "decode",
    "fragment",
})

#: Keywords that fire as their carrier leaves play (or read it after it left).
LEAVE_KEYWORDS = frozenset({
    "decode", "fortitude", "material save", "retaliation", "partition", "ascension",
    "overflow",
})

IMMUNITY_KEYWORDS = frozenset({"progress"})

#: Every keyword name some families@1 rule knows. A keyword clause naming one of
#: these is classified by the tables only; anything else falls back to text.
KNOWN_KEYWORDS = frozenset({
    # §16 table (general_rule.pdf Ver.3.6), by base name
    "security a", "security attack", "blocker", "recovery", "piercing", "draw",
    "jamming", "digisorption", "reboot", "de-digivolve", "retaliation", "digi-burst",
    "rush", "blitz", "delay", "decoy", "armor purge", "save", "material save",
    "evade", "raid", "alliance", "barrier", "blast digivolve", "fortitude",
    "mind link", "partition", "collision", "blast dna digivolve", "scapegoat",
    "vortex", "overclock", "iceclad", "decode", "fragment", "execute", "progress",
    "link", "training",
}) | _EXTRA_OPTIONAL_KEYWORDS | LEAVE_KEYWORDS

#: Timings whose activation is itself optional.
OPTIONAL_TIMINGS = frozenset({"Delay", "Blast Digivolve", "Arts Digivolve"})

#: Timings that name whose turn the clause lives on (the wrong turn is testable).
TURN_GATED_TIMINGS = frozenset({
    "Your Turn", "Opponent's Turn",
    "Start of Your Turn", "End of Your Turn",
    "Start of Opponent's Turn", "End of Opponent's Turn",
    "Start of Your Main Phase", "End of Your Main Phase",
    "Start of Opponent's Main Phase", "End of Opponent's Main Phase",
})

ONCE_PER_TURN = "Once Per Turn"
LEAVE_TIMINGS = frozenset({"On Deletion"})

#: Timings that mark a digivolution / play REQUIREMENT or a rule line rather
#: than an effect. A clause carrying only these is never probed.
CONDITION_TIMINGS = frozenset({"Digivolve", "DNA Digivolve", "Burst Digivolve", "Use Req.", "Rule"})

#: Verbs of an optional processing condition ("By trashing 1 card ..., ...";
#: general_rule.pdf §15-6/15-7). Only at a sentence / clause start, so
#: "returned by your effects" or "0 from [X] by returning" never match.
_COST_VERBS = (
    "trashing", "suspending", "unsuspending", "placing", "deleting", "returning",
    "paying", "revealing", "flipping", "adding",
)

# ---------------------------------------------------------------------------
# Normalization
# ---------------------------------------------------------------------------

_FULLWIDTH = str.maketrans({"＜": "<", "＞": ">", "’": "'", "‘": "'"})
_KEYWORD_SUFFIX = re.compile(
    r"\s*(?:[+\-−]\s*\d|\d|《|≪|\(|\[|<)|\s+[+\-]?x\b|\s+up to\b"
)
_PAREN = re.compile(r"\([^()]*\)?")


def keyword_base(name: str | None) -> str | None:
    """`<Fragment ≪3≫>` / `De-Digivolve 1` / `Digi-Burst up to 4` -> base name.

    Lowercase, brackets dropped, the value / named-card suffix cut, trailing
    dots trimmed: `Security A. +1` -> `security a`.
    """
    if not name:
        return None
    s = name.translate(_FULLWIDTH).strip().strip("<>").strip().lower()
    s = _KEYWORD_SUFFIX.split(s, maxsplit=1)[0]
    s = s.strip(" .")
    return s or None


def _normalize_text(text: str) -> str:
    return re.sub(r"\s+", " ", (text or "").translate(_FULLWIDTH)).strip().lower()


def strip_reminders(text: str) -> str:
    """Remove parenthetical reminder text, including an unclosed trailing
    `(` (the splitter sometimes cuts a reminder in half)."""
    prev = None
    while prev != text:
        prev, text = text, _PAREN.sub(" ", text)
    text = re.sub(r"\s+([.,;:])", r"\1", text)
    return re.sub(r"\s+", " ", text).strip()


@dataclass(frozen=True)
class ClauseView:
    """The normalized facts every family rule reads."""

    clause_id: str
    card_id: str
    timings: tuple[str, ...]
    keyword: str | None      # base name, or None for a non-keyword clause
    body: str                # lowercased text the TEXT rules read
    raw: str                 # lowercased full text (reminders included)

    @property
    def known_keyword(self) -> bool:
        return self.keyword is not None and self.keyword in KNOWN_KEYWORDS

    @property
    def condition_only(self) -> bool:
        return bool(self.timings) and set(self.timings) <= CONDITION_TIMINGS


def view(clause: Mapping) -> ClauseView:
    """Build the view from a `Clause.to_dict()` / extract-output record."""
    raw = _normalize_text(clause.get("text", ""))
    kw = keyword_base(clause.get("keyword")) if clause.get("kind") == "keyword" else None
    if kw is not None and kw not in KNOWN_KEYWORDS:
        body = raw.replace("(", " ").replace(")", " ")  # unknown keyword: read its reminder
        body = re.sub(r"\s+", " ", body).strip()
    elif kw is not None:
        body = ""  # known keyword: the tables decide, never the reminder text
    else:
        body = strip_reminders(raw)
    return ClauseView(
        clause_id=clause["id"],
        card_id=clause.get("card_id") or clause["id"].split("#", 1)[0],
        timings=tuple(clause.get("timings") or ()),
        keyword=kw,
        body=body,
        raw=raw,
    )


# ---------------------------------------------------------------------------
# Family rules: ClauseView -> a short "why" string, or None when not firing.
# ---------------------------------------------------------------------------

_MAY = re.compile(r"\bmay\b")
# "up to 50 copies" is a deck-construction rule, not a choice.
_UP_TO = re.compile(r"\bup to \d+\b(?! cop(?:y|ies)\b)")
_COST = re.compile(r"(?:^|[.,;:・]\s*)by (" + "|".join(_COST_VERBS) + r")\b")


def _first_timing(v: ClauseView, wanted: Iterable[str]) -> str | None:
    wanted = set(wanted)
    return next((t for t in v.timings if t in wanted), None)


def rule_optional_decline(v: ClauseView) -> str | None:
    if v.known_keyword:
        return f"keyword:{v.keyword}" if v.keyword in OPTIONAL_KEYWORDS else None
    t = _first_timing(v, OPTIONAL_TIMINGS)
    if t:
        return f"timing:{t}"
    if _MAY.search(v.body):
        return "text:may"
    if _UP_TO.search(v.body):
        return "text:up to"
    m = _COST.search(v.body)
    if m:
        return f"text:by {m.group(1)}"
    return None


_SCOPE_RULES = (
    (re.compile(r"\bthis digimon\b"), "text:this digimon"),
    (re.compile(r"\b(?:\d+|one|two|three) of (?:your|your opponent's|their)\b"), "text:n of"),
    (re.compile(r"\b(?:any|each) of (?:your|your opponent's|their)\b"), "text:any/each of"),
    (re.compile(r"\ball (?:of )?(?:your |your opponent's |their |other |the )?[^.;]{0,40}?\bdigimon\b"),
     "text:all digimon"),
)


def rule_scope(v: ClauseView) -> str | None:
    if v.known_keyword:
        return None
    for pattern, why in _SCOPE_RULES:
        if pattern.search(v.body):
            return why
    return None


def rule_once_per_turn_multi(v: ClauseView) -> str | None:
    if ONCE_PER_TURN in v.timings:
        return f"timing:{ONCE_PER_TURN}"
    if "[once per turn]" in v.raw:
        return "text:[once per turn]"
    return None


_WOULD = re.compile(r"\b(?:when|if)\b[^.]*?\bwould\b")


def rule_would_replacement(v: ClauseView) -> str | None:
    if v.known_keyword:
        return f"keyword:{v.keyword}" if v.keyword in WOULD_KEYWORDS else None
    if _WOULD.search(v.body):
        return "text:when ... would"
    return None


_GRANT = re.compile(r"\b(?:gain|gains|get|gets) <([^>]+)>")


def rule_granted_keyword(v: ClauseView) -> str | None:
    if v.known_keyword:
        return None
    m = _GRANT.search(v.body)
    if m:
        return f"text:gains <{keyword_base(m.group(1))}>"
    return None


_LEAVES = re.compile(r"\bwhen\b[^.]*?\b(?:leave|leaves|left)\b")
# "would be deleted" is a replacement that keeps the Digimon in play (Evade,
# Armor Purge, ...): would_replacement, not leave_play.
_DELETED = re.compile(r"\bwhen\b[^.]*?\b(?:is|are) deleted\b")


#: A "would leave" clause that PREVENTS the leaving keeps the Digimon in play.
_PREVENTS = re.compile(r"\b(?:doesn't|does not|don't|do not) leave\b|\bprevent (?:that|it|the)\b")


def rule_leave_play(v: ClauseView) -> str | None:
    if v.known_keyword:
        return f"keyword:{v.keyword}" if v.keyword in LEAVE_KEYWORDS else None
    t = _first_timing(v, LEAVE_TIMINGS)
    if t:
        return f"timing:{t}"
    if _LEAVES.search(v.body) and not _PREVENTS.search(v.body):
        return "text:when ... leaves"
    if _DELETED.search(v.body):
        return "text:when ... deleted"
    return None


_IMMUNE = re.compile(
    r"\b(?:isn't|is not|aren't|are not) affected by\b"
    r"|\b(?:don't|do not|doesn't|does not) affect\b"
    r"|\bunaffected by\b"
)


def rule_immunity(v: ClauseView) -> str | None:
    if v.known_keyword:
        return f"keyword:{v.keyword}" if v.keyword in IMMUNITY_KEYWORDS else None
    if _IMMUNE.search(v.body):
        return "text:not affected"
    for m in _GRANT.finditer(v.body):
        if keyword_base(m.group(1)) in IMMUNITY_KEYWORDS:
            return f"text:gains <{keyword_base(m.group(1))}>"
    return None


def rule_timing_gate(v: ClauseView) -> str | None:
    t = _first_timing(v, TURN_GATED_TIMINGS)
    return f"timing:{t}" if t else None


@dataclass(frozen=True)
class Family:
    name: str
    polarity: str  # POSITIVE | NEGATIVE
    rule: Callable[[ClauseView], str | None]


#: version -> families. Family names are unique ACROSS versions, because a probe
#: id carries the family name and not the version.
FAMILY_VERSIONS: dict[str, tuple[Family, ...]] = {
    FAMILY_VERSION: (
        Family("optional_decline", POSITIVE, rule_optional_decline),
        Family("scope", NEGATIVE, rule_scope),
        Family("once_per_turn_multi", POSITIVE, rule_once_per_turn_multi),
        Family("would_replacement", POSITIVE, rule_would_replacement),
        Family("granted_keyword", POSITIVE, rule_granted_keyword),
        Family("leave_play", POSITIVE, rule_leave_play),
        Family("immunity", POSITIVE, rule_immunity),
        Family("timing_gate", NEGATIVE, rule_timing_gate),
    ),
}

FAMILY_NAMES_V1 = tuple(f.name for f in FAMILY_VERSIONS[FAMILY_VERSION])


def _check_registry(registry: Mapping[str, tuple[Family, ...]]) -> None:
    seen: dict[str, str] = {}
    for version, fams in registry.items():
        for f in fams:
            if f.name in seen:
                raise ValueError(
                    f"family {f.name!r} is defined by both {seen[f.name]} and {version}; "
                    "probe ids carry the family name, so names must be unique across versions"
                )
            if f.polarity not in (POSITIVE, NEGATIVE):
                raise ValueError(f"family {f.name!r} has polarity {f.polarity!r}")
            if not re.fullmatch(r"[a-z][a-z0-9_]*", f.name):
                raise ValueError(f"family name {f.name!r} must be snake_case")
            seen[f.name] = version


_check_registry(FAMILY_VERSIONS)


@dataclass(frozen=True)
class Probe:
    id: str
    clause_id: str
    card_id: str
    family: str
    family_version: str
    kind: str   # POSITIVE | NEGATIVE
    why: str    # which rule fired, e.g. "text:may", "keyword:evade"

    def to_dict(self) -> dict:
        return {
            "id": self.id, "clause_id": self.clause_id, "card_id": self.card_id,
            "family": self.family, "family_version": self.family_version,
            "kind": self.kind, "why": self.why,
        }


def probe_id(clause_id: str, family: str, negative: bool) -> str:
    return f"{PROBE_PREFIX}{clause_id}:{family}" + (":neg" if negative else "")


def parse_probe_id(pid: str) -> tuple[str, str, bool] | None:
    """`probe:BT1-001#effect#0:scope:neg` -> (`BT1-001#effect#0`, `scope`, True)."""
    if not pid.startswith(PROBE_PREFIX):
        return None
    parts = pid[len(PROBE_PREFIX):].split(":")
    if len(parts) == 2:
        clause, family = parts
        negative = False
    elif len(parts) == 3 and parts[2] == "neg":
        clause, family, _ = parts
        negative = True
    else:
        return None
    if clause.count("#") != 2 or not family:
        return None
    return clause, family, negative


def natural_key(s: str) -> list:
    """Sort `Q865` before `Q1601` and `#effect#2` before `#effect#10`."""
    return [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", s)]


def probes_for_clause(
    clause: Mapping,
    registry: Mapping[str, tuple[Family, ...]] = FAMILY_VERSIONS,
) -> list[Probe]:
    v = view(clause)
    if v.condition_only:
        return []
    out = []
    for version, fams in registry.items():
        for fam in fams:
            why = fam.rule(v)
            if why is None:
                continue
            negative = fam.polarity == NEGATIVE
            out.append(Probe(
                id=probe_id(v.clause_id, fam.name, negative),
                clause_id=v.clause_id,
                card_id=v.card_id,
                family=fam.name,
                family_version=version,
                kind=fam.polarity,
                why=why,
            ))
    return sorted(out, key=lambda p: natural_key(p.id))


def generate_probes(
    clauses: Iterable[Mapping],
    registry: Mapping[str, tuple[Family, ...]] = FAMILY_VERSIONS,
) -> list[Probe]:
    """Every probe for every clause, sorted by natural id. Deterministic."""
    if registry is not FAMILY_VERSIONS:
        _check_registry(registry)
    out: list[Probe] = []
    for c in clauses:
        out.extend(probes_for_clause(c, registry))
    out.sort(key=lambda p: natural_key(p.id))
    ids = [p.id for p in out]
    if len(ids) != len(set(ids)):
        dupes = sorted({i for i in ids if ids.count(i) > 1})
        raise ValueError(f"duplicate probe ids (duplicate clause ids in the input?): {dupes[:5]}")
    return out
