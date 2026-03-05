from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_083(CardScript):
    """BT13-083 Gizmon: AT | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When you would play this card, by deleting 1 of your level 3 Digimon, reduce the play cost by 4.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT13-083 Delete your 1 level 3 Digimon to get Play Cost -4")
        effect0.set_effect_description("When you would play this card, by deleting 1 of your level 3 Digimon, reduce the play cost by 4.")
        effect0.is_optional = True
        effect0.set_hash_string("PlayCost-4_BT13_083")
        effect0.cost_reduction = 4

        def condition0(context: Dict[str, Any]) -> bool:
            # Requires a Lv3 Digimon on your field to delete
            if context.get('card_source') is not card:
                return False
            if not card or not card.owner:
                return False
            return any(p.is_digimon and p.level == 3 for p in card.owner.battle_area)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete 1 of your Lv3 Digimon as cost for -4 play cost reduction."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon and p.level == 3
            def on_delete(target_perm):
                game.delete_permanent(target_perm, player)
            game.effect_select_own_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # effect1 removed: was duplicate unconditional cost_reduction = 4
        # The cost reduction is handled by effect0 (BeforePayCost) which requires
        # deleting a Lv3 Digimon as a cost.

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Draw 2>. Then, trash 2 cards in your hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-083 Draw 2 and trash 2 cards from hand")
        effect2.set_effect_description("[On Play] <Draw 2>. Then, trash 2 cards in your hand.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed_second(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                if player.hand_cards:
                    game.effect_select_hand_card(
                        player, hand_filter, on_trashed_second,
                        is_optional=False,
                        prompt="Select a second card to trash.")
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False,
                prompt="Select a card to trash.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By returning 2 cards with [Gizmon] in their names from your trash to the bottom of the deck in any order, you may play 1 [Gizmon: XT] from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT13-083 Play 1 [Gizmon: XT] from trash")
        effect3.set_effect_description("[On Deletion] By returning 2 cards with [Gizmon] in their names from your trash to the bottom of the deck in any order, you may play 1 [Gizmon: XT] from your trash without paying the cost.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Gizmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Gizmon')):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
