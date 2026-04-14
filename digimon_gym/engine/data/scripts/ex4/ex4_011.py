from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_011(CardScript):
    """EX4-011 ChaosGallantmon | Lv.6 Red/Purple Digimon

    Alt digivolve: from [WarGrowlmon] Lv5 for cost 3.

    [Trash] [End of Your Turn] By Deleting 1 of your Digimon with a
        digivolution card and [Gallantmon] in its name, you may play this
        card without paying its cost.

    [On Play] Delete 1 of your opponent's Digimon with 7000 DP or less.
        For every 10 total cards in both player's trashes, add 2000 to
        the maximum DP you can choose with DP-based deletion effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [WarGrowlmon] Lv5 for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX4-011 Alternate digivolution requirement")
        effect0.set_effect_description(
            "Alternate digivolution: from [WarGrowlmon] Lv5 for cost 3")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "WarGrowlmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('WarGrowlmon')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Trash] [End of Your Turn] Delete Gallantmon w/ digi-cards,
        #     play this card from trash free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("EX4-011 Trash EOT: play from trash")
        effect1.set_effect_description(
            "[Trash] [End of Your Turn] By Deleting 1 of your Digimon with a "
            "digivolution card and [Gallantmon] in its name, you may play this "
            "card without paying its cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be in trash (not on field)
            if card and card.permanent_of_this_card() is not None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must be your turn
            if not player.is_my_turn:
                return False
            # Card must be in trash
            if card not in player.trash_cards:
                return False
            # Must have a Gallantmon-named Digimon with digivolution cards on field
            has_target = any(
                p.is_digimon
                and p.contains_card_name('Gallantmon')
                and not p.has_no_digivolution_cards
                for p in player.battle_area
            )
            return has_target
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Delete 1 Gallantmon w/ digi-cards, then play self from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def gallantmon_filter(p):
                return (p.is_digimon
                        and p.contains_card_name('Gallantmon')
                        and not p.has_no_digivolution_cards)

            def on_select(target_perm):
                if target_perm is None:
                    return
                # Deletion is a cost — only play if deletion actually succeeds
                # (e.g., Evade/Armor Purge may prevent it)
                deleted = player.delete_permanent(target_perm)
                if not deleted:
                    return
                # Play this card from trash without paying cost
                if card and card in player.trash_cards:
                    played = player.play_card_from_source(card, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": card, "played_permanent": played,
                             "event_player": player, "is_effect_play": True})

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=gallantmon_filter,
                is_optional=True,
                prompt="Select 1 of your [Gallantmon] with digivolution cards to delete."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Play] Delete 1 opp Digimon with scaled DP threshold ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX4-011 On Play: DP-based deletion")
        effect2.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with 7000 DP or "
            "less. For every 10 total cards in both player's trashes, add "
            "2000 to the maximum DP."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with DP <= 7000 + 2000*(total trash / 10)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Calculate max DP threshold
            total_trash = len(player.trash_cards) + len(enemy.trash_cards)
            max_dp = 7000 + 2000 * (total_trash // 10)

            def delete_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= max_dp

            has_target = any(delete_filter(p) for p in enemy.battle_area)
            if not has_target:
                return

            def on_delete(target_perm):
                if target_perm:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete,
                filter_fn=delete_filter,
                is_optional=False,
                prompt=f"Delete 1 opponent Digimon with {max_dp} DP or less."
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
