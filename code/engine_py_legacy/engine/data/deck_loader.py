"""Deck list parsing, validation, and restricted list enforcement.

Supports importing decks from digimoncard.io in two formats:
  - TTS (Tabletop Simulator): JSON array of card ID strings
  - Text: Human-readable lines like "4 Medusamon BT24-017"

Also provides deck validation against game rules and the official restricted list.
"""

from __future__ import annotations

import json
import re
from collections import Counter
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from engine_py_legacy.engine.data.card_database import CardDatabase
from engine_py_legacy.engine.data.enums import CardKind

# ─── Card ID Pattern ─────────────────────────────────────────────────
# Matches standard Digimon TCG card IDs: BT24-017, P-103, LM-027, ST1-01, EX8-037
RE_CARD_ID = re.compile(r'^[A-Z]{1,3}\d*-\d+$')


# ─── Restricted List (ENG) ───────────────────────────────────────────
# Source: https://digimoncard.io/restricted-list
# Matches DCGO's DataBase.ENGBanList

@dataclass
class CardRestriction:
    """Container for the official restricted list."""
    card_limits: Dict[str, int] = field(default_factory=dict)
    """card_id -> max copies allowed (0 = banned, 1 = restricted to 1)"""

    choice_groups: List[Tuple[List[str], List[str]]] = field(default_factory=list)
    """List of (group_a, group_b) — cards from group_a and group_b cannot coexist."""


# 3 Banned cards (0 copies allowed)
_BANNED = {
    "BT2-090": 0,   # Matt Ishida
    "BT5-109": 0,   # Mega Digimon Fusion!
    "EX5-065": 0,   # Sayo & Koh
}

# 47 Restricted cards (limited to 1 copy)
_RESTRICTED = {
    "BT1-090": 1,   # Gravity Crush
    "BT10-009": 1,  # Shoutmon X4
    "BT11-033": 1,  # MirageGaogamon
    "BT11-064": 1,  # Greymon (X Antibody)
    "BT13-012": 1,  # GeoGreymon
    "BT13-110": 1,  # Royal Knights of the Purge
    "BT14-002": 1,  # Bukamon
    "BT14-084": 1,  # T.K. Takaishi
    "BT15-057": 1,  # Numemon (X Antibody)
    "BT15-102": 1,  # Apocalymon
    "BT16-011": 1,  # Garudamon (X Antibody)
    "BT17-069": 1,  # Fenriloogamon
    "BT19-040": 1,  # Sakuyamon
    "BT2-047": 1,   # Argomon
    "BT2-069": 1,   # Gabumon
    "BT3-054": 1,   # Blossomon
    "BT3-103": 1,   # Hidden Potential Discovered!
    "BT4-104": 1,   # Blinding Ray
    "BT4-111": 1,   # Jack Raid
    "BT6-100": 1,   # Reinforcing Memory Boost!
    "BT6-104": 1,   # Parabolic Junk
    "BT7-038": 1,   # JetSilphymon
    "BT7-064": 1,   # DoruGreymon
    "BT7-069": 1,   # Eyesmon: Scatter Mode
    "BT7-072": 1,   # Eyesmon
    "BT7-107": 1,   # Calling From the Darkness
    "BT9-098": 1,   # Awakening of the Golden Knight
    "BT9-099": 1,   # Sunrise Buster
    "EX1-021": 1,   # MetalGarurumon
    "EX1-068": 1,   # Ice Wall!
    "EX2-039": 1,   # Impmon
    "EX2-070": 1,   # Digivolution Plug-In S
    "EX3-057": 1,   # Growlmon
    "EX4-006": 1,   # Guilmon
    "EX4-019": 1,   # MachGaogamon
    "EX4-030": 1,   # Kuzuhamon
    "EX5-015": 1,   # Gabumon (X Antibody)
    "EX5-018": 1,   # Garurumon (X Antibody)
    "EX5-062": 1,   # Anubismon
    "P-008": 1,     # WereGarurumon
    "P-025": 1,     # GranKuwagamon
    "P-029": 1,     # Agunimon
    "P-030": 1,     # Lobomon
    "P-123": 1,     # Ukkomon
    "P-130": 1,     # Lui Ohwada
    "ST2-13": 1,    # Hammer Spark
    "ST9-09": 1,    # Stingmon
}

