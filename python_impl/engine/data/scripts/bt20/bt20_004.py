from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_004(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # TODO: Implement effect for BT20_004
        # This is a placeholder. Card data (stats, effects) is missing and needs to be filled.
        return []
