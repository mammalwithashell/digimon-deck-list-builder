from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....core.permanent import Permanent
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_062(CardScript):
    """BT15-062 Gigadramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal top 4 cards. Add up to 2 level 6+ cards to hand. Rest to deck bottom.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-062 Reveal the top 4 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher Digimon cards among them to the hand. Place the rest at the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 4, select up to 2 level 6+ to hand, rest to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return getattr(c, 'level', None) is not None and c.level >= 6

            game.effect_reveal_and_select_multi(
                player, 4,
                [
                    (reveal_filter, 'hand'),
                    (reveal_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card
        # with [Dark Masters] trait from hand to breeding area without paying cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT15-062 Delete your 1 Digimon to play 1 Digimon from hand")
        effect1.set_effect_description("[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card with the [Dark Masters] trait from you hand to and empty space in your breeding area without paying the cost.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """By deleting 1 of your Digimon (cost), play 1 Dark Masters Digimon from hand free to breeding area."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def own_digimon_filter(p):
                return p.is_digimon

            has_own = any(own_digimon_filter(p) for p in player.battle_area)
            if not has_own:
                return

            def on_delete_cost(target_perm):
                player.delete_permanent(target_perm)

                # Only allow play if breeding area is empty (C# ref: GetBreedingAreaPermanents().Count == 0)
                if player.breeding_area is not None:
                    return

                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Dark Masters' in t or 'DarkMasters' in t for t in traits)

                def on_select_hand(selected_card):
                    # Remove from hand and place in breeding area (not battle area)
                    if selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                    new_perm = Permanent([selected_card])
                    if game:
                        new_perm.turn_played = game.turn_count
                        new_perm._owner_game = game
                    player.breeding_area = new_perm
                    game.logger.log(
                        f"[Effect] {player.player_name} played "
                        f"{selected_card.card_names[0] if selected_card.card_names else 'Unknown'} "
                        f"to breeding area without paying the cost")
                    # Fire On Play effects (C# ref: activateETB: true)
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": selected_card, "played_permanent": new_perm,
                         "event_player": player, "is_effect_play": True},
                    )

                game.effect_select_hand_card(
                    player, play_filter, on_select_hand, is_optional=True,
                    prompt="Select 1 Digimon card with [Dark Masters] trait to play to breeding area.")

            game.effect_select_own_permanent(
                player, on_delete_cost, filter_fn=own_digimon_filter, is_optional=True,
                prompt="Select 1 of your Digimon to delete.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-062 Reboot")
        effect2.set_effect_description("Reboot")
        effect2.is_inherited_effect = True
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
