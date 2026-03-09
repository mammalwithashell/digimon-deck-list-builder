from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_15(CardScript):
    """ST18-15 Anemoi Embrace (Option)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Suspend 1 Digimon. If this effect suspended your Digimon,
        # return 1 of your opponent's suspended Digimon to the bottom of the
        # deck. Then, unsuspend the Digimon suspended by this effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST18-15 Suspend own Digimon, bounce opponent's suspended, unsuspend")
        effect0.set_effect_description(
            "[Main] Suspend 1 Digimon. If this effect suspended your Digimon, "
            "return 1 of your opponent's suspended Digimon to the bottom of the "
            "deck. Then, unsuspend the Digimon suspended by this effect."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()
                # If we suspended our own Digimon
                if target_perm in player.battle_area:
                    # Return 1 opponent's suspended Digimon to bottom of deck
                    enemy = player.enemy
                    if enemy:
                        def bounce_filter(p):
                            return p.is_digimon and p.is_suspended

                        def on_bounce(bounce_target):
                            enemy.return_permanent_to_deck_bottom(bounce_target)

                        game.effect_select_opponent_permanent(
                            player, on_bounce, filter_fn=bounce_filter,
                            is_optional=False)

                    # Then, unsuspend the Digimon suspended by this effect
                    target_perm.unsuspend()

            game.effect_select_any_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect [Security] Return 1 of your opponent's suspended
        # Digimon to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-15 Security: Bounce opponent's suspended Digimon")
        effect1.set_effect_description(
            "Security: Return 1 of your opponent's suspended Digimon to the "
            "bottom of the deck."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy:
                def bounce_filter(p):
                    return p.is_digimon and p.is_suspended

                def on_bounce(bounce_target):
                    enemy.return_permanent_to_deck_bottom(bounce_target)

                game.effect_select_opponent_permanent(
                    player, on_bounce, filter_fn=bounce_filter,
                    is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
