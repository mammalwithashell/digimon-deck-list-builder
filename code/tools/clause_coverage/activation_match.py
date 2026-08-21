"""Match DCGO `effect_activation` recording rows back to denominator clauses.

`effect_activation` rows (`docs/DCGO_RECORDING_SCHEMA.md` §"Effect-activation
row") carry `card_id` + `effect_description` (the printed clause text,
leading with its bracketed timing tag(s) or an angle-bracket keyword) plus
`executed` (did the effect body actually run, vs. offered-and-declined).

This module answers one question: for one activation row, which clause(s)
in this package's extracted denominator (`extract.py` / `card_sources.py`)
does it correspond to? It is pure and I/O-free -- `coverage.py` owns
aggregating match results into per-clause FIRED/OFFERED_ONLY/NOT_FIRED/
UNOBSERVABLE statuses.

## Why fuzzy, not exact, text matching

`effect_description` and this package's extracted clause `text` are the
same underlying printed ability rendered by two different pipelines (a live
DCGO C# `ICardEffect` object vs. `cards.json` / `card_official.json` /
`card_overrides.json`). Measured directly against the real
`D:\\dcgo-build\\vb-corpus3\\` corpus, these pipelines agree on content but
not byte-for-byte: full-width vs. half-width punctuation, curly vs. straight
apostrophes, "•" vs. "・" bullets, `\\r\\n` vs. `\\n`, and small independent
wording choices ("set your memory to 3" vs. "set it to 3"; "[ME] or [VB]"
vs. "[VB] or [ME]"; "in its text" vs. "in their texts"). Exact-equality
matching on the *union* of the real 274-row corpus left 48/274 rows with an
unmatched span purely from this noise; a normalized similarity-ratio match
(threshold `MATCH_THRESHOLD`) resolves all of those down to the small
residue of genuine findings (a card missing from the denominator entirely,
a security-zone clause the extractor could not resolve from text and
punted to `image-required`, and a cross-card granted-ability attribution
where DCGO logs the *recipient* card's `card_id` for text it does not
itself print). See `qa/archetype-qa/` conventions and this package's
README for the source-priority rules the denominator itself follows.

## Matching rule, by clause kind

- **keyword**: match on the angle-bracket keyword NAME only (case-folded,
  normalized), never on body text. DCGO's rendered keyword activation
  omits the parenthetical reminder text this package's extractor keeps
  (e.g. activation `"<Raid> (When this Digimon attacks, ...)"` -- the
  parenthetical is DCGO's OWN reminder text, present on both sides in this
  example, but not reliably so; keyword identity is unambiguous from the
  name alone within one card's clause list).
- **timing**: candidate clauses must have a timing-marker set that is
  comparable with the activation span's (one is a subset of the other --
  DCGO fires a per-trigger subset of a compound printed marker group, e.g.
  a card printed "[On Play] [When Digivolving] [When Attacking] [Once Per
  Turn] <X>" logs three SEPARATE activations, each carrying only its own
  timing plus "Once Per Turn"). Among comparable candidates, pick the one
  with the highest normalized-body similarity ratio; accept only if that
  ratio clears `MATCH_THRESHOLD`. An empty-vs-empty body (both sides
  produced by splitting the same "compound timing marker immediately
  followed by an embedded keyword" shape) is a perfect match --
  disambiguated by the marker-subset requirement, not by body content.
- **untimed**: body similarity only (no markers to compare); a blank
  activation or blank candidate body never matches anything -- otherwise
  every blank `security` "image-required" placeholder clause (this
  package emits one whenever it cannot confirm security-zone text; see
  the README) would spuriously match every unrelated untimed activation.

Reuses `text_split.split_clauses` for BOTH sides of the comparison (the
denominator's own clauses, and re-parsing the activation's raw
`effect_description`) so a single activation whose printed text is a
compound-timing-clause-plus-embedded-keyword (e.g. `EX12-042`'s "[On Play]
[Once Per Turn] Add your top security card to the hand. Then,
<Recovery +1>.") produces the same two-clause split on both sides and each
half can be matched independently.
"""

from __future__ import annotations

import difflib
import re
import unicodedata
from dataclasses import dataclass, field

from tools.clause_coverage.text_split import ClauseSpan, split_clauses

# Similarity floor (difflib.SequenceMatcher ratio, 0..1) for accepting a
# fuzzy body match. Chosen empirically against the real vb-corpus3 corpus:
# every genuine (card, clause) correspondence in that corpus scored >= 0.86
# after normalization; this leaves a comfortable margin while still
# rejecting unrelated bodies (which score far lower -- two different
# clauses on the same card never came within reach of this floor).
MATCH_THRESHOLD = 0.7

