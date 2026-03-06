from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_102(CardScript):
    """BT13-102 Keenan Crier"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent may trash 1 Tamer card or Option card in their hand.
        # If they don't, gain 1 memory and <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-102 Opponent trashes or you gain memory and draw")
        effect0.set_effect_description("[On Play] Your opponent may trash 1 Tamer card or Option card in their hand. If they don't, gain 1 memory and <Draw 1>.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Opponent may trash 1 Tamer/Option from hand; if not, gain 1 memory + draw 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def hand_filter(c):
                return getattr(c, 'is_tamer', False) or getattr(c, 'is_option', False)

            def on_decline():
                """Opponent declined to trash -- owner gains 1 memory and draws 1."""
                player.add_memory(1)
                player.draw_cards(1)

            # Build valid indices for the opponent's hand
            valid = []
            for i, c in enumerate(enemy.hand_cards):
                if hand_filter(c):
                    valid.append(i)  # SEL_HAND_START is 0, so index == action_id

            if not valid:
                # Opponent has no valid cards to trash -- auto-decline
                on_decline()
                return

            def on_select(action_id: int):
                idx = action_id  # SEL_HAND_START is 0
                if 0 <= idx < len(enemy.hand_cards):
                    selected = enemy.hand_cards[idx]
                    enemy.hand_cards.remove(selected)
                    enemy.trash_cards.append(selected)

            game.request_selection(
                GamePhase.SelectHand, enemy, on_select, valid,
                is_optional=True,
                prompt="You may trash 1 Tamer or Option card from your hand.",
                on_decline=on_decline,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-102 Suspend to gain memory")
        effect1.set_effect_description("[Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Opponent's turn only
            if card and card.owner and card.owner.is_my_turn:
                return False
            # Must not already be suspended
            this_perm = card.permanent_of_this_card()
            if this_perm and this_perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, gain 1 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Reward: gain 1 memory
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-102 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
