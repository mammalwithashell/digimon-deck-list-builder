"""Source-priority resolution: per-card printed text -> `Clause` list.

Resolution order per card (README, and CLAUDE.md "Printed card data"):

    1. `data/card_bundles/<ID>.md` (official Bandai DB) -- parsed from its
       machine-readable twin `data/card_official.json`'s `text_sections`
       (same underlying scrape, both written by `build_card_bundles.py`;
       the JSON is far cheaper to parse reliably than the markdown).
    2. `data/cards.json` + `data/card_overrides.json` (overrides win).
    3. `image-required` -- ONLY for the security zone. Absence of text in
       cards.json's `security_effect_description_eng` is not evidence the
       card has no security-face text (see README): that field is
       populated for 3 of ~4300 cards pool-wide, so its emptiness is
       structurally uninformative. A bundle's absence of a "Security"
       section, by contrast, IS informative (it's the official DB, not a
       lossy scrape) -- so a bundled card with no Security section gets
       ZERO security clauses, not an image-required slot.

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
) -> list[Clause]:
    """Resolve one card's printed text to its full `Clause` list.

    Deterministic: processing order is fixed (bundle sections in file
    order; else effect -> xros_req -> dual-option-effect -> inherited ->
    security), so clause `id`s are stable run over run for unchanged data.
    """
    image_cache = image_cache or {}
    img_dir = img_dir or DEFAULT_IMG_DIR

    clauses: list[Clause] = []
    counters = {"effect": 0, "inherited": 0, "security": 0}

    def add(zone: str, label: str, text: str, source: str) -> None:
        for span in split_clauses(text):
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
