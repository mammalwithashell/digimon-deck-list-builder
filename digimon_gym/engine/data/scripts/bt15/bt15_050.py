from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_050(CardScript):
    """BT15-050 Cherrymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal top 4 cards. Add up to 2 level 6+ cards to hand. Rest to deck bottom.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-050 Reveal the top 4 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 2 level 6 or higher cards among them to the hand. Return the rest to the bottom of the deck.")
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
        effect1.set_effect_name("BT15-050 Delete your 1 Digimon to play 1 Digimon from hand")
        effect1.set_effect_description("[End of Your Turn] By deleting 1 of your Digimon, you may play 1 Digimon card with the [Dark Masters] trait from your hand to an empty space in your breeding area without paying the cost.")
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
            from ....core.permanent import Permanent as _Permanent

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Card text: "to an empty space in your breeding area"
            if player.breeding_area is not None:
                return  # Breeding area not empty — cannot use

            def own_digimon_filter(p):
                return p.is_digimon

            has_own = any(own_digimon_filter(p) for p in player.battle_area)
            if not has_own:
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Masters' in t or 'DarkMasters' in t for t in traits)

            # Check there's at least one valid Dark Masters Digimon in hand
            has_dm = any(play_filter(c) for c in player.hand_cards)
            if not has_dm:
                return

            def on_delete_cost(target_perm):
                player.delete_permanent(target_perm)

                # Play 1 Dark Masters Digimon from hand to breeding area (free)
                def on_hand_select(selected_card):
                    if selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                    new_perm = _Permanent([selected_card])
                    if game:
                        new_perm.turn_played = game.turn_count
                        new_perm._owner_game = game
                    player.breeding_area = new_perm
                    game.logger.log(
                        f"[Effect] {player.player_name} played "
                        f"{selected_card.card_names[0]} to breeding area")

                game.effect_select_hand_card(
                    player, play_filter, on_hand_select, is_optional=True,
                    prompt="Select a [Dark Masters] Digimon from hand to play to breeding area.")

            game.effect_select_own_permanent(
                player, on_delete_cost, filter_fn=own_digimon_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: Piercing (per C# reference)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-050 Piercing")
        effect2.set_effect_description("Piercing")
        effect2.is_inherited_effect = True
        effect2._is_piercing = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
