from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_083(CardScript):
    """BT14-083 Joe Kido"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Trash 1 digivolution card of 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-083 Trash digivolution cards")
        effect0.set_effect_description("[On Play] Trash any 1 digivolution card of 1 of your opponent's Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p is not None and p.owner != player and not p.has_no_digivolution_cards

            def on_trash(target_perm):
                trashed = target_perm.trash_digivolution_cards(1)
                player.trash_cards.extend(trashed)

            game.effect_select_opponent_permanent(
                player,
                on_trash,
                filter_fn=target_filter,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When a card is trashed from under one of your opponent's Digimon,
        # by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-083 Memory +1")
        effect1.set_effect_description("[Your Turn] When a digivolution card of an opponent's Digimon is trashed, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card() if card else None
            if perm is None or perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = card.permanent_of_this_card() if card else None
            if not (player and perm) or perm.is_suspended:
                return
            perm.suspend()
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-083 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
