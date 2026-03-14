from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX3_072(CardScript):
    """EX3-072 Megiddo Flame | Option Purple

    [Main] Delete 1 of your opponent's level 4 or lower Digimon. By
        deleting 1 of your Digimon, delete 1 of your opponent's level 6
        or lower Digimon instead.
    [Security] You may play 1 [Guilmon] from your trash without paying
        the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete opp lv4 or lower, or by deleting
        #     own Digimon, delete lv6 or lower instead ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX3-072 Delete opp Digimon with optional upgrade")
        effect0.set_effect_description(
            "[Main] Delete 1 of your opponent's level 4 or lower Digimon. By "
            "deleting 1 of your Digimon, delete 1 of your opponent's level 6 "
            "or lower Digimon instead.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Check if player has own Digimon to delete for the upgrade
            own_digimon = [p for p in player.battle_area if p.is_digimon]

            # Player chooses: delete own Digimon for lv6 threshold,
            # or just use base lv4 threshold
            if own_digimon:
                # Give player the choice via effect_choose_branch
                def on_branch(branch: int):
                    if branch == 0:
                        # Base: delete lv4 or lower
                        _delete_opp_by_level(4)
                    else:
                        # Upgrade: delete own Digimon first, then lv6 or lower
                        def on_own_deleted(own_perm):
                            if own_perm is None:
                                return
                            player.delete_permanent(own_perm)
                            _delete_opp_by_level(6)

                        game.effect_select_own_permanent(
                            player, on_own_deleted,
                            filter_fn=lambda p: p.is_digimon,
                            is_optional=False,
                            prompt="Delete 1 of your Digimon to upgrade the deletion.")

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="Choose: delete opponent's lv4 or lower, OR delete own Digimon to delete lv6 or lower instead.",
                    branch_labels=["Delete lv4 or lower", "Delete own Digimon, then lv6 or lower"])
            else:
                # No own Digimon to delete, just use base
                _delete_opp_by_level(4)

            def _delete_opp_by_level(max_level: int):
                def target_filter(p):
                    return (p.is_digimon and
                            p.level is not None and p.level <= max_level)

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                if any(target_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter,
                        is_optional=False,
                        prompt=f"Delete 1 of your opponent's level {max_level} or lower Digimon.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Play 1 [Guilmon] from trash free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX3-072 Security: Play Guilmon from trash")
        effect1.set_effect_description(
            "[Security] You may play 1 [Guilmon] from your trash without paying the cost.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def guilmon_filter(c):
                return any('Guilmon' in n for n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'trash', guilmon_filter, free=True, is_optional=True,
                prompt="Play 1 [Guilmon] from trash without paying the cost.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
