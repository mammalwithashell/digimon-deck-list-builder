from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_052(CardScript):
    """EX6-052 Bastemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: scapegoat
        # Scapegoat
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-052 Scapegoat")
        effect0.set_effect_description("Scapegoat")
        effect0._is_scapegoat = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 purple level 3 Digimon from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-052 Play 1 level 3 purple Digimon from your trash")
        effect1.set_effect_description("[When Digivolving] You may play 1 purple level 3 Digimon from your trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Opponent's Turn] [Once per turn] When an opponent's Digimon is deleted, you may play 1 purple level 4 or lower Digimon from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-052 Play 1 level 4 or lower purple Digimon from your trash")
        effect2.set_effect_description("[Opponent's Turn] [Once per turn] When an opponent's Digimon is deleted, you may play 1 purple level 4 or lower Digimon from your trash without paying the cost.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayLevel4_EX6_052")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
