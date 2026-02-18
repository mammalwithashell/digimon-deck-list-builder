"""Meta Gauntlet: meta-weighted opponent selection and deck management.

Provides four core components for RL Pilot Agent training:

1. MetaDistribution  - Load local meta stats (DigiLab/card), sample archetypes
2. DeckLibrary       - Ingest deck lists from digimonmeta.com, Egman Events,
                       digimoncard.io (TTS/text), and JSON {card_id: count}
3. VariantGenerator  - Core + Flex randomization to prevent overfitting
4. MetaGauntlet      - Orchestrator tying distribution, library, and variants

Integration with DigimonEnv is provided via MetaGauntletEnv, a Gymnasium
wrapper that samples opponent decks per episode and scales rewards.

Usage:
    from training.gauntlet import MetaDistribution, DeckLibrary, MetaGauntlet

    meta = MetaDistribution.from_file("training/meta_config.json")
    library = DeckLibrary.load("training/deck_library.json")
    gauntlet = MetaGauntlet(meta, library)

    archetype, deck = gauntlet.sample_opponent()
    scale = gauntlet.reward_scale(archetype)
"""

from __future__ import annotations

import json
import random
import re
from collections import Counter
from copy import deepcopy
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
import gymnasium

from digimon_gym.digimon_gym import DigimonEnv
from digimon_gym.engine.data.deck_loader import (
    RE_CARD_ID,
    expand_deck_dict,
    parse_deck,
    parse_text,
    parse_tts,
    summarize_deck,
)


# ─── Card ID regex (re-exported for local parsers) ─────────────────
_RE_CARD_ID = RE_CARD_ID

# Export code format from digimonmeta.com: "4nBT13-007a4nBT13-040a..."
# URL parameter `dg=` uses 'a' as delimiter between {count}n{card_id} entries
_RE_EXPORT_CHUNK = re.compile(
    r'(\d{1,2})'          # count (1-2 digits)
    r'[a-zA-Z]'           # delimiter letter (usually 'n')
    r'([A-Z]{1,3}\d*-\d+)'  # card ID
)

# Egman Events deckbuilder format: "BT21-064:4,EX8-009:4,EX4-006:1"
# URL parameter `deck=` uses comma-separated CARD_ID:COUNT entries
_RE_EGMAN_URL_CHUNK = re.compile(
    r'([A-Z]{1,3}\d*-\d+)'  # card ID
    r':(\d{1,2})'           # :count
)

# Egman Events text format: "4x BT1-085 Tai Kamiya" or "4 BT1-085 Tai Kamiya"
_RE_EGMAN_LINE = re.compile(
    r'^\s*(\d{1,2})\s*x?\s+'   # count with optional 'x'
    r'([A-Z]{1,3}\d*-\d+)'     # card ID
    r'(?:\s+.*)?$'              # optional card name (ignored)
)


# =====================================================================
#  MetaDistribution
# =====================================================================

@dataclass
class ArchetypeStats:
    """Stats for a single archetype in the local meta."""
    meta_share: float  # fraction (0.0-1.0), e.g. 0.40 = 40%
    win_rate: float    # fraction (0.0-1.0), e.g. 0.55 = 55%


