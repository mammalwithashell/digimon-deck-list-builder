from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_003(CardScript):
    """EX4-003 Tsunomon | Lv.2 Digi-Egg | Purple | Lesser

    Inherited Effect [Your Turn] [Once Per Turn] When one of your other Digimon
    digivolves, <Draw 1> (Draw 1 card from your deck.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Uses _is_digivolve_observer pattern so the engine fires this
        # via _fire_digivolve_observers when any other Digimon digivolves.
        effect = ICardEffect()
        effect.set_effect_name("EX4-003 Inherited: Draw 1 on other digivolve")
        effect.set_effect_description(
            "[Your Turn] [Once Per Turn] When one of your other Digimon "
            "digivolves, Draw 1."
        )
        effect.is_inherited_effect = True
        effect._is_digivolve_observer = True
        effect.set_max_count_per_turn(1)
        effect.set_hash_string("Draw1_EX4_003")

        def condition(context: Dict[str, Any]) -> bool:
            # Must be in a digivolution stack (inherited effect)
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] — only on owner's turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # "Other Digimon" check is already handled by the engine:
            # _fire_digivolve_observers skips the digivolving permanent itself,
            # so this effect on a DIFFERENT permanent only fires for OTHER digimon.
            return True
        effect.set_can_use_condition(condition)

        def process(ctx: Dict[str, Any]):
            """Draw 1 card."""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)
        effect.set_on_process_callback(process)
        effects.append(effect)

        return effects
