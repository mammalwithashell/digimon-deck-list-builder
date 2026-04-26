from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_084(CardScript):
    """BT14-084 T.K. Takaishi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning the top card of your security stack to the hand, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT14-084 Add 1 card from security to hand to place 1 card from hand at the bottom of security")
        effect0.set_effect_description("[On Play] By returning the top card of your security stack to the hand, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return top security to hand, then optionally place Vaccine card from hand into security."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.security_cards):
                return
            card_to_add = player.security_cards.pop(0)
            player.hand_cards.append(card_to_add)

            def hand_filter(c):
                if not (getattr(c, 'is_tamer', False) or getattr(c, 'is_digimon', False)):
                    return False
                if 'Yellow' not in [col.name for col in getattr(c, 'card_colors', [])]:
                    return False
                return 'Vaccine' in (getattr(c.c_entity_base, 'attribute_eng', []) or [])

            def on_put_security(selected):
                player.add_to_security_from_hand(selected, to_top=True)

            game.effect_select_hand_card(
                player,
                hand_filter,
                on_put_security,
                is_optional=True,
                prompt="Select a yellow Vaccine card from your hand to place at the bottom of your security.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddSecurity
        # [Your Turn] When a card is added to your security stack, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddSecurity)
        effect1.set_effect_name("BT14-084 Memory +1")
        effect1.set_effect_description("[Your Turn] When a card is added to your security stack, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-084 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