class MetaDistribution:
    """Loads and manages meta archetype distributions from DigiLab/card.

    The config format is a JSON object:
    {
        "archetypes": {
            "Red Hybrid": {"meta_share": 0.40, "win_rate": 0.55},
            "Blue Flare":  {"meta_share": 0.10, "win_rate": 0.48},
            ...
        }
    }

    Meta shares should sum to 1.0 (or close to it). If they don't,
    they are normalized internally for sampling.
    """

    def __init__(self, archetypes: Dict[str, ArchetypeStats]):
        if not archetypes:
            raise ValueError("MetaDistribution requires at least one archetype")
        self._archetypes = archetypes
        self._names: List[str] = list(archetypes.keys())
        # Build sampling weights (normalized meta shares)
        shares = [a.meta_share for a in archetypes.values()]
        total = sum(shares)
        if total <= 0:
            raise ValueError("Total meta share must be positive")
        self._weights = [s / total for s in shares]

    @classmethod
    def from_file(cls, path: str) -> MetaDistribution:
        """Load from a JSON config file.

        Expected format:
        {
            "archetypes": {
                "Red Hybrid": {"meta_share": 0.40, "win_rate": 0.55},
                ...
            }
        }
        """
        with open(path) as f:
            data = json.load(f)
        return cls.from_dict(data)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> MetaDistribution:
        """Create from a dictionary (for testing/programmatic use)."""
        raw = data.get("archetypes", data)
        archetypes: Dict[str, ArchetypeStats] = {}
        for name, stats in raw.items():
            archetypes[name] = ArchetypeStats(
                meta_share=float(stats["meta_share"]),
                win_rate=float(stats.get("win_rate", 0.50)),
            )
        return cls(archetypes)

    def get_weighted_opponent(self) -> str:
        """Sample an archetype name based on meta share %.

        Returns:
            Name of the sampled archetype (e.g., "Red Hybrid").
        """
        return random.choices(self._names, weights=self._weights, k=1)[0]

    def get_archetype_stats(self, archetype: str) -> ArchetypeStats:
        """Get stats for a specific archetype.

        Raises:
            KeyError: If archetype is not in the distribution.
        """
        return self._archetypes[archetype]

    def calculate_reward_scale(self, archetype: str) -> float:
        """Compute reward multiplier based on meta share.

        Formula: 1.0 + meta_share
        - 40% meta share -> 1.4x reward
        -  1% meta share -> 1.01x reward
        - 10% meta share -> 1.1x reward

        Falls back to 1.0 for unknown archetypes.
        """
        stats = self._archetypes.get(archetype)
        if stats is None:
            return 1.0
        return 1.0 + stats.meta_share

    @property
    def archetypes(self) -> List[str]:
        """List all archetype names."""
        return list(self._names)

    def __len__(self) -> int:
        return len(self._archetypes)

    def __repr__(self) -> str:
        return f"MetaDistribution({len(self)} archetypes)"


# =====================================================================
#  Deck Parsers
# =====================================================================

def parse_export_code(code: str) -> List[str]:
    """Parse a digimonmeta.com export code string.

    Format: "{count}n{card_id}" entries separated by 'a', commas,
    whitespace, or concatenated directly.

    Examples (digimonmeta.com dg= parameter):
        "4nBT13-007a4nBT13-040a3nBT13-093"

    Also handles space/comma separated:
        "4nBT24-004 3nBT20-010 2nST1-03"
        "4nBT24-004,3nBT20-010,2nST1-03"

    Returns:
        Flat list of card IDs (one per copy).

    Raises:
        ValueError: If no valid card entries are found.
    """
    matches = _RE_EXPORT_CHUNK.findall(code)
    if not matches:
        raise ValueError(
            "No valid card entries found in export code. "
            "Expected format like '4nBT13-007a4nBT13-040'"
        )
    card_ids: List[str] = []
    for count_str, card_id in matches:
        count = int(count_str)
        card_ids.extend([card_id] * count)
    return card_ids


def parse_egman_url(url_or_param: str) -> List[str]:
    """Parse an Egman Events deckbuilder URL or deck parameter.

    Format: "CARD_ID:COUNT,CARD_ID:COUNT,..."
    Full URL: "https://deckbuilder.egmanevents.com/?deck=BT21-064:4,...&type=digimon"

    Examples:
        "BT21-064:4,EX8-009:4,EX4-006:1"
        "https://deckbuilder.egmanevents.com/?deck=BT21-064:4,EX8-009:4&type=digimon"

    Returns:
        Flat list of card IDs (one per copy).

    Raises:
        ValueError: If no valid card entries are found.
    """
    # Extract deck= parameter from URL if present
    if 'deck=' in url_or_param:
        match = re.search(r'deck=([^&]+)', url_or_param)
        if match:
            url_or_param = match.group(1)

    matches = _RE_EGMAN_URL_CHUNK.findall(url_or_param)
    if not matches:
        raise ValueError(
            "No valid card entries found in Egman URL format. "
            "Expected format like 'BT21-064:4,EX8-009:4'"
        )
    card_ids: List[str] = []
    for card_id, count_str in matches:
        count = int(count_str)
        card_ids.extend([card_id] * count)
    return card_ids


