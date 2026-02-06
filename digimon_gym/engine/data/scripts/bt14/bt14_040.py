from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_040(CardScript):
    """Auto-transpiled from DCGO BT14_040.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Tamer card from your hand on top of your security stack. 
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect0.set_effect_description("[On Play] You may place 1 Tamer card from your hand on top of your security stack. ")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Tamer card from your hand on top of your security stack. 
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect1.set_effect_description("[When Digivolving] You may place 1 Tamer card from your hand on top of your security stack. ")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon card from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-040 Play 1 Digimon from hand")
        effect2.set_effect_description("[All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon card from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Play1Digimon_Bt14_040")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
