from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_002(CardScript):
    """EX9-002 Tsunomon | Lv.2 Digi-Egg | Lesser/DM/Ver.2
    Inherited [Your Turn][Once Per Turn] When face-down cards are placed in
    this Digimon's digivolution cards, this Digimon may digivolve into a
    [Ver.2] trait Digimon card in the hand with the digivolution cost reduced by 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: When face-down cards placed, digivolve into Ver.2 from hand cost -1
        # This is a triggered effect that fires when face-down digi cards are placed.
        # The engine triggers OnEnterFieldAnyone-like events for this.
        # Since this is a complex trigger, we approximate with a [Your Turn] timing
        # that checks for the right conditions.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-002 Inherited: Digivolve into Ver.2 on face-down placement")
        effect0.set_effect_description(
            "Inherited: [Your Turn][Once Per Turn] When face-down cards are "
            "placed in this Digimon's digivolution cards, digivolve into a "
            "[Ver.2] trait Digimon card in the hand with cost reduced by 1."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("digi_from_facedown_EX9_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Ver.2' in t for t in traits)

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_reduction=1, is_optional=True,
                prompt="Select a [Ver.2] Digimon from hand to digivolve into (cost -1).")
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
