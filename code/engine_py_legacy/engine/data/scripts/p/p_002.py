from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_002(CardScript):
    """P-002 Biyomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndBattle
        # [Your Turn] When this Digimon deletes one of your opponent's Digimon in battle and survives�Ctrigger <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndBattle)
        effect0.set_effect_name("P-002 Draw 1")
        effect0.set_effect_description("[Your Turn] When this Digimon deletes one of your opponent's Digimon in battle and survives�Ctrigger <Draw 1>. (Draw 1 card from your deck.)")
        effect0.is_inherited_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
