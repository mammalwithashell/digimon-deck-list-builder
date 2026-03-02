from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_013(CardScript):
    """BT18-013 Deltamon | Lv.4 Red/Purple Composite"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Raid> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT18-013 Raid")
        effect0.set_effect_description("Raid")
        effect0.is_on_attack = True
        effect0._is_raid = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] [When Digivolving] Trash 1 from hand,
        #     return 1 Composite/Wicked God from trash to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT18-013 Trash 1 hand, recover Composite/Wicked God from trash")
        effect1.set_effect_description(
            "[On Play] [When Digivolving] By trashing 1 card in your hand, "
            "return 1 card with the [Composite] or [Wicked God] trait from "
            "your trash to the hand."
        )
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Need at least 1 card in hand to trash as cost
            if card and card.owner:
                if len(card.owner.hand_cards) < 1:
                    return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash 1 hand card, return 1 Composite/Wicked God from trash"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Trash 1 card from hand as cost
            if player.hand_cards:
                trashed = player.hand_cards.pop()
                player.trash_cards.append(trashed)
            # Return 1 card with Composite or Wicked God trait from trash
            for i, c in enumerate(player.trash_cards):
                traits = getattr(c, 'card_traits', [])
                if 'Composite' in traits or 'Wicked God' in traits:
                    recovered = player.trash_cards.pop(i)
                    player.hand_cards.append(recovered)
                    break

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited <Retaliation> ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-013 Retaliation (Inherited)")
        effect2.set_effect_description("Inherited: <Retaliation>")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
