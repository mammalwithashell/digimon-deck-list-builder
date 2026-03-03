from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_206(CardScript):
    """P-206 Digital Gate Open"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("P-206 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-206 Reveal 3")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 Digimon card and 1 Tamer card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_0(c):
                return getattr(c, 'is_digimon', False)

            def reveal_filter_1(c):
                return getattr(c, 'is_tamer', False)

            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_effect_name("P-206 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name(
            "P-206 Play 1 tamer with same colour as any of your digimon on the field, from your hand with 4 reduced cost"
        )
        effect3.set_effect_description(
            "[Main] <Delay>. You may play 1 Tamer card with the same color as any of your Digimon on the field from your hand with the play cost reduced by 4."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return getattr(c, 'is_tamer', False)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=False, manual_reduction=4, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name(
            "P-206 you may play 1 3 cost or less digimon from hand or trash, then add this card to hand"
        )
        effect4.set_effect_description(
            "[Security] You may play 1 Digimon card with a play cost of 3 or less from your hand or trash without paying the cost. Then, add this card to the hand."
        )
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                return getattr(c, 'get_cost_itself', 0) <= 3

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
