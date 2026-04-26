from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class _DynamicSAEffect(ICardEffect):
    """ICardEffect subclass that computes _security_attack_modifier dynamically."""

    def __init__(self, sa_fn):
        super().__init__()
        self._sa_fn = sa_fn

    @property
    def _security_attack_modifier(self) -> int:
        return self._sa_fn()

    @_security_attack_modifier.setter
    def _security_attack_modifier(self, value):
        pass  # ignore static sets


class P_016(CardScript):
    """P-016 Diaboromon | Lv.6 (White, Cost 11)

    [Your Turn] For each [Diaboromon] you have, this Digimon gets
        <Security A. +1>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def get_sa():
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return 0
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return 0
            count = sum(
                1 for p in owner.battle_area
                if p.is_digimon and p.contains_card_name('Diaboromon')
            )
            return count

        effect0 = _DynamicSAEffect(get_sa)
        effect0.set_effect_name("P-016 Security Attack +N per Diaboromon")
        effect0.set_effect_description(
            "[Your Turn] For each [Diaboromon] you have, this Digimon gets "
            "<Security A. +1>."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
