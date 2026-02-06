from dataclasses import dataclass
from .enums import CardColor

@dataclass
class EvoCost:
    card_color: CardColor
    level: int
    memory_cost: int
