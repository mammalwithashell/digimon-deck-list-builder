from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_102(CardScript):
    """BT6-102 Tropical Venom | Option"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] 1 of your opponent's Digimon gains "[On Deletion] Lose 2 memory"
        # until the end of their next turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT6-102 Grant On Deletion lose 2 memory")
        effect0.set_effect_description(
            "[Main] 1 of your opponent's Digimon gains \"[On Deletion] Lose 2 "
            "memory\" until the end of their next turn."
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
                return p.is_digimon

            def on_target(target_perm):
                # Grant "[On Deletion] Lose 2 memory" until end of their next turn
                granted = ICardEffect()
                granted.set_timing(EffectTiming.OnDestroyedAnyone)
                granted.set_effect_name("BT6-102 Granted: On Deletion lose 2 memory")

                def granted_condition(context: Dict[str, Any]) -> bool:
                    destroyed_perm = context.get('permanent')
                    return destroyed_perm is target_perm

                granted.set_can_use_condition(granted_condition)

                def granted_process(ctx2: Dict[str, Any]):
                    owner = target_perm.top_card.owner if target_perm.top_card else None
                    if owner:
                        owner.lose_memory(2)
                        if game:
                            game.logger.log(
                                f"[BT6-102] {target_perm.top_card.card_names[0]} "
                                f"deleted — {owner.player_name} loses 2 memory."
                            )

                granted.set_on_process_callback(granted_process)

                # Grant until end of opponent's next turn
                target_perm.grant_temp_effect(granted, expiry_turn=game.turn_count + 1)

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to gain '[On Deletion] Lose 2 memory'."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
