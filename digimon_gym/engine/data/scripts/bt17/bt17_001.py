from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_001(CardScript):
    """BT17-001 Gigimon | Digi-Egg Lv.2 Red"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Inherited: [When Attacking] By paying 1 cost, delete 1 of your
        #     opponent's Digimon with 3000 DP or less. ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-001 Pay 1 to delete opponent Digimon 3000 DP or less")
        effect0.set_effect_description(
            "[When Attacking] By paying 1 cost, delete 1 of your opponent's "
            "Digimon with 3000 DP or less."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Must be the attacking permanent
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            # Must have a valid opponent target with 3000 DP or less
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            has_target = any(
                p.is_digimon and (p.dp or 0) <= 3000
                for p in enemy.battle_area
            )
            return has_target

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Pay 1 cost, then delete 1 opponent Digimon with 3000 DP or less."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Pay 1 cost
            player.add_memory(-1)

            # Delete 1 opponent Digimon with 3000 DP or less
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) <= 3000

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
