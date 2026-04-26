from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST17_12(CardScript):
    """ST17-12 Giant Missile | Option"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Suspend 1 of your opponent's Digimon. Then, return 1 of their
        # suspended Digimon to the bottom of the deck. 1 of their Digimon can't
        # unsuspend until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST17-12 Suspend, bounce, can't unsuspend")
        effect0.set_effect_description(
            "[Main] Suspend 1 of your opponent's Digimon. Then, return 1 of "
            "their suspended Digimon to the bottom of the deck. 1 of their "
            "Digimon can't unsuspend until the end of their turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Suspend 1 of your opponent's Digimon
            def suspend_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()

            has_unsuspended = any(suspend_filter(p) for p in enemy.battle_area)
            if has_unsuspended:
                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=suspend_filter,
                    is_optional=False,
                    prompt="Suspend 1 of your opponent's Digimon."
                )

            # Step 2: Return 1 of their suspended Digimon to the bottom of the deck
            def bounce_filter(p):
                return p.is_digimon and p.is_suspended

            def on_bounce(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            has_suspended = any(bounce_filter(p) for p in enemy.battle_area)
            if has_suspended:
                game.effect_select_opponent_permanent(
                    player, on_bounce,
                    filter_fn=bounce_filter,
                    is_optional=False,
                    prompt="Return 1 of opponent's suspended Digimon to the bottom of the deck."
                )

            # Step 3: 1 of their Digimon can't unsuspend until end of their turn
            def lockdown_filter(p):
                return p.is_digimon

            def on_lockdown(target_perm):
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_UNSUSPEND,
                    expiry='end_of_opponent_turn',
                )

            has_digimon = any(lockdown_filter(p) for p in enemy.battle_area)
            if has_digimon:
                game.effect_select_opponent_permanent(
                    player, on_lockdown,
                    filter_fn=lockdown_filter,
                    is_optional=False,
                    prompt="Select 1 of opponent's Digimon that can't unsuspend until end of their turn."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect [Security] Activate this card's [Main] effects.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST17-12 Security: Play this card")
        effect1.set_effect_description("Security: Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
