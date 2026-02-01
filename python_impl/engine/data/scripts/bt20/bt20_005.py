from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource

class BT20_005(CardScript):
    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_effect_name("BT20-005 Inherited Effect")
        effect.set_effect_description("[Your Turn] When this Digimon checks a face-up security card, this Digimon gains <Jamming> for the turn.")
        effect.is_inherited_effect = True

        # We assume the engine calls this effect with EffectTiming.OnSecurityCheck

        def condition(context: Dict[str, Any]) -> bool:
            player = context.get("player")
            if not player:
                # Fallback if context is missing player
                permanent = context.get("permanent")
                if permanent and permanent.top_card:
                    player = permanent.top_card.owner

            if not player or not player.is_my_turn:
                return False

            return True

        def on_process():
            print(f"BT20-005 Activated: Digimon gains Jamming for the turn (Keyword addition not supported in engine yet).")

        effect.set_can_use_condition(condition)
        effect.set_on_process_callback(on_process)

        return [effect]
