from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_097(CardScript):
    """Auto-transpiled from DCGO BT14_097.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-097 Also treated as having [Sukamon] in its name")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your non-white Digimon may digivolve into a Digimon card with [Sukamon] in its name in your hand without paying the cost, ignoring its digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-097 Digivolve")
        effect1.set_effect_description("[Main] 1 of your non-white Digimon may digivolve into a Digimon card with [Sukamon] in its name in your hand without paying the cost, ignoring its digivolution requirements.")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve non-white Digimon into [Sukamon] without cost"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor
            def is_non_white(p):
                if not p.is_digimon:
                    return False
                if p.top_card:
                    colors = getattr(p.top_card, 'card_colors', [])
                    return CardColor.White not in colors
                return True
            def on_perm_selected(target_perm):
                def is_sukamon(c):
                    if not c.is_digimon:
                        return False
                    return any('Sukamon' in n for n in c.card_names)
                game.effect_digivolve_from_hand(
                    player, target_perm, is_sukamon,
                    cost_override=0, ignore_requirements=True, is_optional=True)
            game.effect_select_own_permanent(
                player, on_perm_selected, filter_fn=is_non_white, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Until the end of your turn, change 1 of your opponent's Digimon into being white and having 3000 DP and an original name of [Sukamon].
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-097 Original card name is [Sukamon]")
        effect2.set_effect_description("[Security] Until the end of your turn, change 1 of your opponent's Digimon into being white and having 3000 DP and an original name of [Sukamon].")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
