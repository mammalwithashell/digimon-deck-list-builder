from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_021(CardScript):
    """BT20-021 Jesmon GX | Lv.7

    DNA digivolve: Lv.6 [Jesmon] + Lv.6 [Gankoomon], cost 0
    [Hand] [Counter] <Blast Digivolve>
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
        By placing 1 [Royal Knight] trait card from your hand or trash
        as this Digimon's bottom digivolution card, delete 1 of your
        opponent's Digimon with as much or less DP as this Digimon.
    [When Attacking] [Once Per Turn] This Digimon unsuspends. Then, for
        every 2 [Royal Knight] trait cards in this Digimon's digivolution
        cards, trash your opponent's top security card.
    Inherited: Ace Overflow <-5>
    """

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

        # ─── [On Play] [When Digivolving] [When Attacking] [Once Per Turn] ───
        # By placing 1 [Royal Knight] trait card from your hand or trash as
        # this Digimon's bottom digivolution card, delete 1 of your opponent's
        # Digimon with as much or less DP as this Digimon.
        #
        # SINGLE effect with all three flags so OPT is truly shared.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-021 Place RK + delete")
        effect2.set_effect_description(
            "[On Play] [When Digivolving] [When Attacking] [Once Per Turn] "
            "By placing 1 [Royal Knight] trait card from your hand or trash "
            "as this Digimon's bottom digivolution card, delete 1 of your "
            "opponent's Digimon with as much or less DP as this Digimon."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT20_021")
        effect2.is_on_play = True
        effect2.is_when_digivolving = True
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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
                def on_branch(branch: int):
                    if branch == 0:
                        game.effect_select_hand_card(
                            player, rk_filter, place_from_hand, is_optional=True,
                            prompt="Select 1 [Royal Knight] card from hand to place as bottom digivolution card.")
                    else:
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── [When Attacking] [Once Per Turn] Unsuspend + security trash ──────
        # This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards
        # in this Digimon's digivolution cards, trash your opponent's top
        # security card.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT20-021 Unsuspend, security trash")
        effect3.set_effect_description(
            "[When Attacking] [Once per Turn] This Digimon unsuspends. Then, "
            "for every 2 [Royal Knight] trait cards in this Digimon's "
            "digivolution cards, trash your opponent's top security card."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_BT20_021")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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

            # Count Royal Knight trait cards in digivolution cards EXCLUDING top card.
            # C# DigivolutionCards = all sources except the top card.
            digi_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            rk_count = sum(
                1 for cs in digi_cards
                if any('Royal Knight' in t for t in (getattr(cs, 'card_traits', []) or []))
            )

            num_security_trashes = rk_count // 2

            enemy = player.enemy
            if enemy and num_security_trashes > 0:
                for _ in range(num_security_trashes):
                    if enemy.security_cards:
                        # Trash from top (index 0) using proper API
                        enemy.trash_security_card(enemy.security_cards[0])

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ─── Inherited: Ace Overflow <-5> ─────────────────────────────────
        # Engine handles ACE Overflow automatically via entity_base.is_ace /
        # ace_overflow_cost parsed from inherited_effect_description_eng.
        # This effect tag documents the inherited effect in the script.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT20-021 Ace Overflow")
        effect_ace.set_effect_description("Ace Overflow <-5>")
        effect_ace.is_inherited_effect = True
        effect_ace._is_ace_overflow = True
        effect_ace._overflow_amount = -5

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
