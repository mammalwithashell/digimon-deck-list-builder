from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_098(CardScript):
    """BT3-098 Plasma Stake | Option Purple Cost 4
    [Main] Delete 1 of your opponent's Digimon with 13000 DP or more.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Delete 1 opponent Digimon with 13000+ DP
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT3-098 Delete opponent 13000+ DP Digimon")
        effect0.set_effect_description(
            "[Main] Delete 1 of your opponent's Digimon with 13000 DP or more."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.dp >= 13000

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Activate this card's [Main] effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT3-098 Security: Delete opponent 13000+ DP")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.dp >= 13000

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
