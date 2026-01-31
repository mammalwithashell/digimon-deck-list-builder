from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_005_OnPlayEffect()]

class BT22_005_OnPlayEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_on_play = True
        self.effect_name = "[On Play] Reveal the top 3 cards of your deck."

    def on_process_callback(self):
        if self.effect_source_permanent and self.effect_source_permanent.top_card and self.effect_source_permanent.top_card.owner:
            # Reveal 3
            revealed = self.effect_source_permanent.top_card.owner.reveal_top(3)
            # Logic to add to hand would go here (complex selection UI needed)
            pass
