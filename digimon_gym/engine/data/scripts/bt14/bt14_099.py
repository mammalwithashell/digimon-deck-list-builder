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

            if not player:
                return

            # Mill 3 cards from own deck
            if getattr(player, 'library_cards', None):
                mill_count = min(3, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

            # Then, 1 of your Digimon with [Devimon] in its name gains Security Attack +1 for the turn
            battle = getattr(player, 'battle', []) or []
            candidates = []
            for d in battle:
                name = getattr(d, 'card_name', '') or getattr(d, 'name', '') or ''
                if 'Devimon' in name:
                    candidates.append(d)

            if not candidates:
                return

            target = candidates[0]
            if game and hasattr(game, 'prompt_select'):
                try:
                    selected = game.prompt_select(player, candidates, 1, "Select 1 of your Digimon with [Devimon] in its name")
                    if selected:
                        target = selected[0]
                except Exception:
                    pass

            if hasattr(target, 'add_until_end_of_turn_effect'):
                target.add_until_end_of_turn_effect('security_attack', 1)
            elif hasattr(target, 'security_attack_mod'):
                target.security_attack_mod = getattr(target, 'security_attack_mod', 0) + 1
            elif hasattr(target, 'security_attack_plus'):
                target.security_attack_plus = getattr(target, 'security_attack_plus', 0) + 1

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
