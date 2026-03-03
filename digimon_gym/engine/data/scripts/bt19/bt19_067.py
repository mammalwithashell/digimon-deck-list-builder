from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_067(CardScript):
    """BT19-067 Impmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 1 or fewer Tamers, you may play 1 purple Tamer with a play cost of 4 or less from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT19-067 Play 1 purple Tamer from trash")
        effect0.set_effect_description("[On Play] If you have 1 or fewer Tamers, you may play 1 purple Tamer with a play cost of 4 or less from your trash without paying the cost.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
            return tamer_count <= 1

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                play_cost = getattr(c, 'play_cost', getattr(c, 'get_cost_itself', 0))
                if play_cost > 4:
                    return False
                return 'Purple' in [col.name for col in getattr(c, 'card_colors', [])]

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: retaliation
        # Retaliation
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-067 Retaliation")
        effect1.set_effect_description("Retaliation")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1._is_retaliation = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
