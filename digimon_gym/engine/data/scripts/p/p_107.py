from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_107(CardScript):
    """P-107 Defense Training | Option (Black, Cost 2)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Reveal top 2, add 1 black card to hand, rest to bottom.
        #    Then place this card in the battle area. ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-107 Reveal top 2, add 1 black card to hand, rest to bottom")
        effect0.set_effect_description(
            "[Main] Reveal the top 2 cards of your deck. Add 1 black card among them to "
            "your hand. Place the rest at the bottom of your deck in any order. Then, "
            "place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Filter: any black card (Digimon, Tamer, or Option)
            def reveal_filter(c):
                from ....data.enums import CardColor
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Black in colors

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 2, reveal_filter, on_revealed, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("P-107 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Main] <Delay> 1 of your Digimon may digivolve into a black Digimon
        #    card in your hand for its digivolution cost reduced by 2. ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name(
            "P-107 1 Digimon may digivolve into a black Digimon in hand, cost -2"
        )
        effect2.set_effect_description(
            "[Main] <Delay> (By trashing this card after the placing turn, activate the "
            "effect below.) 1 of your Digimon may digivolve into a black Digimon card in "
            "your hand for its digivolution cost. When it would digivolve by this effect, "
            "reduce the cost by 2."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                from ....data.enums import CardColor
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Black in colors

            game.effect_digivolve_from_hand(
                player, perm, digi_filter,
                cost_reduction=2, is_optional=True
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
