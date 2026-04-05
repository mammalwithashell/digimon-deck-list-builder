from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_073(CardScript):
    """EX4-073 Omnimon Alter-B | Lv.7 Purple Digimon | 15000 DP | Cost 14

    Alt digivolve: Lv.6 w/[Omnimon] in name: Cost 4
    DNA: Purple Lv.6 + Blue/Red Lv.6, cost 0

    [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. Then,
        delete 1 of your opponent's Digimon with as much or less DP as this
        Digimon.

    [All Turns] All of your Digimon with [Omnimon] in their names gain
        <Blocker>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.6 w/[Omnimon] in name for cost 4 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX4-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution: Lv.6 w/[Omnimon] in name: Cost 4")
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 6
        effect0._alt_digi_name = "Omnimon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] De-Digivolve 3 one opponent Digimon,
        #     then delete 1 opponent Digimon with DP <= this Digimon's DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX4-073 De-Digivolve 3 + delete by DP")
        effect1.set_effect_description(
            "[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. "
            "Then, delete 1 of your opponent's Digimon with as much or less DP "
            "as this Digimon."
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
            my_perm = card.permanent_of_this_card() if card else None

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

                    # Part 2: Delete 1 opponent Digimon with DP <= this Digimon's DP
                    _delete_by_dp(player, enemy, game, my_perm)

                game.effect_select_opponent_permanent(
                    player, on_de_digi, filter_fn=de_digi_filter,
                    is_optional=False,
                    prompt="Select 1 opponent Digimon to De-Digivolve 3."
                )
            else:
                # No opponent Digimon for de-digivolve, skip to delete
                _delete_by_dp(player, enemy, game, my_perm)

        def _delete_by_dp(player, enemy, game, my_perm):
            """Delete 1 opponent Digimon with DP <= this Digimon's DP."""
            if not my_perm:
                return
            my_dp = my_perm.dp or 0

            def dp_filter(p):
                if not p.is_digimon:
                    return False
                if p not in enemy.battle_area:
                    return False
                p_dp = p.dp if p.dp is not None else 0
                return p_dp <= my_dp

            eligible = [p for p in enemy.battle_area if dp_filter(p)]
            if not eligible:
                return

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete,
                filter_fn=dp_filter,
                is_optional=False,
                prompt=f"Select 1 opponent Digimon with {my_dp} or less DP to delete."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] All of your Digimon with [Omnimon] in
        #     their names gain <Blocker>. ---
        # Uses _applies_to_all_own_digimon pattern with condition checking
        # for [Omnimon] in the permanent's name.
        def _omnimon_blocker_condition(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            ctx_perm = context.get('permanent')
            if ctx_perm and ctx_perm.top_card:
                return ctx_perm.contains_card_name('Omnimon')
            return False

        eff_blocker = ICardEffect()
        eff_blocker.set_effect_name("EX4-073 Blocker (grant to Omnimon Digimon)")
        eff_blocker.set_effect_description("Blocker (grant to all [Omnimon] Digimon)")
        eff_blocker._is_blocker = True
        eff_blocker._applies_to_all_own_digimon = True
        eff_blocker.set_can_use_condition(_omnimon_blocker_condition)
        effects.append(eff_blocker)

        return effects
