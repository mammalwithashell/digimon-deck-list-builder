"""Pure text -> clause-span splitter. No I/O, no card-data knowledge.

Splits printed Digimon TCG card text into discrete, independently-testable
clauses on timing/marker boundaries, per the clause-splitting rules in this
package's `README.md`:

- Each **bracketed timing marker** (`[On Play]`, `[Main]`, `[Security]`, ...)
  starts a new clause. Consecutive timing markers separated only by
  whitespace (`[When Digivolving] [When Attacking]`) are a single COMPOUND
  clause carrying every timing in the run, not one clause per marker.
- Each **angle-bracket keyword** (`<Progress>`, `<Security A. +1>`, full- or
  half-width `＜...＞`) is its own clause too.
- **Both marker families only start a clause at a clause boundary.** A
  marker sitting *inside* a printed sentence is a reference to an ability,
  not the start of a new one — `"[Security] Activate this card's [Main]
  effect."` is ONE clause, and `"... Digimon gain ＜Blocker＞ and
  ＜Retaliation＞."` keeps both granted keywords inline. See
  `_is_clause_boundary` for the predicate and the README's "Positional
  boundary rule" for the worked cases and for what it still cannot see.
- A handful of marker names are printed via angle brackets in the raw text
  (`＜Use Req. (...)＞`, `＜Delay＞`, `＜Arts Digivolve＞`, ...) despite being
  *timing* markers conceptually (they gate WHEN/HOW an effect activates, not
  a persistent/mandatory keyword ability) — `TIMING_MARKER_NAMES` special-
  cases them so they behave like their square-bracket siblings.
- Any square-bracket token that ISN'T a recognized timing marker (`[VB]`,
  `[Gammamon]`, `[NSp]`, ...) is a trait/card-name reference, not a clause
  boundary — it's left as ordinary body text.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

# The timing-marker vocabulary. Sourced from the task's enumerated list
# verbatim, PLUS one directly-observed addition not in that list:
# "Start of Your Main Phase" (EX12-021's real printed text — see the
# README). Both square-bracket AND angle-bracket spellings match these
# names; angle-bracket printing is how the official DB renders a handful of
# these (Use Req., Delay, and the alt-digivolve-mechanism markers).
TIMING_MARKER_NAMES: frozenset[str] = frozenset(
    {
        "On Play",
        "When Digivolving",
        "When Attacking",
        "On Deletion",
        "Main",
        "Your Turn",
        "All Turns",
        "Opponent's Turn",
        "Counter",
        "Security",
        "End of Attack",
        "Start of Your Turn",
        "Once Per Turn",
        "Delay",
        "Rule",
        "Use Req.",
        "Digivolve",
        "DNA Digivolve",
        "Assembly",
        "DigiXros",
        "Arts Digivolve",
        "Blast Digivolve",
        "Burst Digivolve",
        # Directly observed in the VB deck's real printed text (EX12-021's
        # effect text: "[Start of Your Main Phase] By trashing 1 card...").
        # Not in the task's illustrative list, but structurally the same
        # kind of bracketed timing marker (a main-phase-scoped variant of
        # "Start of Your Turn").
        "Start of Your Main Phase",
    }
)

# Matches either a `[...]` (square) span, capturing group "sq", or a
# `<...>` / `＜...＞` (ASCII or full-width angle) span, capturing group
# "ang". Non-nested: content is "anything but the bracket's own delimiter
# family", which is enough because the only nesting observed in real card
# text is square-brackets/parens INSIDE an angle-bracket span (e.g.
# "＜Use Req. ([NSp]/[VB] trait)＞") -- the angle alternative's char class
# happily swallows those since it only excludes angle-bracket characters.
_BRACKET_RE = re.compile(
    r"\[(?P<sq>[^\[\]]+)\]" r"|[<＜](?P<ang>[^<>＜＞]+)[>＞]"
)

# A handful of cards print their use-requirement as unbracketed plain text
# ("Use Requirement: VB trait") instead of the "＜Use Req. (...)＞" bracket
# style -- observed on EX12-018's DUAL/option face. Normalized to a
# synthetic "[Use Req.]" marker before the main scan so both spellings are
# handled by one code path.
_USE_REQUIREMENT_PREFIX_RE = re.compile(
    r"^\s*Use Requirement:\s*([^\r\n]*)", re.IGNORECASE
)

_APOSTROPHE_RE = re.compile("[’ʼ]")
_STRIP_CHARS = " \n\t　・"  # space/tab/newline, ideographic space, ・ bullet

# Whitespace-ish characters that may sit between a clause's last real
# character and the next marker. Deliberately excludes "\n" -- a line break
# is itself boundary evidence (see `_is_clause_boundary`).
_INLINE_SPACE_CHARS = " \t　・"

# Characters that END a printed clause, so a marker following one opens a
# new clause. Beyond the obvious sentence enders these are:
#   - `)` `）` `]` -- the close of a reminder-text / parenthetical span,
#     e.g. "＜Use Req. ([VB] trait)＞ (Specified cards ...) [Main] ...".
#   - `}` -- the `{Hand}` / `{Security}` / `{Trash}` zone prefix printed
#     immediately before its timing marker ("{Hand} [Counter] ...").
#   - quote characters -- a granting sentence opens a quote around the
#     granted ability ('1 of your Digimon gains "[On Deletion] ..."'), and
#     that ability is independently testable, so it keeps its own clause.
_CLAUSE_END_CHARS = frozenset('.!?。！？)）]}"“”「」')

# Timing markers that head a *structured cost line* rather than prose:
# consecutive digivolution-condition lines run together with no sentence
# punctuation ("[Digivolve] Lv.6 w/[CS] trait: Cost 5 [DNA Digivolve] ..."),
# and these names are never printed as inline nouns, so they always open a
# clause. Kept in step with `activation_match.COST_LINE_MARKERS` (which
# imports FROM this module, so the constant cannot travel the other way);
# `test_clause_coverage_text_split.py` pins the two sets equal.
_ALWAYS_BOUNDARY_MARKER_NAMES: frozenset[str] = frozenset(
    {
        "Digivolve",
        "DNA Digivolve",
        "Use Req.",
        "Assembly",
        "DigiXros",
        "Arts Digivolve",
        "Blast Digivolve",
        "Burst Digivolve",
    }
)


def _canonical_marker_name(raw: str) -> str:
    """Normalize a bracket's inner text to a marker-name comparison key.

    Drops a trailing parenthetical reminder embedded IN the bracket (e.g.
    "Use Req. ([VB] trait)" -> "Use Req."), collapses internal whitespace,
    and normalizes curly apostrophes so "Opponent's Turn" matches either
    quote style.
    """
    name = raw.split("(", 1)[0].strip()
    name = _APOSTROPHE_RE.sub("'", name)
    name = re.sub(r"\s+", " ", name)
    return name


def _is_clause_boundary(text: str, prev_end: int, start: int) -> bool:
    """Is the marker at `start` opening a new clause, or embedded in one?

    `prev_end` is the end offset of the last marker that WAS accepted as a
    boundary (0 for the start of the field), so `text[prev_end:start]` is
    the body accumulated so far for the clause in progress. The marker opens
    a new clause when that body is empty (marker runs like
    `＜Progress＞ ＜Piercing＞` or `[When Digivolving] [When Attacking]`),
    ends at a line break, or ends on a clause-ending character. Otherwise
    the marker is a mid-sentence reference and stays inline.
    """
    segment = text[prev_end:start].rstrip(_INLINE_SPACE_CHARS)
    if not segment:
        return True
    if segment.endswith("\n"):
        return True
    return segment[-1] in _CLAUSE_END_CHARS


def _normalize_plain_use_requirement(text: str) -> str:
    match = _USE_REQUIREMENT_PREFIX_RE.match(text)
    if not match:
        return text
    requirement = match.group(1).strip()
    replacement = f"[Use Req.] {requirement}" if requirement else "[Use Req.]"
    return replacement + text[match.end() :]


@dataclass
class ClauseSpan:
    """One split-out clause, before card/zone/id context is attached."""

    kind: str  # "timing" | "keyword" | "untimed"
    timings: list[str] = field(default_factory=list)
    keyword: str | None = None
    text: str = ""


def split_clauses(raw_text: str | None) -> list[ClauseSpan]:
    """Split one field's printed text into `ClauseSpan`s.

    Pure function: no card/zone context, no I/O. Returns `[]` for empty or
    whitespace-only input.
    """
    if not raw_text or not raw_text.strip():
        return []

    text = raw_text.replace("\r\n", "\n").replace("\xa0", " ")
    text = _normalize_plain_use_requirement(text)

    events: list[dict] = []
    prev_boundary_end = 0
    for m in _BRACKET_RE.finditer(text):
        if m.group("sq") is not None:
            style, raw_inner = "square", m.group("sq")
        else:
            style, raw_inner = "angle", m.group("ang")
        canonical = _canonical_marker_name(raw_inner)
        if canonical in TIMING_MARKER_NAMES:
            kind = "timing"
        elif style == "angle":
            kind = "keyword"
        else:
            # Square-bracket, not a recognized timing marker: a trait or
            # card-name reference (e.g. [VB], [Gammamon]) -- inline text,
            # not a clause boundary.
            continue
        if canonical not in _ALWAYS_BOUNDARY_MARKER_NAMES and not _is_clause_boundary(
            text, prev_boundary_end, m.start()
        ):
            # Embedded mid-sentence ("Activate this card's [Main] effect.",
            # "... Digimon gain ＜Blocker＞ and ＜Retaliation＞.") -- a
            # reference to an ability, not the start of one. Leave it as
            # ordinary body text of the clause in progress.
            continue
        prev_boundary_end = m.end()
        events.append({"start": m.start(), "end": m.end(), "kind": kind, "name": canonical})

    clauses: list[ClauseSpan] = []

    first_start = events[0]["start"] if events else len(text)
    preamble = text[:first_start].strip(_STRIP_CHARS)
    if preamble:
        clauses.append(ClauseSpan(kind="untimed", text=preamble))

    i, n = 0, len(events)
    while i < n:
        ev = events[i]
        if ev["kind"] == "timing":
            group = [ev]
            j = i + 1
            while (
                j < n
                and events[j]["kind"] == "timing"
                and text[group[-1]["end"] : events[j]["start"]].strip() == ""
            ):
                group.append(events[j])
                j += 1
            body_start = group[-1]["end"]
            body_end = events[j]["start"] if j < n else len(text)
            body = text[body_start:body_end].strip(_STRIP_CHARS)
            clauses.append(
                ClauseSpan(kind="timing", timings=[g["name"] for g in group], text=body)
            )
            i = j
        else:  # keyword -- always its own clause, never grouped
            body_start = ev["end"]
            body_end = events[i + 1]["start"] if i + 1 < n else len(text)
            body = text[body_start:body_end].strip(_STRIP_CHARS)
            clauses.append(ClauseSpan(kind="keyword", keyword=ev["name"], text=body))
            i += 1

    return clauses
