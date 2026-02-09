from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_007(CardScript):
    """Auto-transpiled from DCGO BT14_007.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-007 This Digimon digivolves")
        effect0.set_effect_description("[Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, this Digimon may digivolve into [Greymon] in your hand without paying the cost.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve into [Greymon] without cost"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            has_tai = any(
                p.top_card and any('Tai Kamiya' in n for n in p.top_card.card_names)
                for p in player.battle_area if p.is_tamer
            )
            if not has_tai:
                return
            def is_greymon(c):
                if not c.is_digimon:
                    return False
                return any('Greymon' in n for n in c.card_names)
            game.effect_digivolve_from_hand(
                player, perm, is_greymon,
                cost_override=0, ignore_requirements=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-007 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.dp_modifier = 2000  # Inherited: +2000 DP while name contains [Greymon] or [Omnimon]
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
