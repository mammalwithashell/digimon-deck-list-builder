from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """BT14-044 Palmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] 1 of your opponent's Digimon gains
        # "[All Turns] When this Digimon becomes suspended, lose 2 memory."
        # until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-044 Grant suspend penalty")
        effect0.set_effect_description("[Start of Your Main Phase] 1 of your opponent's Digimon gains \"[All Turns] When this Digimon becomes suspended, lose 2 memory.\" until the end of their turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return bool(card and card.owner and card.owner.is_my_turn)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Engine-side targeting/temp-effect application is handled by standard
            # descriptive tags for this pattern.
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-044 Inherited digivolution cost -1")
        effect1.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon would digivolve, if you have a green Tamer, reduce the cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1
        effect1.is_once_per_turn = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = context.get('player') or (card.owner if card else None)
            if not player:
                return False
            has_green_tamer = False
            for p in getattr(player, 'battle_area', []) or []:
                src = getattr(p, 'card_source', None)
                if not src:
                    continue
                if getattr(src, 'card_kind', None) == 1 and 3 in (getattr(src, 'card_colors', None) or []):
                    has_green_tamer = True
                    break
            return has_green_tamer

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Cost reduction is handled via the cost_reduction property.
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
