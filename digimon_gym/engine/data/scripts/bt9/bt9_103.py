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

            # Part 1: Opponent's Digimon with play cost 7 or less can't attack players
            # until end of opponent's turn.  Registered as a GLOBAL modifier with a
            # dynamic condition — checks at attack time whether the attacking permanent
            # is an opponent's Digimon with play cost <= 7.  This covers Digimon
            # played AFTER Kongou resolves (C# uses AttackerCondition callback).
            def _cannot_attack_player_condition(target_perm, ctx):
                if not target_perm.is_digimon:
                    return False
                top = target_perm.top_card
                if top is None:
                    return False
                if top.owner is not enemy:
                    return False
                has_play_cost = getattr(top, 'has_play_cost', True)
                if not has_play_cost:
                    return False
                return top.get_cost_itself <= 7

            cap_entry = ModifierEntry(
                modifier_type=ModifierType.CANNOT_ATTACK_PLAYER,
                condition=_cannot_attack_player_condition,
                expiry='end_of_opponent_turn',
                granting_player=player,
            )
            game.modifiers.register(cap_entry)

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