def parse_egman_text(raw: str) -> List[str]:
    """Parse an Egman Events text format deck list.

    Format (card ID comes after count, before name):
        4x BT1-085 Tai Kamiya
        3 BT24-017 Medusamon
        2x ST1-03 Agumon

    Also handles the digimoncard.io text format where card ID is last:
        4 Tai Kamiya BT1-085

    Returns:
        Flat list of card IDs (one per copy).

    Raises:
        ValueError: If no valid card lines are found.
    """
    card_ids: List[str] = []
    lines = raw.strip().split('\n')

    for line in lines:
        line = line.strip()
        if not line or line.startswith('//') or line.startswith('#'):
            continue

        # Try Egman format first: "4x BT1-085 Name"
        m = _RE_EGMAN_LINE.match(line)
        if m:
            count = int(m.group(1))
            card_id = m.group(2)
            card_ids.extend([card_id] * count)
            continue

        # Fall back to digimoncard.io text format: "4 Name BT1-085"
        tokens = line.split()
        if len(tokens) >= 2:
            try:
                count = int(tokens[0].rstrip('x'))
            except ValueError:
                continue
            card_id = tokens[-1]
            if _RE_CARD_ID.match(card_id):
                card_ids.extend([card_id] * count)

    if not card_ids:
        raise ValueError("No valid card entries found in Egman text format")
    return card_ids


def parse_deck_any(raw: str) -> List[str]:
    """Auto-detect format and parse a deck list string.

    Tries formats in order:
    1. JSON (TTS / {card_id: count} dict)
    2. Egman Events URL (deckbuilder.egmanevents.com)
    3. Export code (digimonmeta.com dg= parameter)
    4. Egman Events CARD_ID:COUNT format
    5. Text (Egman Events text / digimoncard.io)

    Returns:
        Flat list of card IDs (one per copy).

    Raises:
        ValueError: If parsing fails in all formats.
    """
    raw = raw.strip()
    if not raw:
        raise ValueError("Empty deck string")

    # 1. Try JSON formats
    if raw.startswith('{') or raw.startswith('['):
        try:
            data = json.loads(raw)
            if isinstance(data, list):
                # TTS format: ["BT24-017", "BT24-017", ...]
                return parse_tts(raw)
            elif isinstance(data, dict):
                # JSON dict: {"BT24-017": 4, "ST1-03": 2}
                return expand_deck_dict(data)
        except (json.JSONDecodeError, ValueError):
            pass

    # 2. Try Egman Events URL format
    if 'deckbuilder.egmanevents.com' in raw or (
        _RE_EGMAN_URL_CHUNK.search(raw) and ':' in raw and '\n' not in raw
    ):
        try:
            return parse_egman_url(raw)
        except ValueError:
            pass

    # 3. Try digimonmeta.com export code (compact format with 'a' delimiter)
    if _RE_EXPORT_CHUNK.search(raw) and '\n' not in raw:
        try:
            return parse_export_code(raw)
        except ValueError:
            pass

    # 4. Try text formats (Egman text + digimoncard.io)
    try:
        return parse_egman_text(raw)
    except ValueError:
        pass

    # 5. Last resort: existing deck_loader parser
    return parse_deck(raw)


# =====================================================================
#  DeckLibrary
# =====================================================================

@dataclass
class DeckEntry:
    """A single deck list with metadata."""
    card_ids: List[str]
    name: str = ""
    source: str = ""  # e.g. "egman", "digimonmeta", "digimoncard.io", "manual"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "cards": summarize_deck(self.card_ids),
            "name": self.name,
            "source": self.source,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> DeckEntry:
        card_ids = expand_deck_dict(data["cards"])
        return cls(
            card_ids=card_ids,
            name=data.get("name", ""),
            source=data.get("source", ""),
        )


