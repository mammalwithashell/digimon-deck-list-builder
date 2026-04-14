from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_006(CardScript):
    """EX4-006 Guilmon | Lv.3
    Alt digivolve: from [Gigimon] for cost 0
    [On Play] If the total trashes of both players add up to 20 or more cards,
    this Digimon gains <Rush> for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt digivolve from Gigimon for cost 0 ---
        # DCGO: AddSelfDigivolutionRequirementStaticEffect with
        # PermanentCondition checking TopCard.CardNames.Contains("Gigimon"),
        # digivolutionCost: 0
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("EX4-006 Alt digi from Gigimon")
        effect_alt.set_effect_description("Alternate digivolution: from Gigimon for 0")
        effect_alt._alt_digi_cost = 0
        effect_alt._alt_digi_name = "Gigimon"

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # [On Play] Conditional Rush
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX4-006 On Play: Conditional Rush")
        effect0.set_effect_description(
            "[On Play] If the total trashes of both players add up to 20 or "
            "more cards, this Digimon gains Rush for the turn."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            total_trash = len(player.trash_cards) + (len(enemy.trash_cards) if enemy else 0)
            if total_trash >= 20:
                perm.grant_keyword('_is_rush', duration=game.turn_count)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
