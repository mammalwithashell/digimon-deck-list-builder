from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_015(CardScript):
    """BT10-015 Shoutmon X5B | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-015 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: armor_purge
        # Armor Purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-015 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Digimon card with [Xros Heart] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Beelzemon] is in this Digimon's digivolution cards, you may play 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying its cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT10-015 Place 1 card to digivolution cards and play 1 Digimon from trash")
        effect2.set_effect_description("[On Play] You may place 1 Digimon card with [Xros Heart] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Beelzemon] is in this Digimon's digivolution cards, you may play 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying its cost.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Digimon card with [Xros Heart] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Beelzemon] is in this Digimon's digivolution cards, you may play 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying its cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT10-015 Place 1 card to digivolution cards and play 1 Digimon from trash")
        effect3.set_effect_description("[When Digivolving] You may place 1 Digimon card with [Xros Heart] in its traits from your hand or from under one of your Tamers under this Digimon as its bottom digivolution card. Then, if [Beelzemon] is in this Digimon's digivolution cards, you may play 1 level 4 or lower Digimon card with [Xros Heart] in its traits from your trash without paying its cost.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT10-015 Effect")
        effect4.set_effect_description("Effect")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
