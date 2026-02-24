from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_056(CardScript):
    """EX11-056 Ryutaro Williams"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-056 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Digimon digivolve into a level 5 or higher Digimon with [Tyrannomon] in its name or the [Dinosaur] trait, by suspending this Tamer, you may hatch in your breeding area. After, 1 of your Digimon in the breeding area may digivolve into a Digimon card with [Tyrannomon] in its name or the [Reptile] or [Dinosaur] trait in the hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-056 Hatch and digivolve in breeding")
        effect1.set_effect_description("[All Turns] When any of your Digimon digivolve into a level 5 or higher Digimon with [Tyrannomon] in its name or the [Dinosaur] trait, by suspending this Tamer, you may hatch in your breeding area. After, 1 of your Digimon in the breeding area may digivolve into a Digimon card with [Tyrannomon] in its name or the [Reptile] or [Dinosaur] trait in the hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 5:
                    return False
                if not (p.contains_card_name('Tyrannomon')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Tyrannomon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                if getattr(c, 'level', None) is None or c.level < 5:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-056 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
