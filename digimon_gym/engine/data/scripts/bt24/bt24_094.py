from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_094(CardScript):
    """Auto-transpiled from DCGO BT24_094.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-094 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-094 Alliance")
        effect1.set_effect_description("Alliance")
        effect1._is_alliance = True
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-094 Your Digimon gain <Alliance>")
        effect2.set_effect_description("Effect")

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OptionSkill
        # [Main] Add your bottom security card to the hand and place this card face up as the bottom security card. Then, you may play 1 green or yellow [TS] trait Digimon card from your hand with the play cost reduced by 3.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-094 Replace your bottom sec with this face-up card, play a [TS] Digimon for -3")
        effect3.set_effect_description("[Main] Add your bottom security card to the hand and place this card face up as the bottom security card. Then, you may play 1 green or yellow [TS] trait Digimon card from your hand with the play cost reduced by 3.")

        def condition3(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # Play Card, Trash From Hand
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-094 Play Card, Trash From Hand")
        effect4.set_effect_description("Play Card, Trash From Hand")
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
