from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_046(CardScript):
    """BT14-046 Togemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] [Once Per Turn] When you would play a green Tamer from your hand,
        # by suspending 1 of your green Digimon, reduce the play cost by 3.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-046 Play green Tamer cost -3")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When you would play a green Tamer from hand, "
            "by suspending 1 of your green Digimon, reduce play cost by 3"
        )
        effect0.cost_reduction = 3

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            action = context.get("action")
            if action not in ("play", "play_card", "would_play"):
                return False

            target_card = context.get("card") or context.get("target_card")
            if target_card is None:
                return False

            # Must be a Tamer from hand and green.
            kind = getattr(target_card, "card_kind", None)
            colors = getattr(target_card, "card_colors", None) or []
            from_hand = context.get("from") in ("hand", "HAND") or context.get("from_hand", False)
            return kind == 1 and 3 in colors and from_hand

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Requires suspending 1 of your green Digimon as cost.
            perm = card.permanent_of_this_card() if card else None
            if perm is not None and hasattr(perm, "suspend"):
                perm.suspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-046 Inherited digivolution cost -1")
        effect1.set_effect_description(
            "[Your Turn][Once Per Turn] When this Digimon would digivolve, "
            "if you have a green Tamer, reduce digivolution cost by 1"
        )
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            action = context.get("action")
            if action not in ("digivolve", "would_digivolve"):
                return False

            # Ensure this Digimon is the one evolving.
            source_perm = context.get("permanent") or context.get("source_permanent")
            my_perm = card.permanent_of_this_card() if card else None
            if source_perm is not None and my_perm is not None and source_perm is not my_perm:
                return False

            owner = card.owner if card else None
            if owner is None:
                return False

            # Need a green Tamer in play.
            for p in getattr(owner, "field", []) or []:
                src = getattr(p, "card", None)
                if src is None:
                    continue
                if getattr(src, "card_kind", None) == 1 and 3 in (getattr(src, "card_colors", None) or []):
                    return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Cost reduction is handled by cost_reduction.
            return None

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
