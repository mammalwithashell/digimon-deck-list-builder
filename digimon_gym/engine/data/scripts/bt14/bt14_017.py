from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_017(CardScript):
    """BT14-017 Dinorexmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _in_battle_and_opponent_has_memory(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            game = context.get('game')
            player = context.get('player')
            if game is None or player is None:
                return False
            try:
                opponent = game.get_opponent(player)
                return opponent is not None and opponent.memory >= 1
            except Exception:
                return False

        # [When Digivolving] Blitz
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-017 Blitz")
        effect0.set_effect_description("Blitz")
        effect0._is_blitz = True
        effect0.set_can_use_condition(_in_battle_and_opponent_has_memory)
        effects.append(effect0)

        # [All Turns] Opponent can't play Digimon with 6000 DP or less
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-017 Opponent can't play Digimon card with DP 6000 or less")
        effect1.set_effect_description("Play Restriction")
        effect1.set_can_use_condition(_in_battle_and_opponent_has_memory)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Restriction"""
            pass  # descriptive-tagged

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns] This Digimon gets +4000 DP
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-017 DP modifier")
        effect2.set_effect_description("DP modifier")
        effect2.dp_modifier = 4000
        effect2.set_can_use_condition(_in_battle_and_opponent_has_memory)
        effects.append(effect2)

        return effects
