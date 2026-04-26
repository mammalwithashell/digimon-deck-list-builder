from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_070(CardScript):
    """P-070 Dorumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.SecuritySkill
        # [Security] At the end of the battle, reveal the top card of your deck. If it�fs a black Digimon card with a play cost of 4 or less, you may play it without paying its memory cost. Add the remaining cards to your hand. Then, add this card to its owner�fs hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("P-070 Reveal the top card and add this card to hand")
        effect0.set_effect_description("[Security] At the end of the battle, reveal the top card of your deck. If it�fs a black Digimon card with a play cost of 4 or less, you may play it without paying its memory cost. Add the remaining cards to your hand. Then, add this card to its owner�fs hand.")
        effect0.is_security_effect = True
        effect0.is_security_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # Reveal the top card of your deck. If it�fs a black Digimon card with a play cost of 4 or less, you may play it without paying its memory cost. Add the remaining cards to your hand. Then, add this card to its owner�fs hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("P-070 Reveal the top card and add this card to hand")
        effect1.set_effect_description("Reveal the top card of your deck. If it�fs a black Digimon card with a play cost of 4 or less, you may play it without paying its memory cost. Add the remaining cards to your hand. Then, add this card to its owner�fs hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 4:
                    return False
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
