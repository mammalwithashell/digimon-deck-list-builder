from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_084(CardScript):
    """BT6-084 Sistermon Ciel | Lv.4 Yellow Digimon | DP 5000 | Cost 4

    [All Turns] All of your Digimon with [Huckmon] in their name or
        [Royal Knight] trait get +2000 DP.

    [On Play] Gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] +2000 DP to Royal Knight / Huckmon Digimon ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT6-084 +2000 DP to Royal Knight/Huckmon")
        effect0.set_effect_description(
            "[All Turns] All of your Digimon with [Royal Knight] in their traits "
            "or [Huckmon] in their name get +2000 DP."
        )
        effect0.dp_modifier = 2000
        effect0._applies_to_all_own_digimon = True

        def _rk_or_huckmon_filter(target_perm) -> bool:
            """Only apply to Royal Knight trait or Huckmon name permanents."""
            if not target_perm.is_digimon:
                return False
            top = getattr(target_perm, 'top_card', None)
            if not top:
                return False
            # Check Royal Knight trait
            if target_perm.has_trait('Royal Knight'):
                return True
            # Check Huckmon name
            if target_perm.contains_card_name('Huckmon'):
                return True
            return False

        effect0._dp_permanent_condition = _rk_or_huckmon_filter

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Gain 1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT6-084 On Play: Gain 1 memory")
        effect1.set_effect_description("[On Play] Gain 1 memory.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
