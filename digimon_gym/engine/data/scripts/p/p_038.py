from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_038(CardScript):
    """P-038 Green Memory Boost!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 4 cards of your deck. Add 1 green Digimon card
        # among them to your hand. Place the remaining cards at the bottom of
        # your deck in any order. Then, place this card in your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-038 Reveal top 4, add 1 green Digimon to hand")
        effect0.set_effect_description(
            "[Main] Reveal the top 4 cards of your deck. Add 1 green Digimon card "
            "among them to your hand. Place the remaining cards at the bottom of "
            "your deck in any order. Then, place this card in your battle area."
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
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Green' not in colors:
                    return False
                return True

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-038 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> — Gain 2 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("P-038 Delay: Gain 2 memory")
        effect2.set_effect_description(
            "[Main] <Delay> — Gain 2 memory."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
