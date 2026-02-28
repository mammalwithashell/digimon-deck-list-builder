from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_045(CardScript):
    """EX6-045 Tsukaimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Delete 1 of your opponent's level 3 Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-045 Delete 1 of your opponent's level 3 Digimon.")
        effect0.set_effect_description("[On Deletion] Delete 1 of your opponent's level 3 Digimon.")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by deleting 1 of your other Digimon, end the attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-045 End the attack by deleting 1 of your Digimon")
        effect1.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by deleting 1 of your other Digimon, end the attack.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("StopAttack_EX6-045")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
