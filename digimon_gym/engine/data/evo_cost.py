from dataclasses import dataclass, field
from typing import Optional, List
from .enums import CardColor

# ─── DigiXros / Assembly ─────────────────────────────────────────────


@dataclass
class EvoCost:
    card_color: CardColor
    level: int
    memory_cost: int


@dataclass
class DnaRequirement:
    """One half of a DNA Digivolution requirement.

    Specifies the color(s), level, and optional name/text constraint for
    one of the two Digimon needed for DNA Digivolution.

    ``card_colors`` is a list because printed DNA reqs can be slash-color
    (e.g. "Blue/Purple Lv.6" — either a Blue or a Purple material
    satisfies the half). An empty list means "any color" (level-or-name
    gated only). A single-entry list is the common case.

    ``name_contains`` matches against the card's name (w/[X] in name).
    ``text_contains`` matches against the card's effect text (w/[X] in text).
    """
    level: int
    card_colors: List[CardColor] = field(default_factory=list)
    name_contains: str = ""
    text_contains: str = ""


@dataclass
class DnaCost:
    """Full DNA Digivolution requirement.

    requirement1 is the 'first listed' Digimon (its sources go on TOP).
    requirement2 is the 'second listed' Digimon (its sources go on BOTTOM).
    """
    requirement1: DnaRequirement
    requirement2: DnaRequirement
    memory_cost: int = 0


@dataclass
class DigiXrosElement:
    """One material requirement in a DigiXros condition."""
    name_contains: str = ""
    trait_match: str = ""
    trait_alternatives: List[str] = field(default_factory=list)
    level_max: Optional[int] = None
    count: int = 1
    is_digimon_only: bool = True
    color: Optional[CardColor] = None


@dataclass
class DigiXrosCost:
    """Full DigiXros/Assembly requirement for a card."""
    elements: List[DigiXrosElement] = field(default_factory=list)
    reduce_cost_per_card: int = 0
    max_materials: int = -1
    different_card_numbers: bool = False
    different_names: bool = False
    has_text: str = ""
    source_zones: List[str] = field(default_factory=lambda: ['hand', 'field'])
