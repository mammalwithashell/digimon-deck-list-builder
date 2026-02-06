from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_066(CardScript):
    """Auto-transpiled from DCGO BT24_066.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Among them, add 1 [Evil], [Dark Dragon], [Evil Dragon] or [Dark Knight] trait card or purple Tamer card to the hand and trash 1 such card. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-066 Reveal 3 from deck. Add 2. Return the rest to bot deck. Trash 1 from hand.")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Among them, add 1 [Evil], [Dark Dragon], [Evil Dragon] or [Dark Knight] trait card or purple Tamer card to the hand and trash 1 such card. Return the rest to the bottom of the deck. Then, trash 1 card in your hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Delete 1 of your opponent's level 3 Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-066 Delete opponents level 3 Digimon")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] Delete 1 of your opponent's level 3 Digimon.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_066_Inherited")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
