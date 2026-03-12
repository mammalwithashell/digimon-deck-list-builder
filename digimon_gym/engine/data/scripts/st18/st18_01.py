from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_01(CardScript):
    """ST18-01 Fluffymon | Digi-Egg Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: is_on_attack (OnUseAttack path — this Digimon is the attacker)
        # [When Attacking][Once Per Turn] You may suspend 1 other Digimon with DP less
        # than or equal to this Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-01 Suspend 1 other Digimon with DP <= this Digimon")
        effect0.set_effect_description(
            "[When Attacking][Once Per Turn] You may suspend 1 other Digimon with DP "
            "less than or equal to this Digimon."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("SuspendOther_ST18_01")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner_perm = card.permanent_of_this_card()
            attacker = context.get('attacker')
            # Only fires when this Digimon is the one attacking
            if owner_perm is None or attacker is None or owner_perm is not attacker:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Suspend 1 other own Digimon with DP <= this Digimon's DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game and perm):
                return
            this_dp = perm.dp if perm.dp is not None else 0

            def target_filter(p):
                if p is perm:
                    return False
                if not p.is_digimon:
                    return False
                if p.is_suspended:
                    return False
                target_dp = p.dp if p.dp is not None else 0
                return target_dp <= this_dp

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True,
                prompt="Select 1 other Digimon with DP <= this Digimon to suspend.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
