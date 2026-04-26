from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_042(CardScript):
    """EX8-042 MegaKabuterimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [NSp] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "NSp"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('NSp' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-042 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.dp_modifier = 3000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndBattle)
        effect2.set_effect_name("EX8-042 Trash the top card of opponent's security")
        effect2.set_effect_description("[All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashSecurity_EX8_042")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
