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
            """Shared logic for [Main] and [Security] -- apply both restrictions."""
            enemy = player.enemy
            if not enemy:
                return

            # Part 1: Opponent's Digimon with play costs 7 or less can't attack players
            # Duration: until end of opponent's turn (turn_count + 1)
            expiry_turn = game.turn_count + 1
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    play_cost = perm.top_card.get_cost_itself if perm.top_card else 0
                    if play_cost <= 7:
                        perm.grant_keyword('_is_cannot_attack_player', duration=expiry_turn)

            # Part 2: Cards can't be added to security stacks by opponent's effects
            # NOTE: Engine does not yet enforce CANNOT_ADD_SECURITY at
            # recovery/add-security decision points. This registers the
            # modifier for future engine support.
            from digimon_gym.engine.interfaces.modifiers import ModifierType, ModifierEntry
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
        effect0.set_effect_name("BT9-103 Opponent restrictions")
        effect0.set_effect_description(
            "[Main] Until the end of your opponent's turn, your opponent's "
            "Digimon with play costs of 7 or less can't attack players, and "
            "cards can't be added to security stacks by your opponent's effects."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Apply both cannot-attack-player and cannot-add-security."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_main_effect(player, game)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT9-103 Security Effect")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Same as main effect -- apply both restrictions."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_main_effect(player, game)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
