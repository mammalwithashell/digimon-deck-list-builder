from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_08(CardScript):
    """ST18-08 Galemon | Lv.4 Green Bird Dragon/LIBERATOR

    [Security] You may play 1 card with the [LIBERATOR] trait and a play cost of
        4 or less from your hand or trash without paying the cost. <Vortex>
    Inherited [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play 1 [LIBERATOR] trait card with cost 4 or less from hand or trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name(
            "ST18-08 Play 1 [LIBERATOR] trait card cost 4 or less from hand or trash"
        )
        effect0.set_effect_description(
            "[Security] You may play 1 card with the [LIBERATOR] trait and a play "
            "cost of 4 or less from your hand or trash without paying the cost."
        )
        effect0.is_optional = True
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play [LIBERATOR] trait card cost <=4 from hand or trash free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                if 'LIBERATOR' not in traits:
                    return False
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: <Vortex> keyword factory ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-08 Vortex")
        effect1.set_effect_description("<Vortex>")
        effect1._is_vortex = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Inherited [Your Turn] This Digimon gets +2000 DP ---
        effect2 = ICardEffect()
        effect2.set_effect_name("ST18-08 +2000 DP on your turn")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
