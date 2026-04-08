from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_103(CardScript):
    """BT9-103 Kongou | Option (Black, Cost 2)

    [Main] Until the end of your opponent's turn, your opponent's Digimon
        with play costs of 7 or less can't attack players, and cards can't
        be added to security stacks by your opponent's effects.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _apply_main_effect(player, game):
            """Shared logic for [Main] and [Security] — apply both restrictions."""
            from digimon_gym.engine.interfaces.modifiers import ModifierType, ModifierEntry
            enemy = player.enemy
            if not enemy:
                return

            # Part 1: Opponent's Digimon with play cost 7 or less can't attack PLAYERS
            # (they CAN still attack other Digimon) until end of opponent's turn.
            # C# ref: GainCanNotAttackPlayerEffect with attackerCondition + defenderCondition.
            # Must use CANNOT_ATTACK_PLAYER, NOT CANNOT_ATTACK.
            #
            # The condition must match only the specific target permanent —
            # ModifierRegistry.has_modifier iterates all entries of a type and
            # calls is_active(query_target); without an identity-filtering
            # condition, the modifier would apply to every permanent.
            for perm in list(enemy.battle_area):
                if not perm.is_digimon:
                    continue
                top = perm.top_card
                if top is None:
                    continue
                has_play_cost = getattr(top, 'has_play_cost', True)
                play_cost = top.get_cost_itself
                if has_play_cost and play_cost <= 7:
                    target_perm = perm
                    game.register_modifier(
                        target_perm,
                        ModifierType.CANNOT_ATTACK_PLAYER,
                        condition=(lambda p=target_perm: (lambda target, ctx: target is p))(),
                        expiry='end_of_opponent_turn',
                    )

            # Part 2: Cards can't be added to security stacks by opponent's effects
            # until end of opponent's turn.  Registered as a global modifier entry.
            entry = ModifierEntry(
                modifier_type=ModifierType.CANNOT_ADD_SECURITY,
                condition=lambda perm, ctx: True,
                expiry='end_of_opponent_turn',
                granting_player=player,
            )
            game.modifiers.register(entry)

        # --- Effect 0: [Main] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT9-103 Opponent restrictions until end of opponent's turn")
        effect0.set_effect_description(
            "[Main] Until the end of your opponent's turn, your opponent's "
            "Digimon with play costs of 7 or less can't attack players, and "
            "cards can't be added to security stacks by your opponent's effects."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_main_effect(player, game)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT9-103 Security Effect")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_main_effect(player, game)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
