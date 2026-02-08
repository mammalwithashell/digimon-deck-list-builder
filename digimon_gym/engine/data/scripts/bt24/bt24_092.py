from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_092(CardScript):
    """Auto-transpiled from DCGO BT24_092.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-092 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-092 1 opponent's Digimon gets -6K DP for the turn. Then, you may link this card.")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: 1 opponent's Digimon gets -6000 DP. Then link."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_target_selected(target_perm):
                target_perm.change_dp(-6000)
                game.logger.log(
                    f"[Effect] {target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'} "
                    f"gets -6000 DP")
                # Check if DP <= 0 (deletion by DP reduction)
                if target_perm.dp <= 0:
                    opp = player.enemy if player else None
                    if opp:
                        opp.delete_permanent(target_perm)
                # Then, may link this card
                game.effect_link_to_permanent(player, card, is_optional=True)

            game.effect_select_opponent_permanent(
                player, on_target_selected,
                filter_fn=lambda p: p.is_digimon)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -6000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-092 1 Opponent's digimon gets -6K DP for the turn.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -6000 DP for the turn.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-092")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: 1 opponent's Digimon gets -6000 DP for the turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_selected(target_perm):
                target_perm.change_dp(-6000)
                game.logger.log(
                    f"[Effect] {target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'} "
                    f"gets -6000 DP")
                if target_perm.dp <= 0:
                    opp = player.enemy if player else None
                    if opp:
                        opp.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_selected,
                filter_fn=lambda p: p.is_digimon)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
