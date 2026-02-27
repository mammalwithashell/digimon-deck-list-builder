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
            has_enemy_digimon = bool(enemy and enemy.battle_area and any(p.is_digimon for p in enemy.battle_area))
            own_angemon = [p for p in (player.battle_area or []) if p.is_digimon and 'Angemon' in (getattr(p, 'name', '') or '')]

            # If only one legal mode exists, resolve it directly.
            if has_enemy_digimon and not own_angemon:
                def on_dp_target(target_perm):
                    if target_perm:
                        target_perm.change_dp(-6000)
                game.effect_select_opponent_permanent(
                    player,
                    on_dp_target,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                )
                return

            if own_angemon and not has_enemy_digimon:
                def after_delete(_deleted):
                    return
                game.effect_select_own_permanent(
                    player,
                    after_delete,
                    filter_fn=lambda p: p.is_digimon and 'Angemon' in (getattr(p, 'name', '') or ''),
                    is_optional=False,
                )
                return

            # If both modes are available, choose one via an optional own-target selection for Angemon.
            # Choosing an Angemon means mode 2; cancel/skip means mode 1.
            if own_angemon and has_enemy_digimon:
                chosen = {'used_mode2': False}

                def on_choose_angemon(angemon_perm):
                    if not angemon_perm:
                        return
                    chosen['used_mode2'] = True

                    def after_enemy_target(enemy_perm):
                        if not enemy_perm:
                            return
                        # Delete chosen Angemon, then place chosen opponent Digimon at bottom of their security.
                        if hasattr(angemon_perm, 'delete'):
                            angemon_perm.delete()
                        elif hasattr(player, 'delete_permanent'):
                            player.delete_permanent(angemon_perm)
                        if enemy and hasattr(enemy, 'put_permanent_to_security'):
                            enemy.put_permanent_to_security(enemy_perm)

                    game.effect_select_opponent_permanent(
                        player,
                        after_enemy_target,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                    )

                game.effect_select_own_permanent(
                    player,
                    on_choose_angemon,
                    filter_fn=lambda p: p.is_digimon and 'Angemon' in (getattr(p, 'name', '') or ''),
                    is_optional=True,
                )

                if not chosen['used_mode2']:
                    def on_dp_target(target_perm):
                        if target_perm:
                            target_perm.change_dp(-6000)
                    game.effect_select_opponent_permanent(
                        player,
                        on_dp_target,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                    )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
