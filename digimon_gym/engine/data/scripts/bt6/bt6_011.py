from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_011(CardScript):
    """BT6-011 SaviorHuckmon | Lv.5 Red Digimon

    Inherited Effect:
    [When Attacking][Once Per Turn] If you have a Digimon with [Sistermon]
        in its name in play, delete 1 of your opponent's Digimon with
        5000 DP or less.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [When Attacking][Once Per Turn] Delete opp Digimon <= 5000 DP ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("BT6-011 Inherited: delete opp Digimon <= 5000 DP")
        effect0.set_effect_description(
            "[When Attacking][Once Per Turn] If you have a Digimon with "
            "[Sistermon] in its name in play, delete 1 of your opponent's "
            "Digimon with 5000 DP or less."
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DeleteDigimon_BT6_011")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have a Sistermon in play
            has_sistermon = any(
                p.is_digimon and p.contains_card_name('Sistermon')
                for p in player.battle_area
            )
            return has_sistermon
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def delete_filter(p):
                return p.is_digimon and p.dp <= 5000

            has_target = any(delete_filter(p) for p in enemy.battle_area)
            if has_target:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=delete_filter,
                    is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
