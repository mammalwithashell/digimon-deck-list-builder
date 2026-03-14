from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_046(CardScript):
    """BT14-046 Togemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: [Your Turn] [Once Per Turn] When you would play a green Tamer
        # from your hand, by suspending 1 of your green Digimon, reduce the play cost by 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT14-046 Play Cost -3")
        effect0.set_effect_description(
            "[Your Turn] [Once Per Turn] When you would play a green Tamer "
            "from your hand, by suspending 1 of your green Digimon, reduce "
            "the play cost by 3."
        )
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT14_046_CostRed3")

        def _cost_reduction_fn(context: Dict[str, Any]) -> int:
            """Dynamic cost reduction: 3 if conditions met and a green Digimon is suspended."""
            played_card = context.get('card_source') or context.get('played_card')
            if not played_card:
                return 0
            # Must be a green Tamer
            from ....data.enums import CardColor
            colors = getattr(played_card, 'card_colors', []) or []
            if CardColor.Green not in colors:
                return 0
            if not getattr(played_card, 'is_tamer', False):
                return 0
            return 3

        effect0._cost_reduction_value_fn = _cost_reduction_fn

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the played card is a green Tamer
            played_card = context.get('card_source') or context.get('played_card')
            if not played_card:
                return False
            from ....data.enums import CardColor
            colors = getattr(played_card, 'card_colors', []) or []
            if CardColor.Green not in colors:
                return False
            if not getattr(played_card, 'is_tamer', False):
                return False
            # Must have a green Digimon to suspend
            owner = card.owner if card else None
            if not owner:
                return False
            has_green_digimon = any(
                p.is_digimon and not p.is_suspended
                and CardColor.Green in (getattr(p.top_card, 'card_colors', []) or [])
                for p in owner.battle_area
            )
            return has_green_digimon

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Suspend 1 of your green Digimon as the cost for the reduction."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor

            def green_digimon_filter(p):
                return (p.is_digimon and not p.is_suspended
                        and CardColor.Green in (getattr(p.top_card, 'card_colors', []) or []))

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_own_permanent(
                player, on_suspend,
                filter_fn=green_digimon_filter,
                is_optional=False,
                prompt="Suspend 1 of your green Digimon to reduce the play cost by 3."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1 (Inherited): [Your Turn] [Once Per Turn] When this Digimon
        # would digivolve, if you have a green Tamer, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name("BT14-046 Inherited: Digivolution cost -1")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon would digivolve, "
            "if you have a green Tamer, reduce the digivolution cost by 1."
        )
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT14_046_InhDigiCostRed")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a green Tamer on field
            owner = card.owner if card else None
            if not owner:
                return False
            from ....data.enums import CardColor
            has_green_tamer = any(
                p.is_tamer and CardColor.Green in (
                    getattr(p.top_card, 'card_colors', []) or []
                )
                for p in owner.battle_area
            )
            return has_green_tamer

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
