from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_102(CardScript):
    """BT16-102 Magnamon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-102 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-102 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-102 Alternate digivolution requirement")
        effect2.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect2._alt_digi_cost = 5
        effect2._alt_digi_name = "Magnamon"

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Magnamon (X-Antibody)] or a card with the [Armor Form] trait is in this Digimon's digivolution cards, this Digimon isn't affected by your opponent's effects and gains +3000 DP until the end of your opponent's turn. Then, unsuspend this Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT16-102 Become unaffected by effects, gain DP and unsuspend.")
        effect3.set_effect_description("[When Digivolving] If [Magnamon (X-Antibody)] or a card with the [Armor Form] trait is in this Digimon's digivolution cards, this Digimon isn't affected by your opponent's effects and gains +3000 DP until the end of your opponent's turn. Then, unsuspend this Digimon.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP +3000, Unsuspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Check condition: [Magnamon (X-Antibody)] or [Armor Form] trait in digi cards
            has_magnamon_x = False
            has_armor_form = False
            for src in perm.card_sources:
                if src.contains_card_name('Magnamon') and src.contains_card_name('X Antibody'):
                    has_magnamon_x = True
                traits = getattr(src, 'card_traits', []) or []
                if any('Armor Form' in t for t in traits):
                    has_armor_form = True
            if not (has_magnamon_x or has_armor_form):
                return
            # Grant +3000 DP
            perm.change_dp(3000)
            # Grant effect immunity until end of opponent's turn
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                value_fn=lambda: True, expiry='end_of_opponent_turn')
            # Unsuspend this Digimon
            perm.unsuspend()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns][Once per turn] When a card is removed from a security stack, you may activate 1 of this Digimon's [When Digivolving] effects.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT16-102 Activate one of this Digimon's [When Digivolving] effects.")
        effect4.set_effect_description("[All Turns][Once per turn] When a card is removed from a security stack, you may activate 1 of this Digimon's [When Digivolving] effects.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT16_102_AllTurns")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
