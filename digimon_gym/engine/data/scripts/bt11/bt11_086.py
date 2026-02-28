from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_086(CardScript):
    """BT11-086 Mervamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-086 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 purple level 4 or lower Digimon card or 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying the cost. If DigiXrosing, add 1 to the number of cards you play with this effect.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-086 Play level 4 or lower Digimon from trash")
        effect1.set_effect_description("[On Play] You may play 1 purple level 4 or lower Digimon card or 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying the cost. If DigiXrosing, add 1 to the number of cards you play with this effect.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 purple level 4 or lower Digimon card or 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying the cost. If DigiXrosing, add 1 to the number of cards you play with this effect.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-086 Play level 4 or lower Digimon from trash")
        effect2.set_effect_description("[When Digivolving] You may play 1 purple level 4 or lower Digimon card or 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying the cost. If DigiXrosing, add 1 to the number of cards you play with this effect.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: rush
        # Rush
        effect3 = ICardEffect()
        effect3.set_effect_name("BT11-086 Rush")
        effect3.set_effect_description("Rush")
        effect3._is_rush = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("BT11-086 Blocker")
        effect4.set_effect_description("Blocker")
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Effect
        effect5 = ICardEffect()
        effect5.set_effect_name("BT11-086 Effect")
        effect5.set_effect_description("Effect")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
