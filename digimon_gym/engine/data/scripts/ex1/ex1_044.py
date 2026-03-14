from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class _DynamicDPEffect(ICardEffect):
    """ICardEffect subclass that computes dp_modifier dynamically."""

    def __init__(self, dp_fn):
        super().__init__()
        self._dp_fn = dp_fn

    @property
    def dp_modifier(self) -> int:
        return self._dp_fn()

    @dp_modifier.setter
    def dp_modifier(self, value):
        pass  # ignore static sets


class EX1_044(CardScript):
    """EX1-044 Keramon | Lv.3 (White, Cost 3)

    Inherited Effect [Your Turn] For each other Digimon you have in play
        with the same name as this Digimon, this Digimon gets +1000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def get_dp():
            perm = card.permanent_of_this_card() if card else None
            if not perm or not perm.top_card:
                return 0
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return 0
            my_name = perm.top_card.card_names[0] if perm.top_card.card_names else None
            if not my_name:
                return 0
            count = sum(
                1 for p in owner.battle_area
                if p is not perm and p.is_digimon and p.contains_card_name(my_name)
            )
            return count * 1000

        effect0 = _DynamicDPEffect(get_dp)
        effect0.set_effect_name("EX1-044 Inherited: +1000 DP per same-name Digimon")
        effect0.set_effect_description(
            "[Your Turn] For each other Digimon you have in play with the same "
            "name as this Digimon, this Digimon gets +1000 DP."
        )
        effect0.is_inherited_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
