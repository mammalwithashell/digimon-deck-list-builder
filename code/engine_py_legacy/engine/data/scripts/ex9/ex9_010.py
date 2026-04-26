from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_010(CardScript):
    """EX9-010 Tuskmon | Lv.4 | Dinosaur/DM/Ver.5
    <Training>
    [When Digivolving][When Attacking][Once Per Turn] By placing 1 card in your
    hand face down as bottom digi card, delete 1 opponent's Digimon with 4000 DP or less.
    Inherited: <Raid>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-010 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving][Once Per Turn] Place hand card, delete 4k or less
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-010 When Digivolving: Place hand, delete 4k")
        effect1.set_effect_description(
            "[When Digivolving][Once Per Turn] Place 1 hand card as bottom digi "
            "card, delete 1 opponent's Digimon with 4000 DP or less."
        )
        effect1.is_when_digivolving = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_delete_EX9_010")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def _place_hand_and_delete(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if not player.hand_cards:
                return

            def on_hand_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)

                def target_filter(p):
                    return p.is_digimon and p.dp is not None and p.dp <= 4000

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

            game.effect_select_hand_card(
                player, lambda c: True, on_hand_selected, is_optional=True,
                prompt="Select 1 card to place face down as bottom digivolution card.")
        effect1.set_on_process_callback(_place_hand_and_delete)
        effects.append(effect1)

        # [When Attacking][Once Per Turn] Same effect
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnTappedAnyone)
        effect1b.set_effect_name("EX9-010 When Attacking: Place hand, delete 4k")
        effect1b.set_effect_description(
            "[When Attacking][Once Per Turn] Place 1 hand card as bottom digi "
            "card, delete 1 opponent's Digimon with 4000 DP or less."
        )
        effect1b.is_on_attack = True
        effect1b.set_max_count_per_turn(1)
        effect1b.set_hash_string("place_delete_EX9_010")

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1b.set_can_use_condition(condition1b)

        effect1b.set_on_process_callback(_place_hand_and_delete)
        effects.append(effect1b)

        # Inherited: <Raid>
        effect2 = ICardEffect()
        effect2.set_effect_name("EX9-010 Inherited: Raid")
        effect2.set_effect_description("Inherited: <Raid>")
        effect2.is_inherited_effect = True
        effect2._is_raid = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
