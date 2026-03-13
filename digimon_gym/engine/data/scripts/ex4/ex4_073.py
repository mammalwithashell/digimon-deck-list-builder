from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_073(CardScript):
    """EX4-073 Omnimon Alter-S | Lv.7 Black Digimon | 15000 DP | Cost 15

    Alt digivolve: Lv.7 [Omnimon] for cost 2.

    [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. Then,
        choose any number of your opponent's Digimon so that their play cost
        total is up to 6 and delete them.

    [When Attacking] By trashing up to 3 level 6 or higher cards in this
        Digimon's digivolution cards, among your opponent's Digimon and
        Tamers, delete 1 with the lowest play cost for each card trashed.
        If you trashed 3 cards, trash the top 2 cards of your opponent's
        security stack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

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
            "Then, choose any number of your opponent's Digimon so that their "
            "play cost total is up to 6 and delete them."
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
            """Delete opponent Digimon with total play cost up to 6."""
            eligible = [
                p for p in enemy.battle_area
                if p.is_digimon and _get_play_cost(p) <= 6
            ]
            if not eligible:
                return

            # Simplified: greedily delete cheapest opponent Digimon up to cost 6
            eligible.sort(key=lambda p: _get_play_cost(p))
            budget = 6
            to_delete = []
            for p in eligible:
                cost = _get_play_cost(p)
                if cost <= budget:
                    to_delete.append(p)
                    budget -= cost
            for p in to_delete:
                if p in enemy.battle_area:
                    enemy.delete_permanent(p)

        def _get_play_cost(perm) -> int:
            if perm.top_card:
                cost = getattr(perm.top_card, 'get_cost_itself', 0)
                if callable(cost):
                    return cost
                return int(cost) if cost else 0
            return 0

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
            "this Digimon's digivolution cards, among your opponent's Digimon "
            "and Tamers, delete 1 with the lowest play cost for each card "
            "trashed. If you trashed 3 cards, trash the top 2 cards of your "
            "opponent's security stack."
        )
        effect2.is_optional = True

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

        def _is_lv6_plus(c) -> bool:
            level = getattr(c, 'level', None)
            if level is None:
                return False
            has_level = getattr(c, 'has_level', True)
            if not has_level:
                return False
            return level >= 6

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

            # Trash up to 3 Lv.6+ digivolution cards
            evo_cards = perm.card_sources[:-1] if len(perm.card_sources) > 1 else []
            eligible_evo = [c for c in evo_cards if _is_lv6_plus(c)]
            trash_count = min(3, len(eligible_evo))
            trashed = []
            for c in eligible_evo[:trash_count]:
                if c in perm.card_sources:
                    perm.card_sources.remove(c)
                    player.trash_cards.append(c)
                    trashed.append(c)

            # For each trashed card, delete 1 opponent Digimon/Tamer with
            # lowest play cost
            for _ in trashed:
                opp_perms = list(enemy.battle_area)
                if not opp_perms:
                    break
                # Find the lowest play cost
                def _perm_cost(p):
                    if p.top_card:
                        cost = getattr(p.top_card, 'get_cost_itself', 0)
                        if callable(cost):
                            return cost
                        return int(cost) if cost else 99
                    return 99

                min_cost = min(_perm_cost(p) for p in opp_perms)
                targets = [p for p in opp_perms if _perm_cost(p) == min_cost]
                if targets:
                    enemy.delete_permanent(targets[0])

            # If trashed 3 cards, trash top 2 of opponent's security
            if len(trashed) >= 3:
                for _ in range(min(2, len(enemy.security_cards))):
                    top_sec = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(top_sec)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
