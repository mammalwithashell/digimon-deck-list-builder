from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_005_OnDeletionEffect()]

class BT22_005_OnDeletionEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_on_deletion = True
        self.effect_name = "[On Deletion] Gain 1 Memory."

    def on_process_callback(self):
        # Mock memory gain
        if self.effect_source_permanent and self.effect_source_permanent.top_card and self.effect_source_permanent.top_card.owner:
            self.effect_source_permanent.top_card.owner.memory += 1
