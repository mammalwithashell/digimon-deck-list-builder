from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_099(CardScript):
    """Auto-transpiled from DCGO BT24_099.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-099 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] By trashing 1 [Appmon] trait card from your hand, <Draw 2>. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-099 Trash 1, Draw 2")
        effect1.set_effect_description("[Main] By trashing 1 [Appmon] trait card from your hand, <Draw 2>. Then, place this card in the battle area.")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(2)
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-099 Link 1 [Appmon] from trash")
        effect2.set_effect_description("Effect")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-099 place in battle area")
        effect3.set_effect_description("[Security] place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
