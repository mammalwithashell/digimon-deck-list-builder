from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_213(CardScript):
    """P-213 Aegiochusmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-213 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Aegiomon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Aegiomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Aegiomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("P-213 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: decode
        # Decode
        effect2 = ICardEffect()
        effect2.set_effect_name("P-213 Decode")
        effect2.set_effect_description("Decode")
        effect2._is_decode = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 3 or fewer security cards, this Digimon gains <Rush> and +3000 DP until your opponent's turn ends. Then, this Digimon may attack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("P-213 Gain <Rush> and +3000 DP, then you may attack")
        effect3.set_effect_description("[When Digivolving] If you have 3 or fewer security cards, this Digimon gains <Rush> and +3000 DP until your opponent's turn ends. Then, this Digimon may attack.")
        effect3.is_when_digivolving = True
        effect3._is_rush = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # If you have 3 or fewer security cards
            player = card.owner if card else None
            if player is None or len(player.security_cards) > 3:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Rush, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if perm:
                perm.grant_keyword('_is_rush')
            # "this Digimon may attack" — register FORCE_ATTACK
            if perm and game:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.FORCE_ATTACK,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: decode
        # Decode
        effect4 = ICardEffect()
        effect4.set_effect_name("P-213 Decode")
        effect4.set_effect_description("Decode")
        effect4.is_inherited_effect = True
        effect4._is_decode = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