class DeckLibrary:
    """Database of deck lists tagged by archetype.

    Supports ingestion from multiple sources:
    - Egman Events text lists ("4x BT1-085 Tai Kamiya")
    - digimonmeta.com export codes ("4nBT24-004...")
    - digimoncard.io TTS/text exports
    - JSON files ({card_id: count} dicts)
    - Direct card ID lists
    """

    def __init__(self):
        self._decks: Dict[str, List[DeckEntry]] = {}

    def add_deck(self, archetype: str, card_ids: List[str],
                 name: str = "", source: str = "manual") -> None:
        """Add a deck list to the library.

        Args:
            archetype: Archetype tag (e.g., "Red Hybrid").
            card_ids: Flat list of card IDs (one per copy).
            name: Optional deck name.
            source: Data source identifier.
        """
        if archetype not in self._decks:
            self._decks[archetype] = []
        self._decks[archetype].append(DeckEntry(
            card_ids=list(card_ids),
            name=name,
            source=source,
        ))

    def add_deck_from_text(self, archetype: str, raw: str,
                           name: str = "", source: str = "digimoncard.io") -> None:
        """Parse a text format deck (digimoncard.io / Egman) and add it."""
        card_ids = parse_egman_text(raw)
        self.add_deck(archetype, card_ids, name=name, source=source)

    def add_deck_from_export_code(self, archetype: str, code: str,
                                  name: str = "",
                                  source: str = "digimonmeta") -> None:
        """Parse a digimonmeta.com export code and add it."""
        card_ids = parse_export_code(code)
        self.add_deck(archetype, card_ids, name=name, source=source)

    def add_deck_from_egman_url(self, archetype: str, url: str,
                               name: str = "",
                               source: str = "egman") -> None:
        """Parse an Egman Events deckbuilder URL and add it.

        Format: "https://deckbuilder.egmanevents.com/?deck=BT21-064:4,...&type=digimon"
        Also accepts just the deck parameter: "BT21-064:4,EX8-009:4"
        """
        card_ids = parse_egman_url(url)
        self.add_deck(archetype, card_ids, name=name, source=source)

    def add_deck_from_tts(self, archetype: str, raw: str,
                          name: str = "",
                          source: str = "digimoncard.io") -> None:
        """Parse a TTS (Tabletop Simulator) JSON array and add it."""
        card_ids = parse_tts(raw)
        self.add_deck(archetype, card_ids, name=name, source=source)

    def add_deck_from_json(self, archetype: str, path: str,
                           name: str = "", source: str = "file") -> None:
        """Load from a JSON file (either {card_id: count} or flat array).

        Also supports a wrapper format:
        {"archetype": "...", "name": "...", "cards": {"BT24-017": 4, ...}}
        """
        with open(path) as f:
            data = json.load(f)

        if isinstance(data, dict):
            # Check for wrapper format
            if "cards" in data:
                card_ids = expand_deck_dict(data["cards"])
                name = name or data.get("name", "")
            else:
                # Direct {card_id: count}
                card_ids = expand_deck_dict(data)
        elif isinstance(data, list):
            # Flat array of card IDs
            card_ids = [c for c in data if isinstance(c, str) and _RE_CARD_ID.match(c)]
        else:
            raise ValueError(f"Unsupported JSON format in {path}")

        self.add_deck(archetype, card_ids, name=name, source=source)

    def add_deck_from_file(self, archetype: str, path: str,
                           name: str = "") -> None:
        """Load a deck from a file, auto-detecting format.

        Supports .json files and plain text files.
        """
        p = Path(path)
        if p.suffix == '.json':
            self.add_deck_from_json(archetype, path, name=name, source="file")
        else:
            raw = p.read_text()
            card_ids = parse_deck_any(raw)
            self.add_deck(archetype, card_ids, name=name, source="file")

    def get_deck(self, archetype: str) -> List[str]:
        """Get a random deck for the given archetype.

        Returns:
            Flat list of card IDs (one per copy).

        Raises:
            KeyError: If archetype has no registered decks.
        """
        decks = self._decks.get(archetype)
        if not decks:
            raise KeyError(f"No decks registered for archetype '{archetype}'")
        entry = random.choice(decks)
        return list(entry.card_ids)

    def get_all_decks(self, archetype: str) -> List[List[str]]:
        """Get all decks for the given archetype.

        Returns:
            List of flat card ID lists.
        """
        decks = self._decks.get(archetype, [])
        return [list(e.card_ids) for e in decks]

    @property
    def archetypes(self) -> List[str]:
        """List all registered archetype names."""
        return list(self._decks.keys())

    def deck_count(self, archetype: Optional[str] = None) -> int:
        """Count decks, optionally filtered by archetype."""
        if archetype:
            return len(self._decks.get(archetype, []))
        return sum(len(v) for v in self._decks.values())

    def save(self, path: str) -> None:
        """Save the library to a JSON file."""
        data: Dict[str, Any] = {}
        for archetype, entries in self._decks.items():
            data[archetype] = [e.to_dict() for e in entries]
        with open(path, 'w') as f:
            json.dump(data, f, indent=2)

    @classmethod
    def load(cls, path: str) -> DeckLibrary:
        """Load the library from a JSON file."""
        lib = cls()
        with open(path) as f:
            data = json.load(f)
        for archetype, entries in data.items():
            for entry_data in entries:
                entry = DeckEntry.from_dict(entry_data)
                if archetype not in lib._decks:
                    lib._decks[archetype] = []
                lib._decks[archetype].append(entry)
        return lib

    def __len__(self) -> int:
        return self.deck_count()

    def __repr__(self) -> str:
        archs = len(self._decks)
        total = self.deck_count()
        return f"DeckLibrary({archs} archetypes, {total} decks)"


