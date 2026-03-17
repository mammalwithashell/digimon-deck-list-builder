from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_002(CardScript):
    """BT22-002 Kyaromon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait Digimon are deleted, <Draw 1> (Draw 1 card from your deck.).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT22-002 Draw 1")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait Digimon are deleted, <Draw 1> (Draw 1 card from your deck.).")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT22_002_Draw1")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Deleted permanent must be a Token or [Puppet] trait Digimon (and not self)
            my_perm = card.permanent_of_this_card()
            deleted_perm = context.get('permanent')
            if not deleted_perm or deleted_perm is my_perm:
                return False
            if getattr(deleted_perm, 'is_token', False):
                return True
            top = deleted_perm.top_card
            if top:
                traits = getattr(top, 'card_traits', []) or []
                if any('Puppet' in t for t in traits):
                    return True
            return False

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
