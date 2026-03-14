from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_033(CardScript):
    """EX11-033 Maneuvermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-033 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
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

        # Shared callback: select a [Maquinamon] from hand, then link it to one of your Digimon.
        def _play_maquinamon_linked(ctx: Dict[str, Any]):
            """Play 1 [Maquinamon] from hand to 1 of your Digimon without paying the cost."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import GamePhase
            from ....game.constants import (
                SEL_HAND_START, SEL_MY_FIELD_START, SEL_MY_BREEDING,
                ACTION_SPACE_SIZE,
            )

            def is_maquinamon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Maquinamon' in n for n in names)

            # Step 1: find valid Maquinamon in hand
            hand_valid = []
            for i, c in enumerate(player.hand_cards):
                if is_maquinamon(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    hand_valid.append(SEL_HAND_START + i)
            if not hand_valid:
                return

            def on_select_card(action_id: int):
                idx = action_id - SEL_HAND_START
                if idx < 0 or idx >= len(player.hand_cards):
                    return
                selected_card = player.hand_cards[idx]
                # Remove from hand
                player.hand_cards.pop(idx)

                # Step 2: choose a Digimon to link it to
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
                    # No valid target — play standalone as fallback
                    played_perm = player.play_card_from_source(selected_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(selected_card)} from hand (no link target)")
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
                        f"{game._card_ref(selected_card)} from hand linked to "
                        f"{game._perm_ref(target)}")

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select_target, link_valid,
                    is_optional=False,
                    prompt="Select a Digimon to link [Maquinamon] to.")

            game.request_selection(
                GamePhase.SelectTarget, player, on_select_card, hand_valid,
                is_optional=True,
                prompt="Select a [Maquinamon] from hand to play to 1 of your Digimon.")

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Play [Maquinamon] linked to a Digimon
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-033 Play [Maquinamon] to 1 of your Digimon")
        effect1.set_effect_description("[On Play] You may play 1 [Maquinamon] from your hand to 1 of your Digimon without paying the cost.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_play_maquinamon_linked)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Play [Maquinamon] linked to a Digimon
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-033 Play [Maquinamon] to 1 of your Digimon")
        effect2.set_effect_description("[When Digivolving] You may play 1 [Maquinamon] from your hand to 1 of your Digimon without paying the cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_maquinamon_linked)
        effects.append(effect2)

        # Timing: EffectTiming.WhenLinked
        # Suspend, Gain Keyword Cannot Unsuspend
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenLinked)
        effect3.set_effect_name("EX11-033 Suspend 1 opponent's Digimon. 1 of their Digimon can't unsuspend.")
        effect3.set_effect_description("Suspend, Gain Keyword Cannot Unsuspend")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_033_YT")
        effect3._is_cannot_unsuspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend 1 opponent's Digimon. Then, 1 of their Digimon can't unsuspend until their turn ends."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            # Then, 1 of opponent's Digimon can't unsuspend until their turn ends
            def cannot_unsuspend_filter(p):
                return p.is_digimon
            def on_cannot_unsuspend(target_perm):
                target_perm.grant_keyword('_is_cannot_unsuspend')
            game.effect_select_opponent_permanent(
                player, on_cannot_unsuspend, filter_fn=cannot_unsuspend_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndBattle
        # Unsuspend
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndBattle)
        effect4.set_effect_name("EX11-033 Unsuspend this Digimon")
        effect4.set_effect_description("Unsuspend")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX11_033_AT_ESS")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
