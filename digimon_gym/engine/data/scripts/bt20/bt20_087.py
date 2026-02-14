from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_087(CardScript):
    """BT20-087 Kota Domoto & Yuji Musya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-087 [Rule] Name: Also treated as [Kota Domoto]/[Yuji Musya].")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: set_memory_3
        # Set memory to 3
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-087 Set memory to 3")
        effect1.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When one of your [Chronicle] trait Digimon attacks by suspending this Tamer, 1 of your Digimon on the field may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand with the digivolution cost reduced by 1.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-087 Digivolve 1 of your Digimon")
        effect2.set_effect_description("[Your Turn] When one of your [Chronicle] trait Digimon attacks by suspending this Tamer, 1 of your Digimon on the field may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand with the digivolution cost reduced by 1.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 6:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-087 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
