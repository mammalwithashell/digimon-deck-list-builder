from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_104(CardScript):
    """BT5-104 Catastrophe Cannon | Option (White, Cost 4)

    [Main] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if you have
        a [Diaboromon] in play, you may play 1 [Diaboromon] Token without
        paying the cost.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _main_logic(player, game):
            """Shared logic for [Main] and [Security]."""
            enemy = player.enemy
            if not enemy:
                return

            def dedigivolve_filter(p):
                return p.is_digimon and len(p.card_sources) > 1

            def on_dedigivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                for rc in removed:
                    enemy.trash_cards.append(rc)
                game.logger.log(f"[BT5-104] De-Digivolved opponent's Digimon by 2.")

                # Then, if you have a Diaboromon in play, play 1 token
                has_diaboromon = any(
                    p.contains_card_name('Diaboromon')
                    for p in player.battle_area if p.is_digimon
                )
                if has_diaboromon:
                    game.effect_play_token(player, 'diaboromon')

            has_target = any(dedigivolve_filter(p) for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_dedigivolve, filter_fn=dedigivolve_filter,
                    is_optional=False,
                    prompt="Select opponent's Digimon to De-Digivolve 2."
                )
            else:
                # No valid de-digivolve target, but still check for token play
                has_diaboromon = any(
                    p.contains_card_name('Diaboromon')
                    for p in player.battle_area if p.is_digimon
                )
                if has_diaboromon:
                    game.effect_play_token(player, 'diaboromon')

        # --- Effect 0: [Main] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT5-104 De-Digivolve 2, then play Diaboromon Token")
        effect0.set_effect_description(
            "[Main] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if "
            "you have a [Diaboromon] in play, you may play 1 [Diaboromon] Token "
            "without paying the cost."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _main_logic(player, game)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT5-104 Security: Activate [Main]")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _main_logic(player, game)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
