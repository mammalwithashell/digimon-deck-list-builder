from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_082(CardScript):
    """BT15-082 Sora Takenouchi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-082 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnReturnCardsToHandFromTrash
        # [All Turns] When a red Digimon card returns from your trash to the hand, by returning this Tamer to the hand, you may play 1 13000 DP or less red Digimon card with [Avian], [Bird], [Beast], [Animal] or [Sovereign], other than [Sea Animal] in one of its traits from your hand without paying the cost. For each of your opponent's security cards, remove 2000 from this effect's playable card's DP maximum.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-082 Return this Tamer to your hand to play a Digimon from your hand.")
        effect1.set_effect_description("[All Turns] When a red Digimon card returns from your trash to the hand, by returning this Tamer to the hand, you may play 1 13000 DP or less red Digimon card with [Avian], [Bird], [Beast], [Animal] or [Sovereign], other than [Sea Animal] in one of its traits from your hand without paying the cost. For each of your opponent's security cards, remove 2000 from this effect's playable card's DP maximum.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not ('Red' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-082 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
