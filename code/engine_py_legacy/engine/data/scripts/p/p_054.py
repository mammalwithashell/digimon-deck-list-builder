from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_054(CardScript):
    """P-054 Seraphimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have a Tamer in play, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-054 Recovery +1 (Deck)")
        effect0.set_effect_description("[When Digivolving] If you have a Tamer in play, <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("P-054 Recovery +1 (Deck)")
        effect1.set_effect_description("[On Deletion] <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
