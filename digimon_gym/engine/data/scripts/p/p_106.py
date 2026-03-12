from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_106(CardScript):
    """P-106 Agility Training"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 2 cards of your deck. Add 1 green card among
        # them to your hand. Place the rest at the bottom of your deck in any
        # order. Then, place this card into your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-106 Reveal top 2, add 1 green card to hand")
        effect0.set_effect_description(
            "[Main] Reveal the top 2 cards of your deck. Add 1 green card among "
            "them to your hand. Place the rest at the bottom of your deck in any "
            "order. Then, place this card into your battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 2, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-106 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> — 1 of your Digimon may digivolve into a green Digimon
        # card in your hand for its digivolution cost. When it would digivolve
        # by this effect, reduce the cost by 2.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("P-106 Delay: 1 Digimon digivolves into green card, cost -2")
        effect2.set_effect_description(
            "[Main] <Delay> — 1 of your Digimon may digivolve into a green Digimon "
            "card in your hand for its digivolution cost. When it would digivolve by "
            "this effect, reduce the cost by 2."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Player selects one of their Digimon to digivolve
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Green' in colors

            def on_digimon_selected(target_perm):
                # Digivolve the selected Digimon into a green hand card, cost -2
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=2, is_optional=True)

            game.effect_select_own_permanent(
                player, on_digimon_selected,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
