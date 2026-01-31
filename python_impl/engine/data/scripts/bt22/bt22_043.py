from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_043(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_043_StartOfTurnEffect()]

class BT22_043_StartOfTurnEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        # self.is_start_of_turn = True # Need to add this property if it doesn't exist
        self.effect_name = "[Start of Turn] Gain 1 Memory if you have a Tamer."

    def can_trigger(self, hashtable: Dict[str, Any]) -> bool:
        # Check Tamer condition
        return False # Placeholder

    def on_process_callback(self):
        if self.effect_source_permanent and self.effect_source_permanent.top_card and self.effect_source_permanent.top_card.owner:
            self.effect_source_permanent.top_card.owner.gain_memory(1)
