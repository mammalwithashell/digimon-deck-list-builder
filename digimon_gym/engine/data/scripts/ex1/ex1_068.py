from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_068(CardScript):
    """EX1-068 Ice Wall! | Option (White, Cost 1)

    [Main] All of your opponent's Digimon gain "[When Attacking] lose 2
    memory" until the end of their next turn.
    [Security] Gain 2 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Grant opponent Digimon "When Attacking lose 2 memory"
        # Uses _is_when_attacked_observer on granted effects attached to
        # each opponent Digimon. The observer fires from
        # _fire_opponent_attack_observers in combat.py.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX1-068 Opponent Digimon lose 2 memory when attacking")
        effect0.set_effect_description(
            "[Main] All of your opponent's Digimon gain \"[When Attacking] "
            "lose 2 memory\" until the end of their next turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Grant per-attack memory penalty to all opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy
            if not enemy:
                return

            for perm in list(enemy.battle_area):
                if not perm.is_digimon:
                    continue

                penalty_effect = ICardEffect()
                penalty_effect.set_timing(EffectTiming.NoTiming)
                penalty_effect.set_effect_name("Ice Wall penalty")
                penalty_effect._is_when_attacked_observer = True

                def make_cb(target_player):
                    def cb(inner_ctx):
                        inner_game = inner_ctx.get('game')
                        if target_player and inner_game:
                            target_player.lose_memory(2)
                            inner_game.logger.log(
                                "[EX1-068] Digimon loses 2 memory when attacking (Ice Wall)")
                    return cb
                penalty_effect.set_on_process_callback(make_cb(enemy))
                penalty_effect.set_can_use_condition(lambda ctx: True)

                # Grant until end of opponent's next turn (~2 turns from now)
                expiry = game.turn_count + 2
                perm.grant_temp_effect(penalty_effect, expiry_turn=expiry)

            game.logger.log(
                "[EX1-068] Opponent's Digimon gain '[When Attacking] lose 2 memory'")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX1-068 Gain 2 memory")
        effect1.set_effect_description("[Security] Gain 2 memory.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
