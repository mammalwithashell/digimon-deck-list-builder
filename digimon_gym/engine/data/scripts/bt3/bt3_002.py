from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_002(CardScript):
    """BT3-002 DemiVeemon | Lv.2 Digi-Egg (Blue)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Inherited Effect: [When Attacking] [Once Per Turn]
        # If this Digimon has <Jamming>, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT3-002 Inherited: Draw 1 if Jamming")
        effect0.set_effect_description(
            "[When Attacking][Once Per Turn] If this Digimon has <Jamming>, "
            "<Draw 1>. (Draw 1 card from your deck.)"
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT3_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Check that this Digimon has Jamming
            if not perm.has_keyword('_is_jamming'):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
