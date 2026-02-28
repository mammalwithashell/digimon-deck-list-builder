from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_067(CardScript):
    """EX6-067 Final Excalibur"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Unsuspend 1 of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait. If you have [Dominimon], unsuspend all of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-067 Unsuspend")
        effect0.set_effect_description("[Main] Unsuspend 1 of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait. If you have [Dominimon], unsuspend all of your Digimon with the [Angel]/[Archangel]/[Three Great Angels] trait instead.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] [Recovery +1 <Deck>]. Then, add this card to the hand
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-067 Recovery +1, Add To Hand")
        effect1.set_effect_description("[Security] [Recovery +1 <Deck>]. Then, add this card to the hand")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
