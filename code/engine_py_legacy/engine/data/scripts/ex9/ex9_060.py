from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_060(CardScript):
    """EX9-060 Devidramon | Lv.4 Purple Evil Dragon/DM/Ver.5
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Training> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-060 Training")
        effect0.set_effect_description(
            "<Training> (In the main phase, by suspending this Digimon, "
            "place your deck's top card face down as this Digimon's bottom "
            "digivolution card.)"
        )
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving][When Attacking][Once Per Turn]
        #     Place 1 hand card as bottom digi card, Draw 1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-060 Place hand card as evo, Draw 1")
        effect1.set_effect_description(
            "[When Digivolving][Once Per Turn] By placing 1 card in your "
            "hand face down as this Digimon's bottom digivolution card, Draw 1."
        )
        effect1.is_when_digivolving = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_draw_EX9_060")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def _place_hand_and_draw(ctx: Dict[str, Any]):
            """Place 1 hand card as bottom evo, Draw 1."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return
            if not player.hand_cards or not perm:
                return

            def on_hand_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)
                player.draw_cards(1)

            game.effect_select_hand_card(
                player, lambda c: True, on_hand_selected, is_optional=True,
                prompt="Select 1 card to place face down as bottom digivolution card.")

        effect1.set_on_process_callback(_place_hand_and_draw)
        effects.append(effect1)

        # --- Effect 1b: Same but for [When Attacking] ---
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnTappedAnyone)
        effect1b.set_effect_name("EX9-060 When Attacking: Place hand card, Draw 1")
        effect1b.set_effect_description(
            "[When Attacking][Once Per Turn] By placing 1 card in your hand "
            "face down as this Digimon's bottom digivolution card, Draw 1."
        )
        effect1b.is_on_attack = True
        effect1b.set_max_count_per_turn(1)
        effect1b.set_hash_string("place_draw_EX9_060")  # shared OPT

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1b.set_can_use_condition(condition1b)

        effect1b.set_on_process_callback(_place_hand_and_draw)
        effects.append(effect1b)

        # --- Effect 2: Inherited [On Deletion]
        #     Delete 1 opponent's level 4 or lower Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX9-060 Inherited: On Deletion delete Lv4 or lower")
        effect2.set_effect_description(
            "Inherited: [On Deletion] Delete 1 of your opponent's level 4 "
            "or lower Digimon."
        )
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent Digimon Lv4 or lower"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level <= 4
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
