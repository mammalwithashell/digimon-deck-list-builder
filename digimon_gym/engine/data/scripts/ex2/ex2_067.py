from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_067(CardScript):
    """EX2-067 Fire Ball | Option Red Cost 2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Delete 1 of your opponent's Digimon with 3000 DP
        #     or less. If an opponent's Digimon wasn't deleted by this effect,
        #     Draw 2. ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX2-067 Delete 3000 DP or less, or Draw 2")
        effect0.set_effect_description(
            "[Main] Delete 1 of your opponent's Digimon with 3000 DP or less. "
            "If an opponent's Digimon wasn't deleted by this effect, <Draw 2>."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with 3000 DP or less; if none deleted, draw 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Try to find and delete a Digimon with 3000 DP or less
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) <= 3000

            candidates = [p for p in enemy.battle_area if target_filter(p)]

            if candidates:
                deleted = [False]

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                    deleted[0] = True

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

                # If the selection was resolved synchronously and nothing was deleted,
                # draw 2. The engine processes selections synchronously in headless mode.
                if not deleted[0]:
                    player.draw_cards(2)
            else:
                # No valid targets — draw 2
                player.draw_cards(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate this card's [Main] effects. ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX2-067 Security: Activate Main effect")
        effect1.set_effect_description(
            "[Security] Activate this card's [Main] effects."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Security: Same as Main — delete 3000 DP or less, or draw 2."""
            process0(ctx)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
