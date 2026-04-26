from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_007(CardScript):
    """BT23-007 Musclemon | Lv.3 Red, Cost 3, DP 2000

    Effect: [Security] At the end of the battle, play this card without paying the cost.
    Inherited: Link Requirements [Link] [Appmon] trait: Cost 1
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alternate digivolution requirement ---
        # [Digivolve] Lv.2 w/[Appmon] trait: Cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-007 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "Appmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Security] Play this card without paying the cost ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT23-007 Security: Play this card")
        effect1.set_effect_description(
            "[Security] At the end of the battle, play this card without "
            "paying the cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play this card from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited Link Requirements [Link] [Appmon] trait: Cost 1 ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-007 Link Requirements")
        effect2.set_effect_description(
            "Link Requirements [Link] [Appmon] trait: Cost 1"
        )
        effect2.is_inherited_effect = True
        effect2._is_link = True
        effect2._link_cost = 1
        effect2._link_trait = "Appmon"

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
