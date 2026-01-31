from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_044(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_044_WhenDigivolvingEffect()]

class BT22_044_WhenDigivolvingEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_when_digivolving = True
        self.effect_name = "[When Digivolving] Suspend 1 of your opponent's Digimon."

    def on_process_callback(self):
        # Suspend logic (mock target)
        print("BT22-044 Effect: Suspending opponent Digimon...")
        # In real engine, would select target.
        pass
