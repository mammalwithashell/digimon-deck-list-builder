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
        # [Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains ＜Security A. +1＞ for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-099 Change Security Attack, Mill")
        effect0.set_effect_description("[Main] Trash the top 3 cards of your deck. Then, 1 of your Digimon with [Devimon] in its name gains ＜Security A. +1＞ for the turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Mill"""
            player = ctx.get('player')
            game = ctx.get('game')

            # Mill 3 cards from own deck
            if player and player.library_cards:
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

            # Then 1 of your Digimon with [Devimon] in its name gains Security Attack +1 for the turn
            if not player:
                return

            candidates = []
            for p in getattr(player, 'battle_area', []) or []:
                card_name = getattr(getattr(p, 'card', None), 'card_name_eng', '') or ''
                if 'Devimon' in card_name:
                    candidates.append(p)

            if not candidates:
                return

            target = candidates[0]
            if game and hasattr(game, 'prompt_select') and len(candidates) > 1:
                selected = game.prompt_select(player, candidates, 1, "Select 1 of your Digimon with [Devimon] in its name")
                if selected:
                    target = selected[0]

            if hasattr(target, 'add_security_attack'):
                target.add_security_attack(1, until_end_of_turn=True)
            elif hasattr(target, 'security_attack'):
                target.security_attack = (getattr(target, 'security_attack', 0) or 0) + 1

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
