from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_093(CardScript):
    """BT3-093 Davis Motomiya | Tamer (Blue, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] Reveal the top 3 cards of your deck. Add 1 blue and 1 green
        Digimon card among them to your hand. Place the remaining cards at
        the bottom of your deck in any order.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT3-093 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Reveal top 3, add 1 blue + 1 green Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT3-093 Reveal top 3, add blue+green Digimon")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 blue and "
            "1 green Digimon card among them to your hand. Place the remaining "
            "cards at the bottom of your deck in any order."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def blue_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Blue' in colors

            def green_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Green' in colors

            game.effect_reveal_and_select_multi(
                player, 3,
                [(blue_digimon_filter, 'hand'), (green_digimon_filter, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play without paying cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT3-093 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
