from __future__ import annotations
from typing import TYPE_CHECKING, List
from abc import ABC, abstractmethod

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..interfaces.card_effect import ICardEffect

class CardScript(ABC):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return []
