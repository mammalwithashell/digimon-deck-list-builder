from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_061(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_061_OnDeletionEffect()]

class BT22_061_OnDeletionEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_on_deletion = True
        self.effect_name = "[On Deletion] Delete 1 of your opponent's Level 3 Digimon."

    def on_process_callback(self):
        print("BT22-061 Effect: Delete 1 opponent Level 3 Digimon.")
        pass
