from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_008(CardScript):
    """BT13-008 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-008 Your 1 [Marcus Damon] becomes Digimon")
        effect1.set_effect_description("[Main][Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also treated as a 3000 DP Digimon and can't digivolve.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BecomeDigimon_BT13_008")
        effect1.is_declarative = True # Main phase effect

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check if player has a Marcus Damon
            player = context.get('player')
            if not player:
                return False
            has_marcus = any(p.is_tamer and p.contains_card_name("Marcus Damon") for p in player.battle_area)
            return has_marcus

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Treat Tamer as Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def filter_marcus(p):
                return p.is_tamer and p.contains_card_name("Marcus Damon")

            def on_select(target_perm):
                # Apply effect: Treat as 3000 DP Digimon, Can't digivolve
                # Note: Engine support for type change is pending.
                # We apply flags that might be supported or just log it.
                # Setting DP is possible.
                target_perm.change_dp(3000) # This adds +3000, doesn't set base.
                # If base is None (Tamer), dp property might handle it if we set something?
                # Currently Permanent.dp returns 0 if base is None.
                # If we assume engine will be updated to handle "Treat as Digimon", we mark it.
                # For now, we stub it.
                game.logger.log(f"[BT13-008] {target_perm.top_card.card_names[0]} treated as Digimon (Simulated)")

            game.effect_select_own_permanent(
                player, on_select, filter_fn=filter_marcus, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-008 Delete 1 Digimon with 3000 DP or less")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT13_008")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Check if the suspended card was a Red or Yellow Tamer owned by player
            # Context key 'suspended_permanent' assumed for OnTappedAnyone
            suspended = context.get('suspended_permanent')
            if not suspended:
                # Fallback: if engine doesn't provide specific context, we might return False
                # to avoid false positives.
                return False

            if not suspended.is_tamer:
                return False

            player = context.get('player')
            if suspended not in player.battle_area:
                return False

            # Check colors
            # suspended.top_card.card_colors returns List[CardColor]
            colors = suspended.top_card.card_colors
            if not (CardColor.Red in colors or CardColor.Yellow in colors):
                return False

            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
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
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