# =====================================================================
#  VariantGenerator (Core + Flex System)
# =====================================================================

class VariantGenerator:
    """Generates deck variants by randomizing flex slots.

    Identifies "core" cards (always at max copies) vs "flex" cards
    (tech options that vary between builds) and produces randomized
    variants to prevent the Pilot Agent from overfitting to specific lists.

    Strategy:
    - Cards at `core_threshold` copies (default 4) are "core" - always kept.
    - Cards below the threshold are "flex" - subject to randomization.
    - A flex_pool (optional) provides alternative cards to swap in.
    - Variants maintain the same deck size as the original.
    """

    def __init__(self, core_threshold: int = 4):
        """
        Args:
            core_threshold: Minimum copy count to be considered "core".
                            Cards with exactly this many copies are core.
        """
        self.core_threshold = core_threshold

    def analyze(self, card_ids: List[str]) -> Tuple[Dict[str, int], Dict[str, int]]:
        """Split a deck into core and flex cards.

        Args:
            card_ids: Flat list of card IDs (one per copy).

        Returns:
            (core_cards, flex_cards) as {card_id: count} dicts.
            Core cards have count >= core_threshold.
            Flex cards have count < core_threshold.
        """
        counts = Counter(card_ids)
        core: Dict[str, int] = {}
        flex_cards: Dict[str, int] = {}

        for card_id, count in counts.items():
            if count >= self.core_threshold:
                core[card_id] = count
            else:
                flex_cards[card_id] = count

        return core, flex_cards

    def generate_variant(
        self,
        master_list: List[str],
        flex_pool: Optional[List[str]] = None,
        flex_variance: float = 0.5,
    ) -> List[str]:
        """Generate a randomized variant of the master list.

        Core cards are preserved exactly. Flex slots are randomized:
        - Each flex card has a `flex_variance` probability of being replaced.
        - Replacements are drawn from `flex_pool` if provided, otherwise
          from the original flex cards (reshuffled counts).

        Args:
            master_list: The original deck as a flat card ID list.
            flex_pool: Optional list of alternative card IDs (flat, with
                       repeats indicating max available copies). If None,
                       flex slots shuffle among existing flex cards only.
            flex_variance: Probability (0.0-1.0) that each flex card gets
                           replaced. 0.0 = no changes, 1.0 = maximum
                           randomization.

        Returns:
            A new deck list (same total size as master_list).
        """
        total_size = len(master_list)
        core, flex = self.analyze(master_list)

        # Start with all core cards
        variant = expand_deck_dict(core)
        flex_slots = total_size - len(variant)

        if flex_slots <= 0:
            # All cards are core — no flex slots to randomize
            return variant

        # Build the flex card pool
        if flex_pool is not None:
            available_flex = list(flex_pool)
        else:
            available_flex = expand_deck_dict(flex)

        # For each original flex card, decide whether to keep or replace
        original_flex = expand_deck_dict(flex)
        new_flex: List[str] = []

        for card_id in original_flex:
            if random.random() >= flex_variance:
                # Keep the original flex card
                new_flex.append(card_id)
            elif available_flex:
                # Replace with a random card from the pool
                idx = random.randrange(len(available_flex))
                replacement = available_flex.pop(idx)
                new_flex.append(replacement)
            else:
                # Pool exhausted — keep original
                new_flex.append(card_id)

        # Ensure we fill exactly flex_slots
        # If new_flex is short (shouldn't happen with good data), pad from pool
        while len(new_flex) < flex_slots and available_flex:
            idx = random.randrange(len(available_flex))
            new_flex.append(available_flex.pop(idx))

        # If still short, duplicate from new_flex
        while len(new_flex) < flex_slots and new_flex:
            new_flex.append(random.choice(new_flex))

        # Trim if over
        new_flex = new_flex[:flex_slots]

        variant.extend(new_flex)
        return variant

    def analyze_multiple(
        self, deck_lists: List[List[str]]
    ) -> Tuple[Dict[str, int], List[str]]:
        """Analyze multiple lists of the same archetype to find consensus core.

        Cards appearing at max copies in ALL lists are "consensus core".
        All other unique cards form the flex pool.

        Args:
            deck_lists: Multiple deck lists for the same archetype.

        Returns:
            (consensus_core, flex_pool) where consensus_core is
            {card_id: count} and flex_pool is a flat list of all flex
            card IDs across all lists.
        """
        if not deck_lists:
            return {}, []

        # Count each card across all lists
        all_counts: List[Counter] = [Counter(d) for d in deck_lists]
        all_cards = set()
        for c in all_counts:
            all_cards.update(c.keys())

        consensus_core: Dict[str, int] = {}
        flex_pool_counter: Counter = Counter()

        for card_id in all_cards:
            counts_in_lists = [c.get(card_id, 0) for c in all_counts]
            min_count = min(counts_in_lists)

            if min_count >= self.core_threshold:
                consensus_core[card_id] = min_count
            else:
                # Add all occurrences to flex pool
                max_count = max(counts_in_lists)
                flex_pool_counter[card_id] = max_count

        flex_pool = expand_deck_dict(dict(flex_pool_counter))
        return consensus_core, flex_pool


