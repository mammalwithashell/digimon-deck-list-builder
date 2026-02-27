from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_099(CardScript):
    """BT14-099 Dark Wings Delusion"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains <Security A. +1> for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-099 Security +1, Mill 3")
        effect0.set_effect_description(
            "[Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains <Security A. +1> for the turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')

            # Trash top 3 cards from own deck
            if player and getattr(player, 'library_cards', None):
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

            # Then, 1 of your Digimon with [Devimon] in its name gains Security Attack +1 for the turn
            if game is not None:
                game.queue_action({
                    'action': 'select_own_digimon',
                    'count': 1,
                    'optional': False,
                    'prompt': 'Select 1 of your Digimon with [Devimon] in its name',
                    'filters': {
                        'name_contains': 'Devimon',
                    },
                    'then': {
                        'action': 'apply_status',
                        'status': 'security_attack',
                        'value': 1,
                        'duration': 'end_of_turn',
                    },
                })

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
