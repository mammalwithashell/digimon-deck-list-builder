from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_134(CardScript):
    """P-134 Shoemon | Lv.3

    [On Play] 1 of your opponent's Digimon gains <Security A. -1> until the
        end of their turn.

    --- Inherited ---
    [When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
        for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Grant SA -1 to 1 opponent's Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-134 <Security A. -1> until the end of opponent's turn")
        effect0.set_effect_description("[On Play] 1 of your opponent's Digimon gains <Security A. -1> until the end of their turn.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                # Grant SA -1 via a temporary effect on the target
                sa_effect = ICardEffect()
                sa_effect.set_timing(EffectTiming.NoTiming)
                sa_effect.set_effect_name("P-134 SA -1 (granted)")
                sa_effect._security_attack_modifier = -1
                sa_effect.is_inherited_effect = False

                def sa_condition(context: Dict[str, Any]) -> bool:
                    return True
                sa_effect.set_can_use_condition(sa_condition)

                if target_perm.top_card:
                    target_perm.top_card._card_effects.append(sa_effect)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent's Digimon to give <Security A. -1>."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1 (Inherited): [When Attacking][Once Per Turn] -2000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("P-134 Opponent's Digimon gets -2000 DP for the turn")
        effect1.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("ChangeDP_P_134")
        effect1.is_on_attack = True

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
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(-2000)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent's Digimon to give -2000 DP."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
