from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_042(CardScript):
    """EX11-042 MockingBirdmon | Lv.5

    Alt digivolution: Lv.4 with [Maquinamon] in text for cost 3.

    [On Play] [When Digivolving] You may play 1 [Maquinamon] from your hand
    or this Digimon's link cards without paying the cost.
    [Your Turn] [Once Per Turn] When this Digimon gets linked, delete 1 of
    your opponent's Digimon with a play cost of 5 or less.

    --- Inherited ---
    [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon
    attacks, you may change the attack target to this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_maquinamon(c):
            names = getattr(c, 'card_names', []) or []
            return any('Maquinamon' in n for n in names)

        # --- Alt digivolution: Lv.4 with [Maquinamon] in text for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Shared callback: play 1 [Maquinamon] from hand or link cards
        def _play_maquinamon(ctx: Dict[str, Any]):
            """Play 1 [Maquinamon] from hand or this Digimon's link cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import GamePhase
            from ....game.constants import (
                SEL_HAND_START, SEL_MY_FIELD_START, SEL_MY_BREEDING,
                ACTION_SPACE_SIZE, SEL_EFFECT_CHOICE_START,
            )

            hand_maq = [c for c in player.hand_cards if _is_maquinamon(c)]
            link_maq = []
            if perm and perm.linked_cards:
                link_maq = [c for c in perm.linked_cards if _is_maquinamon(c)]

            has_hand = len(hand_maq) > 0
            has_link = len(link_maq) > 0

            if not has_hand and not has_link:
                return

            def _do_play_from_hand():
                hand_valid = []
                for i, c in enumerate(player.hand_cards):
                    if _is_maquinamon(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                        hand_valid.append(SEL_HAND_START + i)
                if not hand_valid:
                    return

                def on_select_card(action_id: int):
                    idx = action_id - SEL_HAND_START
                    if idx < 0 or idx >= len(player.hand_cards):
                        return
                    selected_card = player.hand_cards[idx]
                    player.hand_cards.pop(idx)
                    _play_to_field(selected_card)

                game.request_selection(
                    GamePhase.SelectHand, player, on_select_card, hand_valid,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from hand to play.")

            def _do_play_from_links():
                if not perm or not perm.linked_cards:
                    return
                link_valid = []
                for i, c in enumerate(perm.linked_cards):
                    if _is_maquinamon(c):
                        link_valid.append(SEL_EFFECT_CHOICE_START + i)
                if not link_valid:
                    return

                def on_select_link(action_id: int):
                    idx = action_id - SEL_EFFECT_CHOICE_START
                    if idx < 0 or idx >= len(perm.linked_cards):
                        return
                    selected_card = perm.linked_cards[idx]
                    perm.linked_cards.remove(selected_card)
                    _play_to_field(selected_card)

                game.request_selection(
                    GamePhase.SelectEffectChoice, player, on_select_link, link_valid,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from link cards to play.")

            def _play_to_field(selected_card):
                """Play the selected Maquinamon to a Digimon."""
                link_valid = []
                for fi, fp in enumerate(player.battle_area):
                    if fp.is_token:
                        continue
                    if not fp.is_digimon:
                        continue
                    link_valid.append(SEL_MY_FIELD_START + fi)
                ba = player.breeding_area
                if ba is not None and ba.is_digimon and (ba.level or 0) > 2:
                    link_valid.append(SEL_MY_BREEDING)

                if not link_valid:
                    played_perm = player.play_card_from_source(selected_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(selected_card)} (no link target)")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": selected_card, "played_permanent": played_perm, "event_player": player},
                    )
                    return

                def on_select_target(target_action_id: int):
                    if target_action_id == SEL_MY_BREEDING:
                        target = player.breeding_area
                    else:
                        tidx = target_action_id - SEL_MY_FIELD_START
                        if 0 <= tidx < len(player.battle_area):
                            target = player.battle_area[tidx]
                        else:
                            return
                    if target is None:
                        return
                    target.link_card(selected_card)
                    game.logger.log(
                        f"[Effect] {player.player_name} played "
                        f"{game._card_ref(selected_card)} linked to "
                        f"{game._perm_ref(target)}")

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select_target, link_valid,
                    is_optional=False,
                    prompt="Select a Digimon to link [Maquinamon] to.")

            # If both hand and link sources available, let agent choose
            if has_hand and has_link:
                def on_zone_choice(branch: int):
                    if branch == 0:
                        _do_play_from_hand()
                    elif branch == 1:
                        _do_play_from_links()

                game.effect_choose_branch(
                    player, 2, on_zone_choice,
                    prompt="Play [Maquinamon] from hand or link cards?",
                    branch_labels=["From hand", "From link cards"])
            elif has_hand:
                _do_play_from_hand()
            elif has_link:
                _do_play_from_links()

        # --- [On Play]: play 1 [Maquinamon] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-042 Play [Maquinamon]")
        effect1.set_effect_description(
            "[On Play] You may play 1 [Maquinamon] from your hand or this "
            "Digimon's link cards without paying the cost.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_play_maquinamon)
        effects.append(effect1)

        # --- [When Digivolving]: play 1 [Maquinamon] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-042 Play [Maquinamon]")
        effect2.set_effect_description(
            "[When Digivolving] You may play 1 [Maquinamon] from your hand "
            "or this Digimon's link cards without paying the cost.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_maquinamon)
        effects.append(effect2)

        # --- WhenLinked: delete 1 opponent's Digimon with play cost 5 or less ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenLinked)
        effect3.set_effect_name("EX11-042 Delete an opponent's 5 cost or less Digimon")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon gets linked, "
            "delete 1 of your opponent's Digimon with a play cost of 5 or less.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_042_YT")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Delete 1 of opponent's Digimon with play cost <= 5."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                cost = p.top_card.get_cost_itself if p.top_card else 0
                return cost <= 5

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent's Digimon with play cost 5 or less to delete.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Inherited: [Opponent's Turn] [Once Per Turn] When opponent's
        # Digimon attacks, may change the attack target to this Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX11-042 Redirect attack to this Digimon")
        effect4.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, you may change the attack target to this Digimon.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Redirect_EX11_042")
        effect4._is_when_attacked_observer = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            owner = card.owner if card else None
            if owner and owner.is_my_turn:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Redirect attack to this Digimon."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (perm and game):
                return
            game.redirect_attack(perm)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
