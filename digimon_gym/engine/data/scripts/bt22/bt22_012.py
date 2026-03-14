from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_012(CardScript):
    """BT22-012 RizeGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-012 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-012 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 1 or fewer Tamers, you may play 1 red or black Tamer card with a play cost of 4 or less or 1 Tamer card with the [CS] trait.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-012 Play either 1 4 cost or less red/black tamer, or 1 [CS] tamer")
        effect2.set_effect_description("[When Digivolving] If you have 1 or fewer Tamers, you may play 1 red or black Tamer card with a play cost of 4 or less or 1 Tamer card with the [CS] trait.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have 1 or fewer Tamers
            tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
            return tamer_count <= 1

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 red/black Tamer cost 4- or 1 CS trait Tamer from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import CardColor
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                # CS trait tamer: any cost
                traits = getattr(c, 'card_traits', []) or []
                if any('CS' == t for t in traits):
                    return True
                # Red or black tamer with cost 4 or less
                colors = getattr(c, 'card_colors', []) or []
                is_red_or_black = any(
                    clr in (CardColor.Red, CardColor.Black)
                    for clr in colors
                )
                try:
                    cost = c.get_cost_itself
                except Exception:
                    cost = getattr(c, 'play_cost', 99)
                return is_red_or_black and cost is not None and cost <= 4
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-012 Security Attack +1")
        effect3.set_effect_description("Security Attack +1")
        effect3.is_inherited_effect = True
        effect3._security_attack_modifier = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
