from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_095(CardScript):
    """Auto-transpiled from DCGO BT24_095.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-095 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-095 Suspend 1 of opponent's Digimon or Tamers. It can't unsuspend in their next unsuspend phase. Then, you may link this card.")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend 1 opponent's Digimon or Tamer. Then link."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_target_selected(target_perm):
                target_perm.suspend()
                game.logger.log(
                    f"[Effect] Suspended {target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'} "
                    f"(can't unsuspend in next unsuspend phase)")
                # Then, may link this card
                game.effect_link_to_permanent(player, card, is_optional=True)

            game.effect_select_opponent_permanent(
                player, on_target_selected,
                filter_fn=lambda p: p.is_digimon or p.is_tamer)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Return 1 of your opponent's suspended Digimon to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-095 Bounce 1 opponent's suspended Digimon.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] Return 1 of your opponent's suspended Digimon to the hand.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-095")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce 1 opponent's suspended Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_selected(target_perm):
                player.bounce_permanent_to_hand(target_perm)
                game.logger.log(
                    f"[Effect] Bounced {target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'}")

            game.effect_select_opponent_permanent(
                player, on_selected,
                filter_fn=lambda p: p.is_digimon and p.is_suspended)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
