from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_097(CardScript):
    """BT14-097 Suka's Curse"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-097 Also treated as having [Sukamon] in its name")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your non-white Digimon may digivolve into a Digimon card with [Sukamon] in its name in your hand without paying the cost, ignoring its digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-097 Digivolve")
        effect1.set_effect_description("[Main] 1 of your non-white Digimon may digivolve into a Digimon card with [Sukamon] in its name in your hand without paying the cost, ignoring its digivolution requirements.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Sukamon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Until the end of your turn, change 1 of your opponent's Digimon into being white and having 3000 DP and an original name of [Sukamon].
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-097 Original card name is [Sukamon]")
        effect2.set_effect_description("[Security] Until the end of your turn, change 1 of your opponent's Digimon into being white and having 3000 DP and an original name of [Sukamon].")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