# Marker names that describe a digivolve/play COST or CONDITION line, not
# an activated ability. DCGO never dispatches these through
# `Activate_Optional_Effect_Execute` -- there is nothing to "activate":
# digivolve legality and Use Req. gating are evaluated directly by the
# engine, not run as an effect. The task's own investigation names
# `[Digivolve]` / `[DNA Digivolve]` / `[Use Req.]` as a "confirmed" gap;
# the remaining names are the *same* structural kind of line -- the other
# alt-digivolve-mechanism markers `text_split.TIMING_MARKER_NAMES` already
# recognizes (`Assembly`, `DigiXros`, `Arts Digivolve`, `Blast Digivolve`,
# `Burst Digivolve`) -- and are included for the identical reason, not
# just the three literal names given. Directly observed in this package's
# own 120-clause VB denominator: `EX12-018#effect#7` is an "Arts
# Digivolve" cost line with no corresponding activation anywhere in the
# 274-row corpus, exactly as expected for a structurally-unobservable
# clause kind.
COST_LINE_MARKERS: frozenset[str] = frozenset(
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

_BULLET_CHARS_RE = re.compile("[\u30fb\u2022\u2027\u2219\u25cf\u25e6\u00b7]")
_CURLY_APOSTROPHE_RE = re.compile("[\u2019\u02bc\uff07]")
_CURLY_QUOTE_RE = re.compile("[\u201c\u201d]")
_DASH_RE = re.compile("[\u2010-\u2015\u2212]")
_WHITESPACE_RE = re.compile(r"\s+")


def normalize_text(text: str | None) -> str:
    """Normalize printed clause text for cross-pipeline comparison.

    NFKC-normalizes full/half-width forms, folds a handful of bullet
    glyphs (・ • ‧ ∙ ● ◦ ·) and dash variants to plain ASCII equivalents,
    folds curly quotes/apostrophes to straight ones, collapses all
    whitespace runs (including \\r\\n) to a single space, strips, and
    case-folds. Returns "" for falsy input.
    """
    if not text:
        return ""
    s = unicodedata.normalize("NFKC", text)
    s = _BULLET_CHARS_RE.sub(" ", s)
    s = _CURLY_APOSTROPHE_RE.sub("'", s)
    s = _CURLY_QUOTE_RE.sub('"', s)
    s = _DASH_RE.sub("-", s)
    s = _WHITESPACE_RE.sub(" ", s)
    return s.strip().casefold()


def text_similarity(a: str, b: str) -> float:
    """Normalized [0, 1] similarity ratio between two ALREADY-normalized strings."""
    return difflib.SequenceMatcher(None, a, b).ratio()


def is_blind_spot_kind(clause: dict) -> bool:
    """Whether this clause's KIND is a confirmed structural blind spot for
    the `effect_activation` hook (see module + package docstrings):
    angle-bracket keyword clauses (persistent/passive, not activated), or
    a digivolve/Use-Req. cost-line timing clause.

    This is independent of whether an activation row happens to have
    matched this specific clause -- see `coverage.py`'s status-assignment
    logic for how real matched evidence, when present, overrides this
    fallback rather than being discarded.
    """
    if clause.get("kind") == "keyword":
        return True
    if clause.get("kind") == "timing":
        timings = clause.get("timings") or []
        if any(t in COST_LINE_MARKERS for t in timings):
            return True
    return False


@dataclass
class MatchOutcome:
    """Result of matching one `effect_activation` row's `effect_description`
    against one card's candidate clauses."""

    matched_clause_ids: list[str] = field(default_factory=list)
    unmatched_spans: list[ClauseSpan] = field(default_factory=list)


def match_activation_row(
    effect_description: str,
    clauses_for_card: list[dict],
    *,
    threshold: float = MATCH_THRESHOLD,
) -> MatchOutcome:
    """Match one activation row's printed text against ONE card's clauses.

    `clauses_for_card` must already be filtered to the activation's
    `card_id` (this function has no card-identity knowledge of its own).
    Splits `effect_description` with the SAME `split_clauses` used to
    build the denominator, so a compound-timing-plus-embedded-keyword
    description yields one span per denominator clause it corresponds to.
    Each span is matched independently; a span with no acceptable
    candidate is returned in `unmatched_spans` rather than silently
    dropped (see package README "Unmatched activations must be reported
    loudly").
    """
    spans = split_clauses(effect_description)
    outcome = MatchOutcome()
    for span in spans:
        clause_id = _best_match(span, clauses_for_card, threshold)
        if clause_id is not None:
            outcome.matched_clause_ids.append(clause_id)
        else:
            outcome.unmatched_spans.append(span)
    return outcome


def _best_match(span: ClauseSpan, candidates: list[dict], threshold: float) -> str | None:
    same_kind = [c for c in candidates if c.get("kind") == span.kind]

    if span.kind == "keyword":
        target = normalize_text(span.keyword)
        for c in same_kind:
            if normalize_text(c.get("keyword")) == target:
                return c["id"]
        return None

    span_body = normalize_text(span.text)
    if span.kind == "untimed" and not span_body:
        # A blank untimed activation span carries no information to match
        # on, and would otherwise spuriously match every blank
        # "image-required" security placeholder clause on the card.
        return None

    span_markers = set(span.timings) if span.kind == "timing" else None

    best_id: str | None = None
    best_score = 0.0
    for c in same_kind:
        if span_markers is not None:
            c_markers = set(c.get("timings") or [])
            if not (span_markers <= c_markers or c_markers <= span_markers):
                continue
        c_body = normalize_text(c.get("text"))
        if not span_body and not c_body:
            # Both sides are the empty body of a "compound timing marker
            # immediately followed by an embedded keyword" split -- the
            # marker-subset check above is what disambiguates this case,
            # not body content.
            score = 1.0
        elif not span_body or not c_body:
            score = 0.0
        else:
            score = text_similarity(span_body, c_body)
        if score > best_score:
            best_score, best_id = score, c["id"]

    if best_id is not None and best_score >= threshold:
        return best_id
    return None
