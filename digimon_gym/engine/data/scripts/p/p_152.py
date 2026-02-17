from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_152(CardScript):
    """P-152 Shoutmon + Dorulu Cannon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("P-152 Also treated as [Shoutmon]/[Dorulumon]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("P-152 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Shoutmon] for cost 0
        effect1._alt_digi_cost = 0
        effect1._alt_digi_name = "Shoutmon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Shoutmon') or permanent.contains_card_name('Dorulumon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] 1 of your opponent�s Digimon gets -2000 DP for the turn. Then, by placing 1 Digimon card with the [Xros Heart] trait in this Digimon�s digivolution cards under 1 of your Tamers, delete 1 of their Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-152 One of your opponents Digimon gets -2000 DP for the turn, then delete 1 of their Digimon wiht 3000 DP  or less")
        effect2.set_effect_description("[When Attacking] 1 of your opponent�s Digimon gets -2000 DP for the turn. Then, by placing 1 Digimon card with the [Xros Heart] trait in this Digimon�s digivolution cards under 1 of your Tamers, delete 1 of their Digimon with 3000 DP or less.")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -2000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 3000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("P-152 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
