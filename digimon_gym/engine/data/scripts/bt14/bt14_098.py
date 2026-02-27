from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_098(CardScript):
    """BT14-098 DCD Bomb"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] <De-Digivolve 1> 1 of your opponent's Digimon.
        # Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash
        # to the top of the deck, delete up to 6 play cost's total worth of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-098 Delete, De Digivolve")
        effect0.set_effect_description("[Main] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, delete up to 6 play cost's total worth of your opponent's Digimon.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # First: <De-Digivolve 1> 1 opponent Digimon.
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy and removed:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player,
                on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
            )

            # Then: return exactly 3 matching cards from your trash to top of deck to enable delete.
            trash_cards = getattr(player, 'trash_cards', None)
            deck = getattr(player, 'deck', None)
            if trash_cards is None or deck is None:
                return

            def is_valid_trait_card(c) -> bool:
                traits = getattr(c, 'card_traits', []) or []
                return any('D-Brigade' in t for t in traits) or any('DigiPolice' in t for t in traits)

            valid_cards = [c for c in list(trash_cards) if is_valid_trait_card(c)]
            if len(valid_cards) < 3:
                return

            to_return = valid_cards[:3]
            for c in to_return:
                if c in trash_cards:
                    trash_cards.remove(c)
                    deck.insert(0, c)

            # Delete up to 6 play cost total worth of opponent's Digimon.
            enemy = player.enemy if player else None
            if not enemy:
                return

            remaining = 6
            while remaining > 0:
                candidates = [
                    p for p in getattr(enemy, 'battle_area', [])
                    if getattr(p, 'is_digimon', False) and getattr(getattr(p, 'top_card', None), 'play_cost', 0) <= remaining
                ]
                if not candidates:
                    break

                def delete_with_budget(target_perm):
                    nonlocal remaining
                    cost = getattr(getattr(target_perm, 'top_card', None), 'play_cost', 0)
                    if cost <= remaining:
                        enemy.delete_permanent(target_perm)
                        remaining -= cost

                game.effect_select_opponent_permanent(
                    player,
                    delete_with_budget,
                    filter_fn=lambda p, rem=remaining: p.is_digimon and getattr(getattr(p, 'top_card', None), 'play_cost', 0) <= rem,
                    is_optional=True,
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
