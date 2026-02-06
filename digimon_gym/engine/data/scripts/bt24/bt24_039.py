from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_039(CardScript):
    """Auto-transpiled from DCGO BT24_039.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.SecuritySkill
        # [Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-039 Play this card without battling")
        effect0.set_effect_description("[Security] If your opponent has a level 6 or higher Digimon, play this card without battling and without paying the cost.")
        effect0.is_security_effect = True
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-039 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-039 Recovery +1 (Deck)")
        effect2.set_effect_description("[On Deletion] Trigger <Recovery +1 (Deck)>. (Place the top card of your deck on top of your security stack.)")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
