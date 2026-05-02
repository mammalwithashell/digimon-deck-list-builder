from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_077(CardScript):
    """BT15-077 LadyDevimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-077 Reveal the top 4 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 4, add up to 2 Lv6+ to hand, rest to deck bottom"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def lv6_filter(c):
                level = getattr(c, 'level', None)
                return level is not None and level >= 6

            # Use multi-pass reveal: 2 passes selecting Lv6+ to hand, rest to bottom
            passes = [
                (lv6_filter, 'hand'),
                (lv6_filter, 'hand'),
            ]
            game.effect_reveal_and_select_multi(
                player, 4, passes,
                remaining_placement='deck_bottom',
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card with the [Dark Masters] trait from your hand to an empty space in your breeding area without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT15-077 Delete your 1 Digimon to play 1 Digimon from hand")
        effect1.set_effect_description("[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card with the [Dark Masters] trait from your hand to an empty space in your breeding area without paying the cost.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete 1 of your Digimon, then play 1 Dark Masters from hand to breeding free"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Delete 1 of your Digimon (as cost)
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if not own_digimon:
                return

            def digimon_filter(p):
                return p.is_digimon

            def on_delete_own(target_perm):
                if target_perm in player.battle_area:
                    player.delete_permanent(target_perm)
                # Step 2: Play 1 Dark Masters Digimon from hand to breeding area
                # (only if breeding area is empty)
                if player.breeding_area is not None:
                    return  # Breeding area not empty
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Dark Masters' in t for t in traits)

                from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE
                from ....core.permanent import Permanent
                from ....data.enums import GamePhase

                valid = []
                for i, c in enumerate(player.hand_cards):
                    if play_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                        valid.append(SEL_HAND_START + i)
                if not valid:
                    return

                def on_select_breeding(action_id):
                    idx = action_id - SEL_HAND_START
                    if 0 <= idx < len(player.hand_cards):
                        chosen = player.hand_cards[idx]
                        player.hand_cards.remove(chosen)
                        new_perm = Permanent([chosen])
                        if game:
                            new_perm.turn_played = game.turn_count
                            new_perm._owner_game = game
                        player.breeding_area = new_perm
                        game.logger.log(
                            f"[Effect] {player.player_name} played "
                            f"{game._card_ref(chosen)} to breeding area"
                        )
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": chosen, "played_permanent": new_perm,
                             "event_player": player, "is_effect_play": True},
                        )

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select_breeding,
                    valid, True,
                    prompt="Select 1 Digimon with [Dark Masters] trait from hand to play to breeding area."
                )

            game.effect_select_own_permanent(
                player, on_delete_own, filter_fn=digimon_filter, is_optional=True,
                prompt="Select 1 of your Digimon to delete.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: retaliation
        # Retaliation
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-077 Retaliation")
        effect2.set_effect_description("Retaliation")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