# Choice restrictions — cards from side A and side B cannot coexist in the same deck
_CHOICE_GROUPS: List[Tuple[List[str], List[str]]] = [
    # Choice 1: Mother D-Reaper vs Shoto Kazama
    (["EX2-007"], ["EX7-064"]),
    # Choice 2: Chaosmon: Valdur Arm vs Taomon + Sakuyamon (X Antibody)
    (["BT20-037"], ["BT17-035", "EX8-037"]),
]

RESTRICTED_LIST = CardRestriction(
    card_limits={**_BANNED, **_RESTRICTED},
    choice_groups=_CHOICE_GROUPS,
)


# ─── Parsers ─────────────────────────────────────────────────────────

def parse_tts(raw: str) -> List[str]:
    """Parse a TTS (Tabletop Simulator) deck export.

    Input is a JSON array of strings. Non-card-ID entries
    (e.g., "Exported from https://digimoncard.io") are filtered out.

    Returns:
        List of card ID strings (one per copy).

    Raises:
        ValueError: If the input is not valid JSON or not a list.
    """
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid TTS JSON: {e}") from e

    if not isinstance(data, list):
        raise ValueError(f"TTS format expects a JSON array, got {type(data).__name__}")

    card_ids = []
    for item in data:
        if isinstance(item, str) and RE_CARD_ID.match(item):
            card_ids.append(item)
    return card_ids


def parse_text(raw: str) -> List[str]:
    """Parse a digimoncard.io text format deck export.

    Format:
        // DigimonCard.io Deck List
        4 Medusamon BT24-017
        2 Agumon BT21-007
        ...

    Lines starting with // are comments (deck name).
    First token is quantity, last token is card ID, middle is card name (ignored).

    Returns:
        List of card ID strings (one per copy, expanded by quantity).

    Raises:
        ValueError: If no valid card lines are found.
    """
    card_ids: List[str] = []
    lines = raw.strip().split('\n')

    for line in lines:
        line = line.strip()
        # Skip empty lines and comments
        if not line or line.startswith('//'):
            continue

        tokens = line.split()
        if len(tokens) < 2:
            continue

        # First token must be a number (quantity)
        try:
            quantity = int(tokens[0])
        except ValueError:
            continue

        # Last token is the card ID
        card_id = tokens[-1]
        if not RE_CARD_ID.match(card_id):
            continue

        for _ in range(quantity):
            card_ids.append(card_id)

    if not card_ids:
        raise ValueError("No valid card entries found in text format input")

    return card_ids


def parse_deck(raw: str) -> List[str]:
    """Auto-detect format and parse a deck list string.

    Tries TTS (JSON array) first, falls back to text format.

    Returns:
        List of card ID strings (one per copy).

    Raises:
        ValueError: If parsing fails in both formats.
    """
    raw = raw.strip()
    if not raw:
        raise ValueError("Empty deck string")

    # Try JSON (TTS) first
    if raw.startswith('['):
        try:
            return parse_tts(raw)
        except ValueError:
            pass  # Fall through to text parsing

    # Try text format
    try:
        return parse_text(raw)
    except ValueError:
        pass

    raise ValueError(
        "Could not parse deck list. Expected either:\n"
        '  - TTS format: JSON array like ["BT24-017", "BT24-017", ...]\n'
        "  - Text format: lines like '4 Medusamon BT24-017'"
    )


# ─── Utility ─────────────────────────────────────────────────────────

def expand_deck_dict(card_counts: Dict[str, int]) -> List[str]:
    """Convert a {card_id: count} dict to a flat List[str].

    Bridges the scraper's output format to the engine's expected input.
    """
    card_ids: List[str] = []
    for card_id, count in card_counts.items():
        for _ in range(count):
            card_ids.append(card_id)
    return card_ids


