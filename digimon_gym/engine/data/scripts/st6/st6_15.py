from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST6_15(CardScript):
    """ST6-15 Death Claw | Option Purple"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete 1 own Digimon to delete 1 opponent's
        #     level 4 or lower Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST6-15 Delete own Digimon, delete opponent Lv4 or lower")
        effect0.set_effect_description(
            "[Main] You may delete 1 of your Digimon to delete 1 of your "
            "opponent's level 4 or lower Digimon."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete 1 own Digimon, then delete 1 opponent Lv4 or lower"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Delete 1 of your Digimon as cost
            def own_filter(p):
                return p.is_digimon
            def on_own_delete(target_perm):
                player.delete_permanent(target_perm)
                # Then delete 1 opponent Digimon Lv.4 or lower
                def opp_filter(p):
                    return p.is_digimon and p.level is not None and p.level <= 4
                def on_opp_delete(opp_perm):
                    enemy.delete_permanent(opp_perm)
                game.effect_select_opponent_permanent(
                    player, on_opp_delete, filter_fn=opp_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_own_delete, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Delete 1 opponent's level 4 or lower Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("ST6-15 Security: Delete opponent Lv4 or lower")
        effect1.set_effect_description(
            "[Security] Delete 1 of your opponent's level 4 or lower Digimon."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent Digimon Lv4 or lower"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level <= 4
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
