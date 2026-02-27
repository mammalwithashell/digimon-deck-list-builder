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
        effect0.set_effect_name("BT14-046 Play Cost -3")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When you would play a green Tamer from your hand, by suspending 1 of your green Digimon, reduce the play cost by 3.")
        effect0.cost_reduction = 3

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Must be in a play-from-hand timing for a green Tamer
            if context.get('from_zone') not in ('hand', 'HAND'):
                return False

            target_card = context.get('target_card') or context.get('card_to_play') or context.get('card')
            if target_card is None:
                return False

            # card_kind: 1 == Tamer
            if getattr(target_card, 'card_kind', None) != 1:
                return False

            colors = getattr(target_card, 'card_colors', None) or []
            # Green color code in this engine data is 3
            if 3 not in colors:
                return False

            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Pay suspend cost for play-cost reduction."""
            player = ctx.get('player')
            if player is None:
                return

            # Find one of your green Digimon to suspend.
            candidates = []
            for perm in (getattr(player, 'battle_area', None) or []):
                src = getattr(perm, 'card_source', None)
                if src is None:
                    continue
                if getattr(src, 'card_kind', None) != 0:  # Digimon
                    continue
                colors = getattr(src, 'card_colors', None) or []
                if 3 not in colors:
                    continue
                if getattr(perm, 'is_suspended', False):
                    continue
                candidates.append(perm)

            if not candidates:
                return

            chosen = candidates[0]
            if hasattr(chosen, 'suspend'):
                chosen.suspend()
            elif hasattr(chosen, 'is_suspended'):
                chosen.is_suspended = True

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-046 Digivolution Cost -1")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon would digivolve, if you have a green Tamer, reduce the digivolution cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # Must be this Digimon's own digivolution timing.
            source_perm = context.get('source_permanent') or context.get('permanent')
            if source_perm is not None and card is not None:
                if source_perm is not card.permanent_of_this_card():
                    return False

            # Must have a green Tamer in play.
            owner = card.owner if card else None
            if owner is None:
                return False

            has_green_tamer = False
            for perm in (getattr(owner, 'battle_area', None) or []):
                src = getattr(perm, 'card_source', None)
                if src is None:
                    continue
                if getattr(src, 'card_kind', None) != 1:  # Tamer
                    continue
                colors = getattr(src, 'card_colors', None) or []
                if 3 in colors:
                    has_green_tamer = True
                    break

            if not has_green_tamer:
                return False

            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Cost reduction by 1 is handled via cost_reduction property."""
            return

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
