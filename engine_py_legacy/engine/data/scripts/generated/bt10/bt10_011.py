from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_011(CardScript):
    """BT10-011 Canoweissmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-011 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your Tamers becomes suspended, this Digimon gets +2000 DP for the turn. Then, if this Digimon has 12000 DP or more, it gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-011 DP +2000 and gain Security Attack +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When one of your Tamers becomes suspended, this Digimon gets +2000 DP for the turn. Then, if this Digimon has 12000 DP or more, it gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP+2000_BT10_011")
        effect1.dp_modifier = 2000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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

        # Timing: EffectTiming.None
        # Grant Skill
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-011 This Digimon gains all effects of [Gammamon] in digivolution cards")
        effect2.set_effect_description("Grant Skill")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Grant Skill
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-011 This Digimon gains all effects of [Gammamon] in digivolution cards")
        effect3.set_effect_description("Grant Skill")
        effect3.is_inherited_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
