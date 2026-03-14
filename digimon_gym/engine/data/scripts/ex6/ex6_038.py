from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_038(CardScript):
    """EX6-038 Ludomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-038 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Kakkinmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] By paying 1 cost and placing this card as the bottom digivolution card of 1 of your Digimon that's level 3 or has the [Legend-Arms] trait, that Digimon gets +2000 DP until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("EX6-038 +2000 DP")
        effect1.set_effect_description("[Hand] [Main] By paying 1 cost and placing this card as the bottom digivolution card of 1 of your Digimon that's level 3 or has the [Legend-Arms] trait, that Digimon gets +2000 DP until the end of your opponent's turn.")
        effect1.is_optional = True
        effect1.dp_modifier = 2000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When an effect places a digivolution card under this Digimon, <Draw 1>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect2.set_effect_name("EX6-038 Draw 1")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When an effect places a digivolution card under this Digimon, <Draw 1>.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Draw1_EX6_038")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: dp_modifier
        # DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-038 DP modifier")
        effect3.set_effect_description("DP modifier")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
