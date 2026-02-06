from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_093(CardScript):
    """Auto-transpiled from DCGO BT24_093.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Add your top security card to the hand and <Recovery +1 (Deck)>. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-093 Top sec to hand, Recovery +1, place in battle area.")
        effect0.set_effect_description("[Main] Add your top security card to the hand and <Recovery +1 (Deck)>. Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Recovery +1, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.recovery(1)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When your security stack is removed, <Delay>.\r\n• You may place the top stacked card of any your Digimon with [Aegiochusmon] or [Jupitermon] in their names as the top security card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-093 Place top card [Aegiochusmon] or [Jupitermon] on top sec.")
        effect1.set_effect_description("[All Turns] When your security stack is removed, <Delay>.\r\n• You may place the top stacked card of any your Digimon with [Aegiochusmon] or [Jupitermon] in their names as the top security card.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Aegiomon] or [Elecmon] from your hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-093 Play 1 [Aegiomon]/[Elecmon] from hand or trash.")
        effect2.set_effect_description("[Security] You may play 1 [Aegiomon] or [Elecmon] from your hand or trash without paying the cost.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
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
