from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_024(CardScript):
    """BT21-024 Cyberdramon | Lv.5

    [On Play] [When Digivolving] If your opponent has 5 or fewer security cards,
    they place 1 card from their hand as the bottom security card.
    Then, trash their top security card.
    Inherited: [Your Turn] This Digimon gets +4000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _shared_process(ctx: Dict[str, Any]):
            """If opponent has 5 or fewer security, they place 1 hand card as bottom security.
            Then, trash their top security card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Check: opponent has 5 or fewer security cards
            if len(enemy.security_cards) > 5:
                return
            # Opponent places 1 card from their hand as bottom security card
            if enemy.hand_cards:
                # Auto-select first hand card for the opponent (engine limitation)
                hand_card = enemy.hand_cards[0]
                enemy.hand_cards.remove(hand_card)
                enemy.security_cards.insert(0, hand_card)  # bottom = index 0
            # Then, trash their top security card
            if enemy.security_cards:
                top_sec = enemy.security_cards.pop()  # top = last element
                enemy.trash_cards.append(top_sec)

        # [On Play]
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT21-024 Opponent places hand card as security then trashes top security")
        effect0.set_effect_description("[On Play] If your opponent has 5 or fewer security cards, they place 1 card from their hand as the bottom security card. Then, trash their top security card.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_shared_process)
        effects.append(effect0)

        # [When Digivolving]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT21-024 Opponent places hand card as security then trashes top security")
        effect1.set_effect_description("[When Digivolving] If your opponent has 5 or fewer security cards, they place 1 card from their hand as the bottom security card. Then, trash their top security card.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_process)
        effects.append(effect1)

        # Inherited: [Your Turn] This Digimon gets +4000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-024 DP modifier")
        effect2.set_effect_description("[Your Turn] This Digimon gets +4000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 4000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
