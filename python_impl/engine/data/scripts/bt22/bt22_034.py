from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_034(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_034_InheritedEffect()]

class BT22_034_InheritedEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_inherited_effect = True
        self.effect_name = "Inherited: [Your Turn] This Digimon gets +1000 DP."

    def get_change_dp_value(self, permanent: 'Permanent') -> int:
        if permanent.top_card.owner and permanent.top_card.owner.is_my_turn:
            return 1000
        return 0
