from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_021(CardScript):
    """BT20-021 Jesmon GX | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-021 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Blast Digivolve
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-021 Blast Digivolve")
        effect1.set_effect_description("Blast Digivolve")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ─── Shared process for effects 2/3/4 ─────────────────────────────────
        # [On Play] / [When Digivolving] / [When Attacking] [Once Per Turn]
        # By placing 1 [Royal Knight] trait card from your hand or trash as this
        # Digimon's bottom digivolution card, delete 1 of your opponent's Digimon
        # with as much or less DP as this Digimon.

        def _make_rk_place_delete_process():
            """Factory that captures `card` correctly for each timing variant."""

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                perm = card.permanent_of_this_card() if card else None
                if perm is None:
                    return

                def rk_filter(c):
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Royal Knight' in t for t in traits)

                has_hand = any(rk_filter(c) for c in player.hand_cards)
                has_trash = any(rk_filter(c) for c in player.trash_cards)

                if not (has_hand or has_trash):
                    return

                def do_delete(placed_card):
                    """After placing the card, delete opponent Digimon with DP <= this Digimon's DP."""
                    if placed_card is None:
                        return
                    current_perm = card.permanent_of_this_card() if card else None
                    if current_perm is None:
                        return

                    this_dp = current_perm.dp

                    def del_filter(p):
                        return p.is_digimon and p.dp is not None and (this_dp is None or p.dp <= this_dp)

                    def on_delete(target_perm):
                        enemy = player.enemy if player else None
                        if enemy:
                            enemy.delete_permanent(target_perm)

                    game.effect_select_opponent_permanent(
                        player, on_delete,
                        filter_fn=del_filter,
                        is_optional=False,
                        prompt="Select 1 opponent Digimon with DP <= this Digimon's DP to delete.")

                def place_from_hand(selected):
                    if selected is None:
                        return
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                    perm.card_sources.insert(0, selected)
                    do_delete(selected)

                def place_from_trash(selected):
                    if selected is None:
                        return
                    if selected in player.trash_cards:
                        player.trash_cards.remove(selected)
                    perm.card_sources.insert(0, selected)
                    do_delete(selected)

                if has_hand and has_trash:
                    # Let player choose hand or trash
                    def on_branch(branch: int):
                        if branch == 0:
                            game.effect_select_hand_card(
                                player, rk_filter, place_from_hand, is_optional=True,
                                prompt="Select 1 [Royal Knight] card from hand to place as bottom digivolution card.")
                        else:
                            # Select from trash
                            valid_trash = [
                                c for c in player.trash_cards if rk_filter(c)
                            ]
                            if valid_trash:
                                from ....game.constants import SEL_TRASH_START
                                valid_indices = [
                                    SEL_TRASH_START + i
                                    for i, c in enumerate(player.trash_cards)
                                    if rk_filter(c)
                                ]

                                def on_trash_select(action_id: int):
                                    from ....game.constants import SEL_TRASH_START as _TST
                                    idx = action_id - _TST
                                    if 0 <= idx < len(player.trash_cards):
                                        place_from_trash(player.trash_cards[idx])

                                from ....data.enums import GamePhase
                                game.request_selection(
                                    GamePhase.SelectTrash, player, on_trash_select,
                                    valid_indices=valid_indices, is_optional=True,
                                    prompt="Select 1 [Royal Knight] card from trash to place as bottom digivolution card.")

                    game.effect_choose_branch(
                        player, 2, on_branch,
                        prompt="Place Royal Knight from hand or trash?",
                        branch_labels=["From Hand", "From Trash"])

                elif has_hand:
                    game.effect_select_hand_card(
                        player, rk_filter, place_from_hand, is_optional=True,
                        prompt="Select 1 [Royal Knight] card from hand to place as bottom digivolution card.")
                else:
                    # Only trash available
                    from ....game.constants import SEL_TRASH_START
                    valid_indices = [
                        SEL_TRASH_START + i
                        for i, c in enumerate(player.trash_cards)
                        if rk_filter(c)
                    ]

                    def on_trash_select(action_id: int):
                        from ....game.constants import SEL_TRASH_START as _TST
                        idx = action_id - _TST
                        if 0 <= idx < len(player.trash_cards):
                            place_from_trash(player.trash_cards[idx])

                    from ....data.enums import GamePhase
                    game.request_selection(
                        GamePhase.SelectTrash, player, on_trash_select,
                        valid_indices=valid_indices, is_optional=True,
                        prompt="Select 1 [Royal Knight] card from trash to place as bottom digivolution card.")

            return process

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] [Once Per Turn]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect2.set_effect_description("[On Play] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT20_021")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_make_rk_place_delete_process())
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect3.set_effect_description("[When Digivolving] [Once Per Turn] Place 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Delete_BT20_021")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_make_rk_place_delete_process())
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] (place + delete)
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT20-021 Select 1 card, delete 1 card")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this digimon.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete_BT20_021")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_make_rk_place_delete_process())
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] This Digimon unsuspends. Then, for every
        # 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash
        # your opponent's top security card.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT20-021 Unsuspend, Then for every 2 [Royal Knight] traits in sources, trash opponent's top security")
        effect5.set_effect_description("[When Attacking] [Once per Turn] This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash your opponent's top security card")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Unsuspend_BT20_021")
        effect5.is_on_attack = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Unsuspend this Digimon. Then trash floor(RK_count/2) opponent security cards."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return

            # Unsuspend THIS Digimon
            if perm.is_suspended:
                perm.unsuspend()

            # Count Royal Knight trait cards in digivolution stack (all card_sources)
            rk_count = sum(
                1 for cs in perm.card_sources
                if any('Royal Knight' in t for t in (getattr(cs, 'card_traits', []) or []))
            )

            num_security_trashes = rk_count // 2

            enemy = player.enemy
            if enemy and num_security_trashes > 0:
                for _ in range(num_security_trashes):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
