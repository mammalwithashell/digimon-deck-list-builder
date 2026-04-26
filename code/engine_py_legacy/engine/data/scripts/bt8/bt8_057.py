from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_057(CardScript):
    """BT8-057 Shivamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-057 Opponent can't us Option")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] When this Digimon becomes unsuspended during your unsuspend phase, trash the top card of your opponent's security stack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUnTappedAnyone)
        effect1.set_effect_name("BT8-057 Trash the top card of opponent's security")
        effect1.set_effect_description("[Your Turn] When this Digimon becomes unsuspended during your unsuspend phase, trash the top card of your opponent's security stack.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be self-unsuspend trigger
            unsuspended_perm = context.get('event_permanent', context.get('permanent'))
            host_perm = card.permanent_of_this_card() if card else None
            if unsuspended_perm is not host_perm:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
