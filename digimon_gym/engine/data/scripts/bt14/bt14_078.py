from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_078(CardScript):
    """BT14-078 Helloogarmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-078 End turn delete self, draw 2, optional Loogamon recovery")
        effect0.set_effect_description("[End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to your hand.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player:
                return

            # Delete this Digimon
            if perm is not None:
                player.delete_permanent(perm)

            # Draw 2
            player.draw_cards(2)

            # Optional: return 1 [Loogamon] from trash to hand
            trash_cards = getattr(player, 'trash_cards', None) or []
            loogamon_cards = [
                c for c in trash_cards
                if getattr(c, 'card_name_eng', '') == 'Loogamon'
            ]
            if loogamon_cards:
                selected = loogamon_cards[0]
                player.trash_cards.remove(selected)
                player.hand_cards.append(selected)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Deletion] You may trash up to 3 [Dark Animal] or [SoC] trait cards from your hand.
        # Then, delete 1 of your opponent's level 3 or lower Digimon.
        # For each card trashed by this effect, add 1 to the level this effect may choose.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-078 On Deletion trash up to 3, then delete by level")
        effect1.set_effect_description("[On Deletion] You may trash up to 3 cards with the [Dark Animal] or [SoC] trait in your hand. Then, delete 1 of your opponent's level 3 or lower Digimon. For each card trashed by this effect, add 1 to the level this effect may choose.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def has_required_trait(c) -> bool:
                traits = getattr(c, 'card_traits', []) or []
                return any(('Dark Animal' in t) or ('DarkAnimal' in t) or ('SoC' in t) for t in traits)

            trashed_count = 0
            # Trash up to 3 qualifying cards from hand (optional)
            for _ in range(3):
                available = [c for c in (getattr(player, 'hand_cards', []) or []) if has_required_trait(c)]
                if not available:
                    break
                selected = available[0]
                player.hand_cards.remove(selected)
                player.trash_cards.append(selected)
                trashed_count += 1

            max_level = 3 + trashed_count

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                top = getattr(p, 'top_card', None)
                level = getattr(top, 'level', None) if top is not None else None
                return isinstance(level, int) and level <= max_level

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
