from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_023(CardScript):
    """P-023 Patamon's Confession"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] If you have [T.K. Takaishi] in play, place 1 of your [Patamon] at the bottom of your security stack face down. Trash that Digimon's digivolution cards.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-023 Put To Security")
        effect0.set_effect_description("[Main] If you have [T.K. Takaishi] in play, place 1 of your [Patamon] at the bottom of your security stack face down. Trash that Digimon's digivolution cards.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Put To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place a permanent into the security stack
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Add this card to its owner's hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-023 Add To Hand")
        effect1.set_effect_description("[Security] Add this card to its owner's hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
