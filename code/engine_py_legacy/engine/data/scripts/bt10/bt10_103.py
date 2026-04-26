from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_103(CardScript):
    """BT10-103 Gran del Sol"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When you would use this card, if you have 2 or more suspended green Digimon
        # in play, reduce the memory cost by 2.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT10-103 Cost -2")
        effect0.set_effect_description(
            "When you would use this card, if you have 2 or more suspended "
            "green Digimon in play, reduce the memory cost by 2."
        )
        effect0.cost_reduction = 2

        def condition0(context: Dict[str, Any]) -> bool:
            # Only applies when this card is being played
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            from ....data.enums import CardColor
            suspended_green_count = sum(
                1 for p in owner.battle_area
                if p.is_digimon and p.is_suspended
                and CardColor.Green in (getattr(p.top_card, 'card_colors', []) or [])
            )
            return suspended_green_count >= 2

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Suspend 1 of your opponent's Digimon. Then, return 1 of your opponent's suspended Digimon to the bottom of its owner's deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT10-103 Suspend, Bounce")
        effect1.set_effect_description("[Main] Suspend 1 of your opponent's Digimon. Then, return 1 of your opponent's suspended Digimon to the bottom of its owner's deck.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Suspend 1 of your opponent's Digimon
            def suspend_filter(p):
                return p.is_digimon
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=suspend_filter, is_optional=False)

            # Then, return 1 of your opponent's suspended Digimon to the bottom
            # of its owner's deck
            def bounce_filter(p):
                return p.is_digimon and p.is_suspended
            def on_bounce(target_perm):
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=bounce_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-103 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
