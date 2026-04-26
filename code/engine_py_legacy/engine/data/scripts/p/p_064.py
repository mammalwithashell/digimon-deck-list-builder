from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_064(CardScript):
    """P-064 Kiyoshiro Higashimitarai"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnUseAttack
        # [Your Turn] When you attack with a Digimon that has [Jellymon] in its digivolution cards, you may suspend this Tamer to have that Digimon gain <Jamming> for the turn. (This Digimon can't be deleted in battles against Security Digimon.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("P-064 Your Attacking Digimon gains Jamming")
        effect0.set_effect_description("[Your Turn] When you attack with a Digimon that has [Jellymon] in its digivolution cards, you may suspend this Tamer to have that Digimon gain <Jamming> for the turn. (This Digimon can't be deleted in battles against Security Digimon.)")
        effect0.is_optional = True
        effect0.is_on_attack = True
        effect0._is_jamming = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Jamming"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if perm:
                perm.grant_keyword('_is_jamming')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("P-064 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
