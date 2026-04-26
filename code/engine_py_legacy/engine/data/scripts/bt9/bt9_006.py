from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_006(CardScript):
    """BT9-006 Pagumon | Lv.2 Digi-Egg
    Inherited Effect [When Attacking] You may trash 1 card from your hand
    to give this Digimon +1000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited [When Attacking] Trash 1 hand card for +1000 DP
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("BT9-006 Inherited: Trash hand card for +1000 DP")
        effect0.set_effect_description(
            "Inherited: [When Attacking] You may trash 1 card from your hand "
            "to give this Digimon +1000 DP for the turn."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            if player.hand_cards and perm:
                trashed = player.hand_cards.pop()
                player.trash_cards.append(trashed)
                perm.dp_modifier += 1000
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
