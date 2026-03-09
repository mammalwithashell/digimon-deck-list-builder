from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_01(CardScript):
    """ST18-01 Fluffymon | Lv.2 (DigiEgg)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect [When Attacking] [Once Per Turn] You may suspend 1 other
        # Digimon with DP less than or equal to this Digimon.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("ST18-01 Suspend 1 other Digimon with DP <= this Digimon")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] You may suspend 1 other Digimon "
            "with DP less than or equal to this Digimon."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ST18_01_ESS_Suspend")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            my_dp = perm.dp or 0

            def target_filter(p):
                if p is perm:
                    return False
                return p.is_digimon and (p.dp or 0) <= my_dp

            def on_suspend(target_perm):
                target_perm.suspend()

            # "1 other Digimon" — can target own or opponent's
            all_targets = [p for p in player.battle_area + (player.enemy.battle_area if player.enemy else [])
                           if target_filter(p)]
            if all_targets:
                game.effect_select_any_permanent(
                    player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
