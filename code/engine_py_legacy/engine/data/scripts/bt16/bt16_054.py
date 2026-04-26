from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_054(CardScript):
    """BT16-054 Sealsdramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gains <Rush>, and can't be blocked for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT16-054 <Rush>, Can't be Blocked")
        effect0.set_effect_description("[On Play] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gains <Rush>, and can't be blocked for the turn.")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_rush = True
        effect0._is_cannot_be_blocked = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Rush, Gain Keyword Cannot Be Blocked"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_rush')
                perm.grant_keyword('_is_cannot_be_blocked')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gains <Rush>, and can't be blocked for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT16-054 <Rush>, Can't be Blocked")
        effect1.set_effect_description("[When Digivolving] By returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, this Digimon gains <Rush>, and can't be blocked for the turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_rush = True
        effect1._is_cannot_be_blocked = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Rush, Gain Keyword Cannot Be Blocked"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_rush')
                perm.grant_keyword('_is_cannot_be_blocked')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-054 All your Digimon DP modifier")
        effect2.set_effect_description("All your Digimon DP modifier")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 1000
        effect2._applies_to_all_own_digimon = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
