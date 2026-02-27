from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_084(CardScript):
    """BT14-084 T.K. Takaishi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] By returning the top card of your security stack to your hand,
        # you may place 1 yellow card with the [Vaccine] trait from your hand at
        # the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-084 Security top to hand, then optional Vaccine to bottom")
        effect0.set_effect_description("[On Play] By returning the top card of your security stack to your hand, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            return bool(player and player.security_cards)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player or not player.security_cards:
                return

            # Return top security card to hand (top is index 0 in this codebase usage).
            top_sec = player.security_cards.pop(0)
            player.hand_cards.append(top_sec)

            # Optional: place 1 yellow Vaccine card from hand at bottom of security.
            vaccine_candidates = []
            for c in list(player.hand_cards):
                colors = getattr(c, 'card_colors', None) or []
                attrs = getattr(c, 'attribute_eng', None) or []
                types = getattr(c, 'type_eng', None) or []
                forms = getattr(c, 'form_eng', None) or []
                name = getattr(c, 'card_name_eng', '') or ''

                has_yellow = 2 in colors
                has_vaccine = ('Vaccine' in attrs) or ('Vaccine' in types) or ('Vaccine' in forms) or ('[Vaccine]' in name)
                if has_yellow and has_vaccine:
                    vaccine_candidates.append(c)

            if vaccine_candidates:
                chosen = vaccine_candidates[0]
                player.hand_cards.remove(chosen)
                player.security_cards.append(chosen)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When a card is added to your security stack, you may suspend this Tamer to gain 1 Memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-084 Memory +1 by suspending this Tamer")
        effect1.set_effect_description("[Your Turn] When a card is added to your security stack, you may suspend this Tamer to gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = context.get('permanent')
            if perm is None:
                perm = card.permanent_of_this_card() if card else None
            if perm is not None and getattr(perm, 'is_suspended', False):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if perm is None and card:
                perm = card.permanent_of_this_card()
            if perm is None:
                return

            # Suspend this Tamer as cost; then gain 1 memory.
            perm.suspend()
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-084 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
