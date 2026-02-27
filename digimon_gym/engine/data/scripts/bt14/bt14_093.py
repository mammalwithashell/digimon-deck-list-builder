from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_093(CardScript):
    """BT14-093 Emissary of Hope"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-093 Main")
        effect0.set_effect_description("[Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>. ")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Complex security-search digivolution flow is not represented here.
            # Keep effect non-destructive instead of applying incorrect actions.
            return

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-093 Play Patamon, Add To Hand")
        effect1.set_effect_description("[Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                name = getattr(c, 'card_name', None) or getattr(c, 'name', '')
                return isinstance(name, str) and 'Patamon' in name

            game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)
            game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

            if player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
