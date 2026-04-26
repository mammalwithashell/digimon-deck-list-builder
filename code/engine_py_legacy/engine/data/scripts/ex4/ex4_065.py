from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_065(CardScript):
    """EX4-065 Trident Gaia | Option | Red | Cost:8

    [Main] Delete 1 of your opponent's Digimon with the highest DP. If a
        Digimon with 13000 DP or more is deleted by this effect, trash the top
        card of your opponent's security stack.
    [Security] Activate this card's [Main] effect.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete highest-DP opponent Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX4-065 Trident Gaia")
        effect0.set_effect_description(
            "[Main] Delete 1 of your opponent's Digimon with the highest DP. "
            "If a Digimon with 13000 DP or more is deleted by this effect, "
            "trash the top card of your opponent's security stack."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete opponent's highest-DP Digimon, then trash top security if 13000+."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Find the highest DP among opponent's Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if not opp_digimon:
                return
            max_dp = max(p.dp for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp == max_dp

            def on_delete(target_perm):
                deleted_dp = target_perm.dp or 0
                enemy.delete_permanent(target_perm)
                # If deleted Digimon had 13000+ DP, trash top of opponent security
                if deleted_dp >= 13000:
                    if enemy.security_cards:
                        top_sec = enemy.security_cards[-1]
                        enemy.trash_security_card(top_sec)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate this card's [Main] effect ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX4-065 Trident Gaia (Security)")
        effect1.set_effect_description(
            "[Security] Activate this card's [Main] effect."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(process0)
        effects.append(effect1)

        return effects
