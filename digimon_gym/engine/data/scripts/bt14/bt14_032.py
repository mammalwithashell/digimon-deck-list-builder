from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_032(CardScript):
    """BT14-032 Chuumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Add the top card of your security stack to the hand.
        # Then, you may place 1 card with [Sukamon] in its name from your hand on top of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-032 Return top security to hand, then may place Sukamon from hand to security")
        effect0.set_effect_description("[On Play] Add the top card of your security stack to the hand. Then, you may place 1 card with [Sukamon] in its name from your hand on top of your security stack.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            # Add the top card of your security stack to hand.
            if getattr(player, 'security_cards', None):
                if len(player.security_cards) > 0:
                    sec_card = player.security_cards.pop(0)
                    player.hand_cards.append(sec_card)

            # You may place 1 card with [Sukamon] in its name from hand on top of security.
            hand = getattr(player, 'hand_cards', [])
            sukamon_idx = None
            for i, c in enumerate(hand):
                name = getattr(c, 'card_name', None) or getattr(c, 'name', None) or ''
                if 'Sukamon' in str(name):
                    sukamon_idx = i
                    break

            if sukamon_idx is not None:
                chosen = hand.pop(sukamon_idx)
                player.security_cards.insert(0, chosen)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-032 DP -3000")
        effect1.set_effect_description("[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.dp_modifier = -3000

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
