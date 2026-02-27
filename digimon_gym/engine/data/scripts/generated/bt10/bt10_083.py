from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_083(CardScript):
    """BT10-083 Minervamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: retaliation
        # Retaliation
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-083 Retaliation")
        effect0.set_effect_description("Retaliation")
        effect0._is_retaliation = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Opponent's Turn] When an opponent plays a Digimon, you may play 1 purple level 4 or lower Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played by this effect don't activate.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-083 Play 1 level 4 or lower Digimon from trash")
        effect1.set_effect_description("[Opponent's Turn] When an opponent plays a Digimon, you may play 1 purple level 4 or lower Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played by this effect don't activate.")
        effect1.is_optional = True
        effect1.is_on_play = True

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
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If your opponent has 2 or fewer Digimon in play, you may play 1 purple level 5 or lower Digimon card or 1 [Mervamon] from your trash without paying its memory cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-083 Play 1 Digimon from trash")
        effect2.set_effect_description("[On Deletion] If your opponent has 2 or fewer Digimon in play, you may play 1 purple level 5 or lower Digimon card or 1 [Mervamon] from your trash without paying its memory cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
