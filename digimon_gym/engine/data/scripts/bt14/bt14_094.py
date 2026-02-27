from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_094(CardScript):
    """BT14-094 Heaven's Knuckle"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Activate 1 of the effects below:
        # - 1 opponent's Digimon gets -6000 DP for the turn.
        # - By deleting 1 of your [Angemon], place 1 of your opponent's Digimon at the bottom of their security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-094 Choose: -6000 DP OR delete [Angemon] to bottom security")
        effect0.set_effect_description("[Main] Activate 1 of the effects below: - 1 of your opponent's Digimon gets -6000 DP for the turn. - By deleting 1 of your [Angemon], place 1 of your opponent's Digimon at the bottom of their security stack.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy
            enemy_digimon = [p for p in (enemy.battle_area if enemy else []) if p.is_digimon]
            if not enemy_digimon:
                return

            own_angemon = [
                p for p in player.battle_area
                if p.is_digimon and 'Angemon' in (getattr(p, 'name', '') or '')
            ]

            # Branch 1: simple -6000 DP to one opponent Digimon.
            # Branch 2: if possible, delete 1 own [Angemon] to place 1 opponent Digimon at bottom security.
            use_branch2 = bool(own_angemon)

            if use_branch2:
                # deterministically choose an [Angemon] to delete and an opponent Digimon to move
                angemon_target = own_angemon[0]
                player.delete_permanent(angemon_target)

                # choose opponent Digimon with lowest DP for deterministic behavior
                move_target = min(
                    enemy_digimon,
                    key=lambda p: (p.dp if p.dp is not None else 999999)
                )
                enemy.put_permanent_to_security(move_target)
                return

            # Fallback / chosen branch 1
            dp_target = min(
                enemy_digimon,
                key=lambda p: (p.dp if p.dp is not None else 999999)
            )
            dp_target.change_dp(-6000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
