from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_008(CardScript):
    """BT5-008 Gaossmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: [Your Turn] Your other [Gaossmon] all get +3000 DP.
        # Uses the engine's aura dp_modifier system: _applies_to_all_own_digimon=True
        # combined with _dp_permanent_condition to restrict to other Gaossmon only.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT5-008 Other Gaossmon +3000 DP")
        effect0.set_effect_description("[Your Turn] Your other [Gaossmon] all get +3000 DP.")
        effect0.dp_modifier = 3000
        effect0._applies_to_all_own_digimon = True

        def _gaossmon_filter(target_perm) -> bool:
            """Only apply to OTHER Gaossmon permanents (exclude self)."""
            own_perm = card.permanent_of_this_card() if card else None
            if target_perm is own_perm:
                return False
            top = getattr(target_perm, 'top_card', None)
            if not top:
                return False
            names = getattr(top, 'card_names', []) or []
            return any('Gaossmon' in n for n in names)

        effect0._dp_permanent_condition = _gaossmon_filter

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Effect 1: [Opponent's Turn] Your opponent can't reduce digivolution costs.
        # descriptive-tagged: preventing opponent cost reductions not supported by engine
        effect1 = ICardEffect()
        effect1.set_effect_name("BT5-008 Opponent can't reduce evo costs")
        effect1.set_effect_description("[Opponent's Turn] Your opponent can't reduce digivolution costs.")

        def condition1(context: Dict[str, Any]) -> bool:
            return False  # descriptive-tagged: not implementable

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
