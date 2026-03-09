from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_11(CardScript):
    """ST18-11 Parrotmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Suspend 1 of your opponent's Digimon. Then, 1 of your
        # opponent's Digimon can't unsuspend until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST18-11 Suspend 1 opponent's Digimon + can't unsuspend")
        effect0.set_effect_description(
            "[On Play] Suspend 1 of your opponent's Digimon. Then, 1 of your "
            "opponent's Digimon can't unsuspend until the end of their turn."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_suspend(target_perm):
                target_perm.suspend()
                # Then, 1 opponent's Digimon can't unsuspend
                def lock_filter(p):
                    return p.is_digimon

                def on_lock(lock_target):
                    if game:
                        from digimon_gym.engine.interfaces.modifiers import ModifierType
                        game.register_modifier(
                            ModifierType.CANNOT_UNSUSPEND, lock_target,
                            value_fn=lambda: True, expiry='end_of_opponent_turn')

                game.effect_select_opponent_permanent(
                    player, on_lock, filter_fn=lock_filter, is_optional=False)

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: <Piercing>
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-11 Piercing")
        effect1.set_effect_description("Piercing")
        effect1._is_piercing = True
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