# =====================================================================
#  MetaGauntlet (Orchestrator)
# =====================================================================

class MetaGauntlet:
    """Ties MetaDistribution + DeckLibrary + VariantGenerator together.

    Used during training to sample meta-weighted opponent decks with
    optional randomization of flex slots.
    """

    def __init__(
        self,
        meta: MetaDistribution,
        library: DeckLibrary,
        variant_gen: Optional[VariantGenerator] = None,
        use_variants: bool = True,
    ):
        """
        Args:
            meta: The local meta distribution.
            library: Deck library with archetypes matching meta.
            variant_gen: Optional variant generator. Created with
                         defaults if None and use_variants is True.
            use_variants: Whether to generate randomized variants.
        """
        self.meta = meta
        self.library = library
        self.use_variants = use_variants
        self.variant_gen = variant_gen or (
            VariantGenerator() if use_variants else None
        )
        self._validate()

    def _validate(self) -> None:
        """Warn about archetypes in meta but missing from library."""
        for archetype in self.meta.archetypes:
            if self.library.deck_count(archetype) == 0:
                import warnings
                warnings.warn(
                    f"Archetype '{archetype}' is in meta distribution "
                    f"but has no decks in the library",
                    stacklevel=2,
                )

    def sample_opponent(self) -> Tuple[str, List[str]]:
        """Sample an archetype from the meta and return a deck.

        Returns:
            (archetype_name, deck_ids) tuple.

        Raises:
            KeyError: If the sampled archetype has no decks in the library.
        """
        archetype = self.meta.get_weighted_opponent()
        deck = self.library.get_deck(archetype)

        if self.use_variants and self.variant_gen is not None:
            deck = self.variant_gen.generate_variant(deck)

        return archetype, deck

    def reward_scale(self, archetype: str) -> float:
        """Get reward scaling for the given archetype.

        Returns:
            Multiplier (1.0 + meta_share). Unknown archetypes return 1.0.
        """
        return self.meta.calculate_reward_scale(archetype)


