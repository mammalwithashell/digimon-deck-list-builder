from dataclasses import dataclass, field
from typing import Optional, List
from .enums import CardColor


@dataclass
class EvoCost:
    card_color: CardColor
    level: int
    memory_cost: int


@dataclass
class DnaRequirement:
    """One half of a DNA Digivolution requirement.

    Specifies the color, level, and optional name constraint for one
    of the two Digimon needed for DNA Digivolution.
    """
    level: int
    card_color: Optional[CardColor] = None
    name_contains: str = ""


@dataclass
class DnaCost:
    """Full DNA Digivolution requirement.

    requirement1 is the 'first listed' Digimon (its sources go on TOP).
    requirement2 is the 'second listed' Digimon (its sources go on BOTTOM).
    """
    requirement1: DnaRequirement
    requirement2: DnaRequirement
    memory_cost: int = 0