def summarize_deck(card_ids: List[str]) -> Dict[str, int]:
    """Summarize a flat card ID list into {card_id: count} mapping."""
    return dict(Counter(card_ids))


# ─── Validation ──────────────────────────────────────────────────────

@dataclass
class DeckValidationResult:
    """Result of deck validation."""
    is_valid: bool
    errors: List[str] = field(default_factory=list)
    """Hard failures — deck cannot be used."""
    warnings: List[str] = field(default_factory=list)
    """Soft issues — deck can still be used but may have problems."""


def validate_deck(
    card_ids: List[str],
    restricted_list: Optional[CardRestriction] = None,
) -> DeckValidationResult:
    """Validate a deck list against game rules and the restricted list.

    Checks:
        1. Card existence in CardDatabase (unknown → warning)
        2. Main deck size = 50, Digi-Egg deck size 0-5
        3. Per-card copy limits (max_count_in_deck from card data)
        4. Restricted list (banned cards, restricted copies, choice groups)

    Args:
        card_ids: Flat list of card ID strings (one per copy).
        restricted_list: Override the default RESTRICTED_LIST for testing.

    Returns:
        DeckValidationResult with is_valid, errors, and warnings.
    """
    if restricted_list is None:
        restricted_list = RESTRICTED_LIST

    errors: List[str] = []
    warnings: List[str] = []

    db = CardDatabase()
    counts = Counter(card_ids)

    # Classify cards into main deck and egg deck
    main_count = 0
    egg_count = 0
    unknown_ids: List[str] = []

    for card_id in set(card_ids):
        entity = db.get_card(card_id)
        if entity is None:
            unknown_ids.append(card_id)
            # Still count for size check — assume main deck
            main_count += counts[card_id]
            continue

        if entity.card_kind == CardKind.DigiEgg:
            egg_count += counts[card_id]
        else:
            main_count += counts[card_id]

    # 1. Unknown card warnings
    for uid in sorted(unknown_ids):
        warnings.append(f"Unknown card ID: {uid} (not in card database)")

    # 2. Deck size checks
    if main_count != 50:
        errors.append(f"Main deck must be exactly 50 cards (got {main_count})")
    if egg_count > 5:
        errors.append(f"Digi-Egg deck must be 0-5 cards (got {egg_count})")

    # 3. Per-card copy limits
    for card_id, count in counts.items():
        entity = db.get_card(card_id)
        if entity is None:
            continue  # Already warned above
        max_copies = entity.max_count_in_deck
        if count > max_copies:
            errors.append(
                f"{card_id} ({entity.card_name_eng}): {count} copies exceeds "
                f"max {max_copies} per deck"
            )

    # 4. Restricted list checks
    for card_id, count in counts.items():
        if card_id in restricted_list.card_limits:
            limit = restricted_list.card_limits[card_id]
            if limit == 0:
                entity = db.get_card(card_id)
                name = entity.card_name_eng if entity else card_id
                errors.append(f"{card_id} ({name}) is banned")
            elif count > limit:
                entity = db.get_card(card_id)
                name = entity.card_name_eng if entity else card_id
                errors.append(
                    f"{card_id} ({name}): {count} copies exceeds restricted "
                    f"limit of {limit}"
                )

    # 5. Choice group checks
    deck_ids_set = set(card_ids)
    for group_a, group_b in restricted_list.choice_groups:
        has_a = any(cid in deck_ids_set for cid in group_a)
        has_b = any(cid in deck_ids_set for cid in group_b)
        if has_a and has_b:
            a_names = ", ".join(group_a)
            b_names = ", ".join(group_b)
            errors.append(
                f"Choice restriction violated: cannot include cards from "
                f"[{a_names}] and [{b_names}] in the same deck"
            )

    return DeckValidationResult(
        is_valid=len(errors) == 0,
        errors=errors,
        warnings=warnings,
    )
