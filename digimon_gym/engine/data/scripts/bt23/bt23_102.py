from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_102(CardScript):
    """BT23-102 Mastemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-102 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 5
        effect0._alt_digi_color = CardColor.Yellow

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-102 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: barrier
        # Barrier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-102 Barrier")
        effect2.set_effect_description("Barrier")
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: partition
        # Partition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-102 Partition")
        effect3.set_effect_description("Partition")
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 5 or lower yellow or purple card from your hand or trash without paying the cost. Then, if this Digimon's stack has 2 or more same-level cards, trash the top cards of both players' security stacks so that they have 3 cards left.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-102 Play 1 level 5 or lower yellow/purple card from hand or trash. then if digimon has 2+ same level cards in stack, trash both security till 3")
        effect4.set_effect_description("[When Digivolving] You may play 1 level 5 or lower yellow or purple card from your hand or trash without paying the cost. Then, if this Digimon's stack has 2 or more same-level cards, trash the top cards of both players' security stacks so that they have 3 cards left.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                return 'Yellow' in colors or 'Purple' in colors
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When security stacks are removed from, you may place 1 Digimon as the bottom security card.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT23-102 Place 1 digimon as bottom security")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When security stacks are removed from, you may place 1 Digimon as the bottom security card.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_102_AT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Put To Security, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place a permanent into the security stack
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security, filter_fn=target_filter, is_optional=True)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
