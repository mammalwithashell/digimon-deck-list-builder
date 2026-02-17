from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_062(CardScript):
    """P-062 Hiro Amanokawa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When you attack with a Digimon that has [Gammamon] in its digivolution cards, you may suspend this Tamer to have that Digimon gain <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)
        effect0 = ICardEffect()
        effect0.set_effect_name("P-062 Security Attack +1")
        effect0.set_effect_description("[Your Turn] When you attack with a Digimon that has [Gammamon] in its digivolution cards, you may suspend this Tamer to have that Digimon gain <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)")
        effect0.is_optional = True
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Change Security Attack"""
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
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("P-062 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
