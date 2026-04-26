from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Trash selection action-ID offset (mirrors game.py SEL_TRASH_START)
_SEL_TRASH_START = 130


class BT17_070(CardScript):
    """BT17-070 Gulfmon | Lv.6 Purple | Dark Animal, Virus | 11000 DP | Cost 12

    [On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters]
        in its text from your hand or trash as this Digimon's bottom digivolution
        card, delete 1 of your opponent's level 5 or lower Digimon.
    [When Attacking] By returning 7 cards from your opponent's trash to the
        bottom of the deck, unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared filter for placing Dark Masters Lv.5 card ---
        def _is_dark_masters_lv5(c) -> bool:
            level = getattr(c, 'level', None)
            if level != 5:
                return False
            text = getattr(c, 'card_text', '') or ''
            return 'Dark Masters' in text

        def _has_dark_masters_in_hand_or_trash() -> bool:
            player = card.owner if card else None
            if not player:
                return False
            for c in player.hand_cards:
                if _is_dark_masters_lv5(c):
                    return True
            for c in player.trash_cards:
                if _is_dark_masters_lv5(c):
                    return True
            return False

        # --- Shared process for On Play / When Digivolving ---
        def _place_digi_and_delete(ctx: Dict[str, Any]):
            """Place 1 Lv.5 Dark Masters card as bottom digi, then delete opp Lv.5- Digimon.

            C# reference flow:
            1. If both hand and trash have qualifying cards, present zone choice
               (SetBoolSelection). If only one zone, auto-select that zone.
            2. Player selects which specific card from chosen zone.
            3. If a card was placed, delete 1 of opponent's Lv.5-or-lower Digimon.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            can_hand = any(_is_dark_masters_lv5(c) for c in player.hand_cards)
            can_trash = any(_is_dark_masters_lv5(c) for c in player.trash_cards)
            if not can_hand and not can_trash:
                return

            def _do_delete_after_place():
                """After placing a card, delete 1 of opponent's Lv.5-or-lower Digimon."""
                enemy = player.enemy
                if not enemy:
                    return

                def delete_filter(p):
                    return (p.is_digimon and p.level is not None and p.level <= 5)

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=delete_filter, is_optional=False)

            def _place_from_hand(selected_card):
                """Callback after player selects a card from hand."""
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                perm.add_card_source_bottom(selected_card)
                _do_delete_after_place()

            def _select_from_hand():
                """Present hand selection for qualifying cards."""
                game.effect_select_hand_card(
                    player, _is_dark_masters_lv5, _place_from_hand,
                    is_optional=False,
                    prompt="Select 1 card to place on bottom of digivolution cards.")

            def _place_from_trash_callback(action_id):
                """Callback after player selects a trash card by action ID."""
                idx = action_id - _SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    selected = player.trash_cards.pop(idx)
                    perm.add_card_source_bottom(selected)
                    _do_delete_after_place()

            def _select_from_trash():
                """Present trash selection for qualifying cards."""
                valid_trash = []
                for i, c in enumerate(player.trash_cards):
                    if _is_dark_masters_lv5(c):
                        valid_trash.append(_SEL_TRASH_START + i)
                if valid_trash:
                    game.request_selection(
                        GamePhase.SelectTrash, player,
                        _place_from_trash_callback, valid_trash,
                        is_optional=False,
                        prompt="Select 1 card to place on bottom of digivolution cards.")

            if can_hand and can_trash:
                # Both zones have qualifying cards -- present zone choice
                def on_zone_choice(branch: int):
                    if branch == 0:
                        _select_from_hand()
                    else:
                        _select_from_trash()

                game.effect_choose_branch(
                    player, 2, on_zone_choice,
                    prompt="From which area do you choose a digivolution card?",
                    branch_labels=["From hand", "From trash"])
            elif can_hand:
                # Only hand -- skip zone choice, go directly to hand selection
                _select_from_hand()
            else:
                # Only trash -- skip zone choice, go directly to trash selection
                _select_from_trash()

        # --- Effect 0: [On Play] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-070 On Play: Place Dark Masters, delete opp Lv.5-")
        effect0.set_effect_description(
            "[On Play] By placing 1 level 5 card with [Dark Masters] in its "
            "text from your hand or trash as this Digimon's bottom digivolution "
            "card, delete 1 of your opponent's level 5 or lower Digimon."
        )
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_dark_masters_in_hand_or_trash()
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_place_digi_and_delete)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-070 When Digivolving: Place Dark Masters, delete opp Lv.5-")
        effect1.set_effect_description(
            "[When Digivolving] By placing 1 level 5 card with [Dark Masters] "
            "in its text from your hand or trash as this Digimon's bottom "
            "digivolution card, delete 1 of your opponent's level 5 or lower "
            "Digimon."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_dark_masters_in_hand_or_trash()
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_place_digi_and_delete)
        effects.append(effect1)

        # --- Effect 2: [When Attacking] Return 7 cards from opponent's trash to deck bottom, unsuspend ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT17-070 When Attacking: Return 7 opp trash, unsuspend")
        effect2.set_effect_description(
            "[When Attacking] By returning 7 cards from your opponent's trash "
            "to the bottom of the deck, unsuspend this Digimon."
        )
        effect2.is_on_attack = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            return len(enemy.trash_cards) >= 7
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Return 7 cards from opponent's trash to deck bottom, unsuspend self."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy or len(enemy.trash_cards) < 7:
                return
            # Return 7 cards to deck bottom
            for _ in range(7):
                if enemy.trash_cards:
                    c = enemy.trash_cards.pop(0)
                    enemy.library_cards.append(c)
            # Unsuspend this Digimon
            perm.unsuspend()
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
