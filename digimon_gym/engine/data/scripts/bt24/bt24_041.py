from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_041(CardScript):
    """BT24-041 Minervamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 with [Beastkin] or [Dark Dragon] or [TS] for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-041 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Beastkin"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card):
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return (any('Beastkin' in tr for tr in traits)
                    or any('Dark Dragon' in tr for tr in traits)
                    or any('TS' in tr for tr in traits))
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have an [Iliad] trait Digimon or Tamer,
        # reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-041 Reduce play cost (5)")
        effect1.set_effect_description(
            "When this card would be played, if you have an [Iliad] trait Digimon or Tamer, "
            "reduce the play cost by 5.")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            for p in owner.battle_area:
                if not (p.is_digimon or p.is_tamer):
                    continue
                traits = []
                if p.top_card:
                    traits = getattr(p.top_card, 'card_traits', []) or []
                if any('Iliad' in t for t in traits):
                    return True
            return False

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Helper: shared play filter for On Play / When Digivolving / On Deletion
        def _iliad_play_filter(c) -> bool:
            if not (getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)):
                return False
            cost = c.get_cost_itself if hasattr(c, 'get_cost_itself') else getattr(c, 'play_cost', 99)
            if cost is None or cost > 5:
                return False
            traits = getattr(c, 'card_traits', []) or []
            if not any('Iliad' in t for t in traits):
                return False
            return True

        # Helper: De-Digivolve step (called after play resolves or is skipped)
        def _do_de_digivolve(player, game):
            digi_count = len([p for p in player.battle_area if p.is_digimon])
            if digi_count > 0:
                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(digi_count)
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.trash_cards.extend(removed)
                game.effect_select_opponent_permanent(
                    player, on_de_digivolve, filter_fn=lambda p: p.is_digimon,
                    is_optional=False)

        # Helper: shared process — play from hand (optional), then De-Digivolve 1 opponent's
        # Digimon by the count of your own Digimon in the battle area.
        # Uses manual selection to chain the two steps properly.
        def _shared_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE
            from ....data.enums import GamePhase
            valid = []
            for i, c in enumerate(player.hand_cards):
                if _iliad_play_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_HAND_START + i)

            if not valid:
                _do_de_digivolve(player, game)
                return

            def on_select(action_id):
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    sel_card = player.hand_cards[idx]
                    played_perm = player.play_card_from_source(sel_card, pay_cost=False)
                    game.logger.log(
                        f"[Effect] {player.player_name} played "
                        f"{game._card_ref(sel_card)} from hand")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": sel_card, "played_permanent": played_perm,
                         "event_player": player},
                    )
                _do_de_digivolve(player, game)

            def on_decline():
                _do_de_digivolve(player, game)

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="You may play 1 play cost 5 or lower [Iliad] trait card from your "
                       "hand without paying the cost.",
                on_decline=on_decline,
            )

        # Timing: EffectTiming.OnEnterFieldAnyone — [On Play]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-041 Play Iliad card, De-Digivolve (On Play)")
        effect3.set_effect_description(
            "[On Play] You may play 1 play cost 5 or lower [Iliad] trait Digimon card or "
            "Tamer card from your hand without paying the cost. Then, to 1 of your opponent's "
            "Digimon, <De-Digivolve 1> for each of your Digimon.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone — [When Digivolving]
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-041 Play Iliad card, De-Digivolve (When Digivolving)")
        effect4.set_effect_description(
            "[When Digivolving] You may play 1 play cost 5 or lower [Iliad] trait Digimon card or "
            "Tamer card from your hand without paying the cost. Then, to 1 of your opponent's "
            "Digimon, <De-Digivolve 1> for each of your Digimon.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_process)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone — [On Deletion]
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("BT24-041 Play Iliad card, De-Digivolve (On Deletion)")
        effect5.set_effect_description(
            "[On Deletion] You may play 1 play cost 5 or lower [Iliad] trait Digimon card or "
            "Tamer card from your hand without paying the cost. Then, to 1 of your opponent's "
            "Digimon, <De-Digivolve 1> for each of your Digimon.")
        effect5.is_on_deletion = True

        def condition5(context: Dict[str, Any]) -> bool:
            ctx_perm = context.get('permanent')
            owner_perm = card.permanent_of_this_card() if card else None
            # On deletion: the card is no longer on field; match via deletion context
            if owner_perm is not None and ctx_perm is not None and ctx_perm is not owner_perm:
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_shared_process)
        effects.append(effect5)

        # [Opponent's Turn] All of your [Iliad] trait Digimon gain <Reboot> and <Blocker>.
        # Use _applies_to_all_own_digimon with condition checking Iliad trait
        def _iliad_opp_turn_condition(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            if owner.is_my_turn:
                return False
            ctx_perm = context.get('permanent')
            if ctx_perm and ctx_perm.top_card:
                traits = getattr(ctx_perm.top_card, 'card_traits', []) or []
                return any('Iliad' in t for t in traits)
            return False

        for kw_name, kw_attr in [('Reboot', '_is_reboot'), ('Blocker', '_is_blocker')]:
            eff = ICardEffect()
            eff.set_effect_name(f"BT24-041 {kw_name} (grant to Iliad Digimon)")
            eff.set_effect_description(f"{kw_name} (grant to all Iliad Digimon)")
            setattr(eff, kw_attr, True)
            eff._applies_to_all_own_digimon = True
            eff.set_can_use_condition(_iliad_opp_turn_condition)
            effects.append(eff)

        return effects
