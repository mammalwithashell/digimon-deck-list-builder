from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_012(CardScript):
    """EX11-012 Medusamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: rush
        # Rush
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-012 Rush")
        effect0.set_effect_description("Rush")
        effect0._is_rush = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: progress
        # Progress
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-012 Progress")
        effect1.set_effect_description("Progress")
        effect1._is_progress = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenDigivolving
        # Delete 1 opponent Digimon with DP <= own DP, Return 1 to Deck, Play Token
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenDigivolving)
        effect2.set_effect_name("EX11-012 Delete, Return To Deck, Play Token")
        effect2.set_effect_description("Delete, Return To Deck, Play Token")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Return To Deck, Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            def target_filter(p):
                own_perm = card.permanent_of_this_card()
                own_dp = own_perm.dp if own_perm else 0
                return p.is_digimon and (p.dp or 0) <= (own_dp or 0)
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Return 1 opponent permanent to deck bottom (player selects)
            def target_filter_return(p):
                return True
            def on_return(target_perm):
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter_return, is_optional=False)
            # "Then, by returning 1 card from your opponent's trash to the bottom
            #  of the deck, they play 1 [Petrification] Token."
            # "By returning" = cost. If opponent's trash is empty, skip token.
            from ....game.constants import SEL_TRASH_START as _SEL_TRASH
            from ....data.enums import GamePhase

            valid_trash = [_SEL_TRASH + i for i in range(len(enemy.trash_cards))]
            if not valid_trash:
                return

            def on_trash_selected(action_id):
                idx = action_id - _SEL_TRASH
                if 0 <= idx < len(enemy.trash_cards):
                    chosen = enemy.trash_cards[idx]
                    enemy.trash_cards.remove(chosen)
                    enemy.library_cards.append(chosen)
                    # Cost paid — play 1 Petrification Token on opponent's field
                    game.effect_play_token(player, 'petrification', on_opponent_field=True, count=1)

            game.request_selection(
                GamePhase.SelectTarget, player, on_trash_selected,
                valid_trash, is_optional=False,
                prompt="Select 1 card from opponent's trash to return to deck bottom (cost for Petrification Token).")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # Delete 1 opponent Digimon with DP <= own DP, Return 1 to Deck, Play Token
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("EX11-012 Delete, Return To Deck, Play Token")
        effect3.set_effect_description("Delete, Return To Deck, Play Token")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete, Return To Deck, Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            def target_filter(p):
                own_perm = card.permanent_of_this_card()
                own_dp = own_perm.dp if own_perm else 0
                return p.is_digimon and (p.dp or 0) <= (own_dp or 0)
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Return 1 opponent permanent to deck bottom (player selects)
            def target_filter_return(p):
                return True
            def on_return(target_perm):
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter_return, is_optional=False)
            # "Then, by returning 1 card from your opponent's trash to the bottom
            #  of the deck, they play 1 [Petrification] Token."
            from ....game.constants import SEL_TRASH_START as _SEL_TRASH
            from ....data.enums import GamePhase

            valid_trash = [_SEL_TRASH + i for i in range(len(enemy.trash_cards))]
            if not valid_trash:
                return

            def on_trash_selected(action_id):
                idx = action_id - _SEL_TRASH
                if 0 <= idx < len(enemy.trash_cards):
                    chosen = enemy.trash_cards[idx]
                    enemy.trash_cards.remove(chosen)
                    enemy.library_cards.append(chosen)
                    game.effect_play_token(player, 'petrification', on_opponent_field=True, count=1)

            game.request_selection(
                GamePhase.SelectTarget, player, on_trash_selected,
                valid_trash, is_optional=False,
                prompt="Select 1 card from opponent's trash to return to deck bottom (cost for Petrification Token).")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # By deleting 1 of your tokens, this Digimon doesn't leave the battle area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("EX11-012 By deleting a token, this does not leave")
        effect4.set_effect_description("[When this Digimon would leave the battle area] By deleting 1 of your tokens, this Digimon doesn't leave.")
        effect4.is_optional = True
        effect4.set_hash_string("EX11_012_WHEN_REMOVED")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # Must be this Digimon leaving
            leaving_perm = context.get('event_permanent') or context.get('permanent')
            owner_perm = card.permanent_of_this_card()
            if leaving_perm is None or leaving_perm is not owner_perm:
                return False
            # Player must have at least one token on the field
            if not any(p.is_token for p in owner.battle_area):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete 1 own token, prevent this Digimon from leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            owner_perm = card.permanent_of_this_card()
            def token_filter(p):
                return p.is_token
            def on_token_delete(token_perm):
                player.delete_permanent(token_perm)
                # Register CANNOT_BE_REMOVED on self to prevent leaving
                if owner_perm and game:
                    from ....interfaces.modifiers import ModifierType
                    game.register_modifier(
                        owner_perm, ModifierType.CANNOT_BE_REMOVED,
                        value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_own_permanent(
                player, on_token_delete, filter_fn=token_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
