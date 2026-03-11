from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_103(CardScript):
    """BT3-103 Hidden Potential Discovered! | Option (Green, Cost 0)

    [Main] For the turn, when one of your green Digimon would next digivolve,
        by suspending 1 of your Digimon, reduce the digivolution cost by 5.
    [Security] Add this card to the hand.

    BLOCKED: The main effect requires a player-level temporary digivolution
    cost reduction hook that fires on the NEXT qualifying green digivolve,
    with suspend-as-cost. The engine's Player.digivolve() only applies
    WhenWouldDigivolve effects from the target permanent's own digivolution
    stack — there is no mechanism to register a one-shot player-level hook
    for a future digivolution event. The security effect is implemented.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Add this card to hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("BT3-103 Security: Add to hand")
        effect0.set_effect_description("[Security] Add this card to the hand.")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Add this card to the owner's hand."""
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
