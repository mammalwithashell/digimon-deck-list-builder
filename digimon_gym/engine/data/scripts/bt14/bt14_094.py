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
        effect0.set_effect_name("BT14-094 Heaven's Knuckle")
        effect0.set_effect_description("[Main] Activate 1 of the effects below: - 1 opponent's Digimon gets -6000 DP for the turn. - By deleting 1 of your [Angemon], place 1 of your opponent's Digimon at the bottom of their security stack.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return

            enemy = player.enemy
            enemy_digimon = [p for p in (enemy.battle_area if enemy else []) if p.is_digimon]

            # Determine if mode 2 is available (you can delete one of your [Angemon]).
            own_angemon = [
                p for p in player.battle_area
                if p.is_digimon and getattr(p, 'name', None) and 'angemon' in p.name.lower()
            ]

            # If mode 2 isn't available, resolve mode 1 directly.
            if not own_angemon:
                if enemy_digimon:
                    target = min(
                        [p for p in enemy_digimon if p.dp is not None],
                        key=lambda p: p.dp,
                        default=None,
                    )
                    if target is not None:
                        target.change_dp(-6000)
                return

            # If no game selector is available, fallback deterministically to mode 1.
            if not game or not hasattr(game, 'prompt_choose_from_list'):
                if enemy_digimon:
                    target = min(
                        [p for p in enemy_digimon if p.dp is not None],
                        key=lambda p: p.dp,
                        default=None,
                    )
                    if target is not None:
                        target.change_dp(-6000)
                return

            # Choose exactly one mode.
            choice = game.prompt_choose_from_list(
                player,
                "Choose 1 effect for Heaven's Knuckle",
                ["-6000 DP", "Delete 1 [Angemon] to bottom-deck 1 opponent Digimon"],
                can_cancel=False,
            )

            if choice == 0:
                if enemy_digimon:
                    target = min(
                        [p for p in enemy_digimon if p.dp is not None],
                        key=lambda p: p.dp,
                        default=None,
                    )
                    if target is not None:
                        target.change_dp(-6000)
                return

            # Mode 2: delete 1 of your [Angemon], then place 1 opponent Digimon at bottom of security.
            def on_delete_angemon(angemon_perm):
                if angemon_perm is None:
                    return
                angemon_perm.delete()

                if not enemy or not enemy.battle_area:
                    return

                def enemy_filter(p):
                    return p.is_digimon

                def on_put_enemy_to_security(target_perm):
                    if target_perm is not None and enemy:
                        enemy.put_permanent_to_security(target_perm)

                if hasattr(game, 'effect_select_enemy_permanent'):
                    game.effect_select_enemy_permanent(
                        player, on_put_enemy_to_security, filter_fn=enemy_filter, is_optional=False
                    )
                else:
                    # Safe deterministic fallback: choose lowest-DP opponent Digimon.
                    candidates = [p for p in enemy.battle_area if p.is_digimon]
                    target = min([p for p in candidates if p.dp is not None], key=lambda p: p.dp, default=None)
                    if target is not None:
                        enemy.put_permanent_to_security(target)

            def own_angemon_filter(p):
                return p.is_digimon and getattr(p, 'name', None) and 'angemon' in p.name.lower()

            game.effect_select_own_permanent(
                player, on_delete_angemon, filter_fn=own_angemon_filter, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
