from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_096(CardScript):
    """P-096 Prism Garrett"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-096 Ignore color requirements")
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

        # Timing: EffectTiming.OptionSkill
        # [Main] By placing up to 2 Digimon cards with <Save> in their text from under your Tamers or your trash under 1 of your Digimon with <Save> in its text as its bottom digivolution cards, that Digimon gains +1000 DP for each card placed by this effect.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-096 Effect")
        effect1.set_effect_description("[Main] By placing up to 2 Digimon cards with <Save> in their text from under your Tamers or your trash under 1 of your Digimon with <Save> in its text as its bottom digivolution cards, that Digimon gains +1000 DP for each card placed by this effect.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Add this card to its owner's hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-096 Add To Hand")
        effect2.set_effect_description("[Security] Add this card to its owner's hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
