from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_005(CardScript):
    """BT13-005 Dorimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If this Digimon has 4 or more digivolution cards, <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-005 Draw 1")
        effect0.set_effect_description("[When Attacking] If this Digimon has 4 or more digivolution cards, <Draw 1>. (Draw 1 card from your deck.)")
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Get the permanent from context or from effect if not present in context.
            permanent = context.get('permanent', None)
            if permanent is None:
                permanent = getattr(effect, 'effect_source_permanent', None)
            if not (permanent and hasattr(permanent, 'digivolution_cards') and len(permanent.digivolution_cards) >= 4):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
