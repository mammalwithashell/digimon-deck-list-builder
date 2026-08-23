"""Source-priority resolution: per-card printed text -> `Clause` list.

Resolution order per card (README, and CLAUDE.md "Printed card data"):

    1. `data/card_bundles/<ID>.md` (official Bandai DB) -- parsed from its
       machine-readable twin `data/card_official.json`'s `text_sections`
       (same underlying scrape, both written by `build_card_bundles.py`;
       the JSON is far cheaper to parse reliably than the markdown).
    2. `data/cards.json` + `data/card_overrides.json` (overrides win).
    3. **DCGO's C# card script** -- a SECOND source consulted only when the
       above produced no security text. DCGO scripts declare a
       `EffectTiming.SecuritySkill` block for every card with a security
       effect, so a script that EXISTS and contains no such block is
       positive evidence of absence (verified on EX12-020 Gasamon against
       the card face, DCGO, and cards.json independently) -- no
       image-required slot is emitted. A card DCGO has no script for stays
       genuinely unknown and falls through to (4).
    4. `image-required` -- ONLY for the security zone. Absence of text in
       cards.json's `security_effect_description_eng` is not evidence the
       card has no security-face text (see README): that field is
       populated for 3 of ~4300 cards pool-wide, so its emptiness is
       structurally uninformative. A bundle's absence of a "Security"
       section, by contrast, IS informative (it's the official DB, not a
       lossy scrape) -- so a bundled card with no Security section gets
       ZERO security clauses, not an image-required slot.

Clause texts that are ingestion artifacts rather than printed card text
(`|applinkdp =` MediaWiki residue, the bare field label "Inherited Effect",
content-free empty spans) are filtered out before they become clauses. The
filter is an exact-match blocklist, never a heuristic -- see
`is_ingestion_artifact`.

Deliberately NOT consulted for the "inherited" zone: `cards.json`'s
`inherited_effect_description_eng` field for Tamer/Option/Dual-kind cards.
The Digimon TCG rules concept "Inherited Effect" only exists for
Digimon/Digi-Egg cards (cards that can become digivolution material); for
other kinds the API positionally reuses this field for something else
entirely (observed: security-face text for EX12-073/EX12-066/EX12-069) --
an unverified ingestion quirk, not documented API behavior. Trusting it
would be exactly the silent-wrong-denominator failure this tool exists to
prevent, so it is read for Digimon/Digi-Egg kinds only. See the README.
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

from tools.clause_coverage.models import Clause
from tools.clause_coverage.text_split import split_clauses

CARD_KIND_DIGIMON = 0
CARD_KIND_TAMER = 1
CARD_KIND_OPTION = 2
CARD_KIND_DIGIEGG = 3
CARD_KIND_DUAL = 4

_KINDS_WITH_INHERITED_ZONE = frozenset({CARD_KIND_DIGIMON, CARD_KIND_DIGIEGG})

DEFAULT_IMG_DIR = os.environ.get(
    "DIGIMON_CARD_IMAGE_DIR",
    r"C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card",
)

# Bundle text_sections `label` -> clause zone. Anything not listed here
# (DUAL Effect / DUAL Rule / Special Digivolution Condition / Special Play
# Condition / and any future label) defaults to "effect" -- all of those
# are printed ability/condition text on the card face, just not the plain
# "Effect" box.
_LABEL_TO_ZONE = {
    "effect": "effect",
    "inherited effect": "inherited",
    "security": "security",
}


# --------------------------------------------------------------------------
# DCGO: the second source for the security zone
#
# DCGO's card scripts live at
# `<root>/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs` with
# UNDERSCORED filenames (`EX12-020` -> `EX12_020.cs`). The colour subdirectory
# is NOT derivable from a card id, so the lookup globs the set's colours --
# same approach as `code/tools/dcgo-harness/src/exam/dcgo_pool.rs`.
#
# Every DCGO card with a security effect declares it under the
# `EffectTiming.SecuritySkill` timing. Verified across the whole checkout:
# the substring "SecuritySkill" appears ONLY as `EffectTiming.SecuritySkill`
# (911 occurrences, zero other uses), and the neighbouring security-ish
# timings -- OnAddSecurity / OnLoseSecurity / OnSecurityCheck /
# OnDetermineDoSecurityCheck / OnDiscardSecurity / OnFaceUpSecurityIncreased
# -- do not contain it. So a plain substring test is exact here.
# --------------------------------------------------------------------------

_DCGO_CARD_EFFECT_SUBDIR = ("Assets", "Scripts", "CardEffect")
_DCGO_SECURITY_TIMING_TOKEN = "SecuritySkill"

#: DCGO has a script for the card and it declares a security effect.
DCGO_SECURITY_PRESENT = "present"
#: DCGO has a script for the card and it declares NO security effect --
#: positive evidence of absence.
DCGO_SECURITY_ABSENT = "absent"
#: No usable DCGO checkout, or no script for this card. Genuinely unknown;
#: the caller must fall back to the image-required slot.
DCGO_SECURITY_UNKNOWN = "unknown"

#: Sentinel distinguishing "caller said nothing, resolve the default root"
#: from "caller explicitly said there is no DCGO" (`dcgo_root=None`).
_RESOLVE_DEFAULT_DCGO_ROOT = object()


def _base_repo_root() -> Path | None:
    """The BASE repo root, following a worktree's `.git` pointer file.

    CLAUDE.md rule 29: the DCGO submodule is checked out once in the base
    repo; in a linked worktree `./DCGO` is an intentionally-empty
    placeholder. Pure path resolution (no subprocess) so it works wherever
    the package is importable.
    """
    project_root = Path(__file__).resolve().parents[3]
    git = project_root / ".git"
    if git.is_dir():
        return project_root
    if git.is_file():
        try:
            pointer = git.read_text(encoding="utf-8").strip()
        except OSError:
            return None
        if pointer.startswith("gitdir:"):
            gitdir = Path(pointer.split(":", 1)[1].strip())
            # .../<base>/.git/worktrees/<name> -> base repo is 3 levels up.
            if gitdir.parent.name == "worktrees":
                return gitdir.parent.parent.parent
    return None


def default_dcgo_root() -> Path | None:
    """Where to look for DCGO when the caller doesn't say.

    `DIGIMON_DCGO_ROOT` overrides (set it to the empty string to disable the
    DCGO consultation entirely). Otherwise the base-repo `DCGO/` checkout.
    Returns `None` when neither resolves -- the package must stay usable
    where DCGO is absent, and absence means "unknown", never "no clause".
    """
    env = os.environ.get("DIGIMON_DCGO_ROOT")
    if env is not None:
        return Path(env) if env.strip() else None
    base = _base_repo_root()
    return base / "DCGO" if base is not None else None


def _dcgo_script_path(dcgo_root: Path, card_id: str) -> Path | None:
    """`EX12-020` -> `<root>/Assets/Scripts/CardEffect/EX12/Blue/EX12_020.cs`."""
    set_name, _, rest = card_id.partition("-")
    if not set_name or not rest:
        return None
    set_dir = dcgo_root.joinpath(*_DCGO_CARD_EFFECT_SUBDIR, set_name)
    if not set_dir.is_dir():
        return None
    file_name = f"{card_id.replace('-', '_')}.cs"
    try:
        colours = sorted(set_dir.iterdir())
    except OSError:
        return None
    for colour in colours:
        candidate = colour / file_name
        if colour.is_dir() and candidate.is_file():
            return candidate
    return None


def dcgo_security_verdict(card_id: str, dcgo_root: str | os.PathLike | None) -> str:
    """Does DCGO say this card has a security effect?

    One of `DCGO_SECURITY_PRESENT` / `DCGO_SECURITY_ABSENT` /
    `DCGO_SECURITY_UNKNOWN`. Every failure mode (no root, a worktree's empty
    `./DCGO` placeholder, no script for the card, an unreadable file) answers
    UNKNOWN, so a missing DCGO can only ever preserve today's behaviour --
    it can never silently delete a slot.
    """
    if dcgo_root is None:
        return DCGO_SECURITY_UNKNOWN
    root = Path(dcgo_root)
    if not root.joinpath(*_DCGO_CARD_EFFECT_SUBDIR).is_dir():
        return DCGO_SECURITY_UNKNOWN
    script = _dcgo_script_path(root, card_id)
    if script is None:
        return DCGO_SECURITY_UNKNOWN
    try:
        source = script.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return DCGO_SECURITY_UNKNOWN
    return DCGO_SECURITY_PRESENT if _DCGO_SECURITY_TIMING_TOKEN in source else DCGO_SECURITY_ABSENT


# --------------------------------------------------------------------------
# Ingestion-artifact filtering
#
# Deliberately an exact-match blocklist of OBSERVED artifacts, not a
# heuristic: a clever "looks like junk" rule risks eating real printed text,
# which corrupts the denominator in the more dangerous direction (a real
# clause silently stops being tracked). Every entry is grounded in measured
# pool-wide data:
#
# - "Inherited Effect": `effect_description_eng` is prefixed with the literal
#   box label on 10 cards (BT25-001..006, EX12-001..004); on 9 of them the
#   splitter emits it as a standalone content-free leading clause. The 10th
#   (EX12-001) is one long clause that merely BEGINS with the label -- exact
#   matching leaves its real text alone, which is the whole point of not
#   using a prefix rule.
# - `|applinkdp =`: `inherited_effect_description_eng` is exactly this on 33
#   cards. It is the only pipe-leading value anywhere in the three text
#   fields pool-wide, so anchoring on a leading `|` cannot collide with card
#   text.
# - empty/whitespace-only: only when the span carries no timing and no
#   keyword either. A keyword clause legitimately has EMPTY text (all its
#   content is in `keyword` -- e.g. EX12-065's <Fortitude>), and so does a
#   marker-only timing clause; a blanket empty-text rule would delete both.
# --------------------------------------------------------------------------

_ARTIFACT_EXACT_TEXTS = frozenset(
    {
        "inherited effect",
        "security effect",
    }
)

# A whole-clause MediaWiki template key line, e.g. `|applinkdp =`. Anchored at
# the start: no printed card text begins with a pipe.
_MEDIAWIKI_KEY_LINE = re.compile(r"\|\s*[A-Za-z_][A-Za-z0-9_]*\s*=[^\n]*")


def _normalize_artifact_text(text: str) -> str:
    """Fold the non-breaking spaces the API ingest sprinkles into card text."""
    return text.replace(" ", " ").strip()


def is_ingestion_artifact(
    text: str,
    *,
    kind: str,
    timings: list[str],
    keyword: str | None,
) -> bool:
    """Is this span scrape residue rather than printed card text?

    "Effect" alone is deliberately NOT on the blocklist: it is an ordinary
    English word that legitimately ends real clause fragments (e.g. `ST1-15`
    splits a sentence ending in "effect."), and eating those would be exactly
    the silent-loss failure this filter exists to avoid.
    """
    normalized = _normalize_artifact_text(text)
    if normalized.casefold() in _ARTIFACT_EXACT_TEXTS:
        return True
    if _MEDIAWIKI_KEY_LINE.fullmatch(normalized):
        return True
    if not normalized and kind == "untimed" and not timings and not keyword:
        return True
    return False


def _load_json(path: Path) -> dict:
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def load_cards_index(path: Path) -> dict:
    return _load_json(path)


def load_overrides_index(path: Path) -> dict:
    return _load_json(path)


def load_official_index(path: Path) -> dict:
    """Load `data/card_official.json` -> {card_id: card_official_entry}.

    Missing file is tolerated (empty index -- every card falls through to
    the cards.json path), since the official mirror is a nice-to-have, not
    a hard dependency.
    """
    if not Path(path).exists():
        return {}
    data = _load_json(path)
    return data.get("cards", {})


def load_image_cache(path: Path | None) -> dict:
    """Load a human/vision-filled-in text cache: {card_id: {zone: text}}.

    See README "Filling in image-required slots". Optional -- an absent
    path yields an empty cache and every unresolved security slot stays
    image-required.
    """
    if not path:
        return {}
    if not Path(path).exists():
        return {}
    return _load_json(Path(path))


def _merged_card_record(card_id: str, cards_index: dict, overrides_index: dict) -> tuple[dict, set[str]]:
    """Shallow-merge `card_overrides.json`'s patch onto the base cards.json record.

    Returns (merged_record, field_names_touched_by_the_patch) so callers can
    tag `source="card_overrides"` per-field rather than per-card.
    """
    base = dict(cards_index.get(card_id, {}))
    patch = overrides_index.get(card_id)
    touched: set[str] = set()
    if isinstance(patch, dict):
        for key, value in patch.items():
            if key.startswith("_"):  # "_comment"-style metadata, not a field
                continue
            base[key] = value
            touched.add(key)
    return base, touched


def extract_card_clauses(
    card_id: str,
    *,
    cards_index: dict,
    overrides_index: dict,
    official_index: dict,
    image_cache: dict | None = None,
    img_dir: str | None = None,
    dcgo_root: str | os.PathLike | None = _RESOLVE_DEFAULT_DCGO_ROOT,
) -> list[Clause]:
    """Resolve one card's printed text to its full `Clause` list.

    Deterministic: processing order is fixed (bundle sections in file
    order; else effect -> xros_req -> dual-option-effect -> inherited ->
    security), so clause `id`s are stable run over run for unchanged data.

    `dcgo_root` selects the DCGO checkout consulted as the security zone's
    second source: omit it to resolve `default_dcgo_root()`, or pass `None`
    to skip DCGO entirely (which restores the pre-DCGO behaviour exactly).
    """
    image_cache = image_cache or {}
    img_dir = img_dir or DEFAULT_IMG_DIR
    if dcgo_root is _RESOLVE_DEFAULT_DCGO_ROOT:
        dcgo_root = default_dcgo_root()

    clauses: list[Clause] = []
    counters = {"effect": 0, "inherited": 0, "security": 0}

    def add(zone: str, label: str, text: str, source: str) -> None:
        for span in split_clauses(text):
            # Scrape residue never becomes a clause -- and never consumes an
            # index, so the surviving siblings renumber down. See
            # `is_ingestion_artifact`.
            if is_ingestion_artifact(
                span.text, kind=span.kind, timings=span.timings, keyword=span.keyword
            ):
                continue
            idx = counters[zone]
            counters[zone] += 1
            clauses.append(
                Clause(
                    id=f"{card_id}#{zone}#{idx}",
                    card_id=card_id,
                    zone=zone,
                    label=label,
                    kind=span.kind,
                    timings=span.timings,
                    keyword=span.keyword,
                    text=span.text,
                    source=source,
                )
            )

    official = official_index.get(card_id)

    if official is not None:
        for section in official.get("text_sections", []) or []:
            label = str(section.get("label", "")).strip()
            text = section.get("text") or ""
            if not text.strip():
                continue
            zone = _LABEL_TO_ZONE.get(label.lower(), "effect")
            add(zone, label or "bundle", text, source="bundle")
        # Bundle exists and has no "Security" section: the official DB is
        # authoritative, so a missing section is a CONFIRMED absence, not
        # lossy silence -- deliberately NOT an image-required slot.
        return clauses

    record, touched = _merged_card_record(card_id, cards_index, overrides_index)
    kind = record.get("card_kind", CARD_KIND_DIGIMON)

    def source_for(field_name: str) -> str:
        return "card_overrides" if field_name in touched else "cards_json"

    effect_text = record.get("effect_description_eng") or ""
    if effect_text.strip():
        add("effect", "effect_description_eng", effect_text, source_for("effect_description_eng"))

    xros_req = record.get("xros_req") or ""
    if xros_req.strip():
        add("effect", "xros_req", xros_req, source_for("xros_req"))

    dual = record.get("dual")
    if isinstance(dual, dict):
        option_text = (dual.get("option") or {}).get("effect_text") or ""
        if option_text.strip():
            add("effect", "dual.option.effect_text", option_text, "cards_json")

    if kind in _KINDS_WITH_INHERITED_ZONE:
        inherited_text = record.get("inherited_effect_description_eng") or ""
        if inherited_text.strip():
            add(
                "inherited",
                "inherited_effect_description_eng",
                inherited_text,
                source_for("inherited_effect_description_eng"),
            )
    # else: Tamer/Option/Dual -- deliberately not read; see module docstring.

    security_text = record.get("security_effect_description_eng") or ""
    security_resolved = False
    if security_text.strip():
        add(
            "security",
            "security_effect_description_eng",
            security_text,
            source_for("security_effect_description_eng"),
        )
        security_resolved = True

    if not security_resolved:
        cached = (image_cache.get(card_id) or {}).get("security")
        if cached and str(cached).strip():
            add("security", "security (image-cache)", str(cached), "image-cache")
            security_resolved = True

    if not security_resolved and dcgo_security_verdict(card_id, dcgo_root) == DCGO_SECURITY_ABSENT:
        # DCGO has a script for this card and it declares no
        # `EffectTiming.SecuritySkill` block: positive evidence the card
        # prints no [Security] box at all. Emitting an image-required slot
        # here would inflate the denominator with a clause that does not
        # exist and can therefore never be measured. Anything less certain
        # (no DCGO checkout, or no script for this card) still falls through.
        return clauses

    if not security_resolved:
        clauses.append(
            Clause(
                id=f"{card_id}#security#{counters['security']}",
                card_id=card_id,
                zone="security",
                label="image-required",
                kind="untimed",
                timings=[],
                keyword=None,
                text="",
                source="image-required",
                image_path=str(Path(img_dir) / f"{card_id}.webp"),
            )
        )
        counters["security"] += 1

    return clauses
