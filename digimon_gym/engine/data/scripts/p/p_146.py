from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_146(CardScript):
    """P-146 Recharge Plug-In Q"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-146 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] 1 of your opponent's Digimon gains <Security A. -1> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-146 1 opponent's digimon gains <Security A. -1> for the turn")
        effect1.set_effect_description("[Security] 1 of your opponent's Digimon gains <Security A. -1> for the turn.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Place this card as 1 of your non-white Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-146 Place as bottom Digivolution source")
        effect2.set_effect_description("[Main] Place this card as 1 of your non-white Digimon's bottom digivolution card.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When this Digimon would be deleted by battle, by placing 1 [Reload Plug-In Q] from this Digimon's digivolution cards at the bottom of your security stack, prevent that deletion.
        effect3 = ICardEffect()
        effect3.set_effect_name("P-146 Place card from digivolution sources to security, to prevent deletion by battle")
        effect3.set_effect_description("[All Turns] When this Digimon would be deleted by battle, by placing 1 [Reload Plug-In Q] from this Digimon's digivolution cards at the bottom of your security stack, prevent that deletion.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
