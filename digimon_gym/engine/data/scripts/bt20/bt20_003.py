from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_003(CardScript):
    """BT20-003 Bibimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] You may place 1 of your Tamers with [Pulsemon] in its text or the [SoC] or [SEEKERS] trait as the bottom digivolution card of this Digimon with no Tamer cards in its digivolution cards.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-003 If this has no tamers in source, you may place 1 tamer with Pulsemon in text, or Soc or SEEKERS trait as a source")
        effect0.set_effect_description("[End of Your Turn] [Once Per Turn] You may place 1 of your Tamers with [Pulsemon] in its text or the [SoC] or [SEEKERS] trait as the bottom digivolution card of this Digimon with no Tamer cards in its digivolution cards.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EndOfTurn_BT20-003")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Pulsemon' in text):
                    return False
            else:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
