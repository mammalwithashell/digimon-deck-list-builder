from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_024(CardScript):
    """BT24-024 Submarimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 named [Armadillomon] OR Lv.3 with [TS] trait, cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-024 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Armadillomon"
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent is None:
                return False
            top = permanent.top_card if hasattr(permanent, 'top_card') else None
            if top is None:
                return False
            # Armadillomon name OR (Lv.3 AND TS trait)
            has_name = any('Armadillomon' in n for n in getattr(top, 'card_names', []))
            has_ts = any('TS' in t for t in getattr(top, 'card_traits', []))
            is_lv3 = getattr(top, 'level', 0) == 3
            return has_name or (is_lv3 and has_ts)
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: armor_purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-024 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack (inherited)
        # [When Attacking] [Once Per Turn] You may play 1 Tamer card with the [TS] trait
        # from your hand with the play cost reduced by 2.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-024 Play one [TS] Tamer for cost reduced by 2.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] You may play 1 Tamer card with the [TS] trait from your hand with the play cost reduced by 2.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_024_WA_Play_Tamer")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play 1 [TS] Tamer from hand for cost -2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('TS' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand',
                filter_fn=tamer_filter,
                free=False,
                manual_reduction=2,
                is_optional=True
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
