from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_08(CardScript):
    """ST18-08 Galemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Security] You may play 1 card with the [LIBERATOR] trait and a play
        # cost of 4 or less from your hand or trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-08 Security: Play 1 LIBERATOR cost 4 or less")
        effect0.set_effect_description(
            "[Security] You may play 1 card with the [LIBERATOR] trait and a play "
            "cost of 4 or less from your hand or trash without paying the cost."
        )
        effect0.is_security_effect = True
        effect0.is_optional = True

        def condition_sec(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition_sec)

        def process_sec(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                cost = getattr(c, 'play_cost', 99) or 99
                return any('LIBERATOR' in t for t in traits) and cost <= 4

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process_sec)
        effects.append(effect0)

        # <Vortex>
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-08 Vortex")
        effect1.set_effect_description("Vortex")
        effect1._is_vortex = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Inherited: [Your Turn] This Digimon gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("ST18-08 +2000 DP")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
