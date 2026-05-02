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

        # Uses NoTiming + _is_deletion_observer so _fire_deletion_observers() picks it up.
        # This is an observer effect — it watches OTHER permanents being deleted,
        # NOT a self-deletion trigger.
        # [Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait Digimon are deleted, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-002 Draw 1")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait Digimon are deleted, <Draw 1> (Draw 1 card from your deck.).")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT22_002_Draw1")
        effect0._is_deletion_observer = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm:
                return False
            # Deleted permanent must belong to the same player as card owner
            owner = card.owner
            deleted_owner = None
            if deleted_perm.top_card:
                deleted_owner = getattr(deleted_perm.top_card, 'owner', None)
            if deleted_owner is not owner:
                return False
            # Must be a Token or a Digimon with [Puppet] trait
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
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
