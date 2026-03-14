from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_073(CardScript):
    """EX4-073 Omnimon Alter-B | Lv.7 Black Digimon | 15000 DP | Cost 15

    Alt digivolve: Lv.7 [Omnimon] for cost 2.

    [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. Then,
        delete up to 6 play cost's total worth of their Digimon.

    [When Attacking] By trashing up to 3 level 6 or higher cards in this
        Digimon's digivolution cards, for each card trashed, activate the
        effect below. Then, if you trashed 3 cards, trash the top 2 cards
        of your opponent's security stack.
        - Delete 1 of your opponent's Digimon or Tamers with the lowest
          play cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _get_play_cost(perm) -> int:
            if perm.top_card:
                cost = getattr(perm.top_card, 'get_cost_itself', 0)
                return int(cost) if cost else 0
            return 0

        # --- Effect 0: Alt digivolve from Lv.7 Omnimon for cost 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX4-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution: Lv.7 [Omnimon] for cost 2")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 7
        effect0._alt_digi_name = "Omnimon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] De-Digivolve 3 one opponent Digimon,
        #     then delete opponent Digimon with total play cost up to 6 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX4-073 De-Digivolve 3 + delete up to cost 6")
        effect1.set_effect_description(
            "[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. "
            "Then, delete up to 6 play cost's total worth of their Digimon."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Part 1: De-Digivolve 3 one opponent Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                def de_digi_filter(p):
                    return p.is_digimon and p in enemy.battle_area

                def on_de_digi(target_perm):
                    if target_perm and len(target_perm.card_sources) > 1:
                        removed = target_perm.de_digivolve(3)
                        for rc in removed:
                            enemy.trash_cards.append(rc)

                    # Part 2: Delete opponent Digimon with total play cost <= 6
                    _delete_cost_limited(player, enemy, game)

                game.effect_select_opponent_permanent(
                    player, on_de_digi, filter_fn=de_digi_filter,
                    is_optional=False,
                    prompt="Select 1 opponent Digimon to De-Digivolve 3."
                )
            else:
                # No opponent Digimon for de-digivolve, skip to delete
                _delete_cost_limited(player, enemy, game)

        def _delete_cost_limited(player, enemy, game):
            """Delete opponent Digimon with total play cost up to 6.

            Lets the agent select targets one by one. After each selection,
            the remaining budget is checked to constrain further selections.
            """
            budget_remaining = [6]  # mutable container for closure

            def _do_select_loop():
                # Find eligible targets whose cost fits in remaining budget
                eligible = [
                    p for p in enemy.battle_area
                    if p.is_digimon and _get_play_cost(p) <= budget_remaining[0]
                ]
                if not eligible:
                    return

                def cost_filter(p):
                    return (p.is_digimon and p in enemy.battle_area
                            and _get_play_cost(p) <= budget_remaining[0])

                def on_delete(target_perm):
                    cost = _get_play_cost(target_perm)
                    budget_remaining[0] -= cost
                    enemy.delete_permanent(target_perm)
                    # Continue selecting if budget and targets remain
                    _do_select_loop()

                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=cost_filter,
                    is_optional=True,
                    prompt=f"Select opponent Digimon to delete (budget: {budget_remaining[0]} remaining)."
                )

            _do_select_loop()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Attacking] Trash up to 3 Lv.6+ evo cards,
        #     delete 1 lowest-cost opponent per card trashed,
        #     if 3 trashed then trash top 2 security ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX4-073 Trash evo cards to delete lowest cost + security")
        effect2.set_effect_description(
            "[When Attacking] By trashing up to 3 level 6 or higher cards in "
            "this Digimon's digivolution cards, for each card trashed, "
            "delete 1 of your opponent's Digimon or Tamers with the lowest "
            "play cost. If you trashed 3 cards, trash the top 2 cards of "
            "your opponent's security stack."
        )
        effect2.is_optional = True

        def _is_lv6_plus(c) -> bool:
            level = getattr(c, 'level', None)
            if level is None:
                return False
            has_level = getattr(c, 'has_level', True)
            if not has_level:
                return False
            return level >= 6

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            if not perm.is_digimon:
                return False
            # Must have at least 1 Lv.6+ evo card
            evo_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            has_eligible = any(
                _is_lv6_plus(c) for c in evo_cards
            )
            return has_eligible
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD

            # Find field index of this permanent
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return
            base = 2000 + field_idx * SOURCES_PER_FIELD

            # Let agent select which Lv.6+ evo cards to trash (up to 3)
            evo_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            eligible_evo = [c for c in evo_cards if _is_lv6_plus(c)]
            if not eligible_evo:
                return

            trashed = []
            max_trash = min(3, len(eligible_evo))

            # Select evo cards to trash one at a time
            def _select_next_evo():
                if len(trashed) >= max_trash:
                    _do_deletions()
                    return

                current_eligible = [
                    c for c in perm.card_sources[:-1]
                    if _is_lv6_plus(c) and c not in trashed
                ]
                if not current_eligible:
                    _do_deletions()
                    return

                # Build valid action IDs for source selection
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs in current_eligible and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    _do_deletions()
                    return

                def on_select_evo(action_id: int):
                    idx = action_id - base
                    if 0 <= idx < len(perm.card_sources):
                        chosen = perm.card_sources[idx]
                        if chosen in perm.card_sources:
                            perm.card_sources.remove(chosen)
                            player.trash_cards.append(chosen)
                            trashed.append(chosen)
                    _select_next_evo()

                game.request_selection(
                    GamePhase.SelectSource, player, on_select_evo, valid,
                    is_optional=True,
                    prompt=f"Select a Lv.6+ digivolution card to trash ({len(trashed)}/{max_trash})."
                )

            def _do_deletions():
                if not trashed:
                    return

                # For each trashed card, delete 1 opponent Digimon/Tamer
                # with the lowest play cost
                def _delete_lowest_loop(remaining_count):
                    if remaining_count <= 0:
                        # Check security trash
                        if len(trashed) >= 3:
                            for _ in range(min(2, len(enemy.security_cards))):
                                top_sec = enemy.security_cards.pop(0)
                                enemy.trash_cards.append(top_sec)
                        return

                    opp_perms = list(enemy.battle_area)
                    if not opp_perms:
                        if len(trashed) >= 3:
                            for _ in range(min(2, len(enemy.security_cards))):
                                top_sec = enemy.security_cards.pop(0)
                                enemy.trash_cards.append(top_sec)
                        return

                    # Find lowest play cost among opponent permanents
                    def _perm_cost(p):
                        if p.top_card:
                            cost = getattr(p.top_card, 'get_cost_itself', 0)
                            return int(cost) if cost else 99
                        return 99

                    min_cost = min(_perm_cost(p) for p in opp_perms)

                    def lowest_filter(p):
                        return (p.is_digimon or p.is_tamer) and _perm_cost(p) == min_cost

                    def on_delete(target_perm):
                        enemy.delete_permanent(target_perm)
                        _delete_lowest_loop(remaining_count - 1)

                    has_target = any(lowest_filter(p) for p in opp_perms)
                    if has_target:
                        game.effect_select_opponent_permanent(
                            player, on_delete,
                            filter_fn=lowest_filter,
                            is_optional=False)
                    else:
                        _delete_lowest_loop(remaining_count - 1)

                _delete_lowest_loop(len(trashed))

            _select_next_evo()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
