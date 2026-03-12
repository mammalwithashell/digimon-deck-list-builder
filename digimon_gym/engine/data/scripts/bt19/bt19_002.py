from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_002(CardScript):
    """BT19-002 Puyoyomon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnUseAttack
        # [Opponent's Turn] When any of your opponent's Digimon attack, by returning this Digimon with [Aqua]/[Sea Animal] in one of its traits to the bottom of the deck, return 1 of your opponent's Digimon with as high or lower a level as the returned Digimon to the hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT19-002 Bottom deck this Digimon to return to hand an opponent's Digimon with the same level")
        effect0.set_effect_description("[Opponent's Turn] When any of your opponent's Digimon attack, by returning this Digimon with [Aqua]/[Sea Animal] in one of its traits to the bottom of the deck, return 1 of your opponent's Digimon with as high or lower a level as the returned Digimon to the hand.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
