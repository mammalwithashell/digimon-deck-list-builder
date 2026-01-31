from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from typing import List, TYPE_CHECKING, Dict, Any

if TYPE_CHECKING:
    from ....core.card_source import CardSource
    from ....core.permanent import Permanent

class BT22_002(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        return [BT22_002_InheritedEffect()]

class BT22_002_InheritedEffect(ICardEffect):
    def __init__(self):
        super().__init__()
        self.is_inherited_effect = True
        self.is_on_attack = True # Mocking When Attacking trigger
        self.effect_name = "Inherited: [When Attacking] Draw 1."

    def can_trigger(self, hashtable: Dict[str, Any]) -> bool:
        # Simple mock: Always trigger if it's the attacking permanent
        return True # Real engine would check hashtable["permanent"] == self.effect_source_permanent

    def on_process_callback(self):
        # Mock Draw
        if self.effect_source_permanent and self.effect_source_permanent.top_card and self.effect_source_permanent.top_card.owner:
            self.effect_source_permanent.top_card.owner.draw()