# =====================================================================
#  Standalone reward function
# =====================================================================

def calculate_reward_scale(
    opponent_archetype: str, meta: MetaDistribution
) -> float:
    """Compute reward multiplier based on meta share.

    If "Red Hybrid" is 40% of the meta, beating it grants 1.4x reward.
    If "Rogue Deck" is 1%, beating it grants 1.01x reward.

    Args:
        opponent_archetype: Name of the opponent archetype.
        meta: MetaDistribution with archetype stats.

    Returns:
        Float multiplier >= 1.0.
    """
    return meta.calculate_reward_scale(opponent_archetype)


# =====================================================================
#  MetaGauntletEnv — Gymnasium wrapper
# =====================================================================

class MetaGauntletEnv(gymnasium.Wrapper):
    """Wraps DigimonEnv to inject meta-weighted opponent decks per episode.

    On each reset():
    1. Samples an archetype from the MetaDistribution
    2. Pulls a deck from the DeckLibrary (optionally generating a variant)
    3. Passes it as deck2 to the underlying DigimonEnv
    4. Scales terminal rewards by the archetype's meta share

    The agent's deck (deck1) is configured at construction time.

    Usage:
        base_env = DigimonEnv(deck1=my_deck)
        gauntlet = MetaGauntlet(meta, library)
        env = MetaGauntletEnv(base_env, gauntlet)

        obs, info = env.reset()
        # opponent deck is now sampled from the meta
        info["opponent_archetype"]  # e.g. "Red Hybrid"
        info["reward_scale"]        # e.g. 1.4
    """

    def __init__(self, env: DigimonEnv, gauntlet: MetaGauntlet):
        super().__init__(env)
        self.gauntlet = gauntlet
        self._current_archetype: Optional[str] = None
        self._reward_scale: float = 1.0
        self._digimon_env = env

    def reset(self, **kwargs):
        """Reset with a meta-sampled opponent deck."""
        archetype, opponent_deck = self.gauntlet.sample_opponent()
        self._current_archetype = archetype
        self._reward_scale = self.gauntlet.reward_scale(archetype)

        # Inject opponent deck via options
        options = kwargs.pop("options", {}) or {}
        options["deck2"] = opponent_deck
        kwargs["options"] = options

        obs, info = self.env.reset(**kwargs)
        info["opponent_archetype"] = archetype
        info["reward_scale"] = self._reward_scale
        return obs, info

    def step(self, action):
        """Step with meta-weighted reward scaling on terminal outcomes."""
        obs, reward, terminated, truncated, info = self.env.step(action)

        # Scale terminal rewards (win/loss) by meta weight
        if terminated or truncated:
            if abs(reward) > 0.5:  # terminal ±1.0 reward
                reward *= self._reward_scale

        info["opponent_archetype"] = self._current_archetype
        info["reward_scale"] = self._reward_scale
        return obs, reward, terminated, truncated, info

    @property
    def current_archetype(self) -> Optional[str]:
        """The archetype of the current opponent."""
        return self._current_archetype
