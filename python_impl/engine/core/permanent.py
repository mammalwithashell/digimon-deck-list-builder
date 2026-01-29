from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional
from ..data.enums import EffectTiming

if TYPE_CHECKING:
    from .card_source import CardSource
    from ..interfaces.card_effect import ICardEffect

class Permanent:
    def __init__(self, card_sources: List['CardSource']):
        self.card_sources: List['CardSource'] = card_sources
        self.is_suspended: bool = False

    @property
    def digivolution_cards(self) -> List['CardSource']:
        return self.card_sources

    @property
    def top_card(self) -> Optional['CardSource']:
        if len(self.card_sources) > 0:
            return self.card_sources[-1]
        return None

    @property
    def level(self) -> int:
        return self.top_card.level if self.top_card else 0

    @property
    def is_token(self) -> bool:
        return self.top_card.is_token if self.top_card else False

    @property
    def is_digimon(self) -> bool:
        return self.top_card.is_digimon if self.top_card else False

    @property
    def is_tamer(self) -> bool:
        return self.top_card.is_tamer if self.top_card else False

    @property
    def is_option(self) -> bool:
        return self.top_card.is_option if self.top_card else False

    @property
    def dp(self) -> int:
        return self.top_card.base_dp if self.top_card else 0

    def can_attack(self, card_effect: Optional['ICardEffect'], without_tap: bool = False, is_vortex: bool = False) -> bool:
        return True

    def can_block(self, attacking_permanent: 'Permanent') -> bool:
        return False

    def effect_list(self, timing: EffectTiming) -> List['ICardEffect']:
        return []

    def add_card_source(self, card_source: 'CardSource'):
        self.card_sources.append(card_source)

    def discard_evo_roots(self, ignore_overflow: bool = False, put_to_trash: bool = True):
        # Generator placeholder
        pass
