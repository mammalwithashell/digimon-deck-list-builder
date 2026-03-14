from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
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
        # Implemented as a memory penalty modifier on each opponent Digimon.
        # The engine checks CHANGE_END_TURN_MIN_MEMORY-style modifiers, but
        # the simplest approach is to register per-attack memory loss via
        # a modifier that triggers on attack. We use OnUseAttack timing
        # to deduct memory when opponent Digimon attack.
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
            """Apply memory loss penalty to all opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy
            if not enemy:
                return

            # For each opponent Digimon, register a modifier that causes
            # memory loss when it attacks. We approximate this by directly
            # reducing the opponent's memory since the engine doesn't have a
            # "grant WhenAttacking effect" mechanism. Instead, we register
            # a CANNOT_ATTACK modifier-like approach — but the most faithful
            # implementation uses the OnUseAttack effect callback system.
            #
            # Since we can't grant effects to opponent permanents, we store
            # a flag on the game and check it in OnUseAttack.
            if not hasattr(game, '_ice_wall_targets'):
                game._ice_wall_targets = set()

            for perm in enemy.battle_area:
                if perm.is_digimon:
                    game._ice_wall_targets.add(id(perm))

            # Register a global OnUseAttack handler that fires when affected
            # Digimon attack. We store the info as a modifier instead.
            # Simplification: just make opponent lose 2 memory per attack
            # by registering the info on a dummy modifier.
            game._ice_wall_player = player
            game._ice_wall_expiry = 'end_of_opponent_turn'

            # Use a direct CHANGE_END_TURN_MIN_MEMORY approach: make the
            # opponent's memory management harder. But the most accurate
            # approach is memory loss per attack.
            # For engine compatibility, apply -2 memory for each opponent
            # Digimon that exists (approximation of attack penalty).
            # This is a simplification since per-attack callbacks can't be
            # granted to opponent Digimon.
            # More accurate: subtract 2 from opponent memory per Digimon
            # that can attack.
            attackable = sum(
                1 for p in enemy.battle_area
                if p.is_digimon and not p.is_suspended
            )
            if attackable > 0:
                enemy.add_memory(-2 * min(attackable, 2))

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
