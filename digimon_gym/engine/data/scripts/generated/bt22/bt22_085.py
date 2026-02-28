from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_085(CardScript):
    """BT22-085 Rina Shinomiya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-085 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon with [Veedramon] in its name gets +3000 DP until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-085 1 digimon with [Veedramon] in name gains +3k DP ")
        effect1.set_effect_description("[On Play] 1 of your Digimon with [Veedramon] in its name gets +3000 DP until your opponent's turn ends.")
        effect1.is_on_play = True
        effect1.dp_modifier = 3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn] When one of your Digimon with [Veedramon] in its name attacks, by returning this Tamer to the hand, that Digimon gains <Jamming> for the turn. (This Digimon can't be deleted in battles against Security Digimon.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-085 Bounce this tamer to hand, give attacking digimon <Jamming>")
        effect2.set_effect_description("[Your Turn] When one of your Digimon with [Veedramon] in its name attacks, by returning this Tamer to the hand, that Digimon gains <Jamming> for the turn. (This Digimon can't be deleted in battles against Security Digimon.)")
        effect2.is_optional = True
        effect2.is_on_attack = True
        effect2._is_jamming = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Jamming"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_jamming')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-085 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
