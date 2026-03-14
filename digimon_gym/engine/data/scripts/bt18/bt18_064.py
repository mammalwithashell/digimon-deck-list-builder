from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_064(CardScript):
    """BT18-064 Mercurymon | Lv.4

    [On Play] [When Digivolving] Until the end of your opponent's turn,
    this Digimon can't be returned to the hand or deck by your opponent's
    effects.

    Inherited: [Opponent's Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _grant_return_protection(ctx: Dict[str, Any]):
            """Grant CANNOT_RETURN_TO_HAND/DECK until end of opponent's turn."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            game.register_modifier(
                perm, ModifierType.CANNOT_RETURN_TO_HAND,
                condition=lambda p, c, fp=perm: p is fp,
                expiry='end_of_opponent_turn',
            )
            game.register_modifier(
                perm, ModifierType.CANNOT_RETURN_TO_DECK,
                condition=lambda p, c, fp=perm: p is fp,
                expiry='end_of_opponent_turn',
            )

        # [On Play]
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-064 Can't be returned to hand/deck")
        effect0.set_effect_description(
            "[On Play] Until the end of your opponent's turn, this Digimon "
            "can't be returned to the hand or deck by your opponent's effects."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_grant_return_protection)
        effects.append(effect0)

        # [When Digivolving]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT18-064 Can't be returned to hand/deck")
        effect1.set_effect_description(
            "[When Digivolving] Until the end of your opponent's turn, this "
            "Digimon can't be returned to the hand or deck by your opponent's effects."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_grant_return_protection)
        effects.append(effect1)

        # Inherited: [Opponent's Turn] This Digimon gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-064 +2000 DP on opponent's turn")
        effect2.set_effect_description(
            "[Opponent's Turn] This Digimon gets +2000 DP."
        )
        effect2.is_inherited_effect = True
        effect2._dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return owner is not None and not owner.is_my_turn

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
