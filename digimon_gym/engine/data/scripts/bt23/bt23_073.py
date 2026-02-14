from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_073(CardScript):
    """BT23-073"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's level 3 Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-073 Delete 1 level 3 Digimon")
        effect0.set_effect_description("[On Play] Delete 1 of your opponent's level 3 Digimon.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your other Digimon with the [Eater] or [Hudie] trait would leave the battle area, by deleting this Digimon or placing it as the bottom digivolution card of your [Mother Eater] in the breeding area, 1 of those Digimon doesn't leave.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-073 Prevent a Digimon from leaving battle area")
        effect1.set_effect_description("[All Turns] [Once Per Turn] When any of your other Digimon with the [Eater] or [Hudie] trait would leave the battle area, by deleting this Digimon or placing it as the bottom digivolution card of your [Mother Eater] in the breeding area, 1 of those Digimon doesn't leave.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Substitute_BT23_073")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -1
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-073 Play Cost -1")
        effect2.set_effect_description("Cost -1")
        effect2.cost_reduction = 1

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction handled via cost_reduction property

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
