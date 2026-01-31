from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_003(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_003_OnPlayEffect()]

class BT22_003_OnPlayEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_on_play = True
        self.effect_name = "[On Play] Reveal the top 3 cards of your deck. Add 1 card to your hand."

    def on_process_callback(self):
        # Mock reveal and add
        # Real implementation would use player.reveal_cards, player.add_to_hand, etc.
        pass
