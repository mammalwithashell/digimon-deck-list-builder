from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_072(CardScript):
    """BT21-072 Arresterdramon: Superior Mode | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-072 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Hero] trait for cost 0
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "Hero"

        def condition0(context: Dict[str, Any]) -> bool:
            # _alt_digi_trait = "Hero" already encodes the base trait requirement;
            # the engine validator checks the base permanent's traits.
            # Condition should not check card.permanent_of_this_card() since this
            # card is in hand when alt-digi is evaluated.
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-072 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Piercing
        effect_p = ICardEffect()
        effect_p.set_effect_name("BT21-072 Piercing")
        effect_p.set_effect_description("Piercing")
        effect_p._is_piercing = True
        def condition_p(context: Dict[str, Any]) -> bool:
            return True
        effect_p.set_can_use_condition(condition_p)
        effects.append(effect_p)

        # [When Digivolving] This Digimon may attack without suspending.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-072 Attack without suspending")
        effect2.set_effect_description("[When Digivolving] This Digimon may attack without suspending.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Grant attack-without-suspending via CAN_ATTACK_UNSUSPENDED modifier"""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CAN_ATTACK_UNSUSPENDED,
                    value_fn=lambda: True, expiry='permanent')
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [All Turns] This Digimon gets +1000 DP for each of its digivolution cards.
        # C# uses ChangeSelfDPStaticEffect (EffectTiming.None) — a static DP modifier.
        # We override dp_modifier on the effect instance to return a dynamic value
        # based on the current digivolution card count.
        class _DynamicDPEffect(ICardEffect):
            """ICardEffect subclass with dynamic dp_modifier property."""
            @property
            def dp_modifier(self):
                this_perm = card.permanent_of_this_card() if card else None
                if not this_perm:
                    return 0
                digi_card_count = max(0, len(this_perm.card_sources) - 1)
                return 1000 * digi_card_count

            @dp_modifier.setter
            def dp_modifier(self, value):
                pass  # Ignore static assignment; always computed dynamically

        effect3 = _DynamicDPEffect()
        effect3.set_effect_name("BT21-072 DP +1000 per digivolution card")
        effect3.set_effect_description("[All Turns] This Digimon gets +1000 DP for each of its digivolution cards.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: dp_modifier
        # DP modifier
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-072 DP modifier")
        effect4.set_effect_description("DP modifier")
        effect4.is_inherited_effect = True
        effect4.dp_modifier = 2000

        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
