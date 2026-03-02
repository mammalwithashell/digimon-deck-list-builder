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

        # --- Effect 0: [Main] Can't attack players (play cost <= 7) ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT9-103 Opponent Digimon can't attack players")
        effect0.set_effect_description(
            "[Main] Until the end of your opponent's turn, your opponent's "
            "Digimon with play costs of 7 or less can't attack players."
        )
        effect0.is_on_play = True
        effect0._is_cannot_attack_player = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Grant cannot_attack_player to opponent Digimon with play cost <= 7."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    play_cost = getattr(perm.top_card, 'play_cost', 0) or 0
                    if play_cost <= 7:
                        perm.grant_keyword(
                            '_is_cannot_attack_player',
                            condition=lambda p, e=enemy: game.current_player == e,
                            granting_player=player,
                        )
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
            """Same as main effect — apply cannot_attack_player."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    play_cost = getattr(perm.top_card, 'play_cost', 0) or 0
                    if play_cost <= 7:
                        perm.grant_keyword(
                            '_is_cannot_attack_player',
                            condition=lambda p, e=enemy: game.current_player == e,
                            granting_player=player,
                        )
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
