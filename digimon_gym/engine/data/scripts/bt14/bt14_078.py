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

        # [End of Your Turn] Delete this Digimon and <Draw 2>.
        # Then, you may return 1 [Loogamon] from your trash to your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-078 Delete self, Draw 2, optional return [Loogamon]")
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
            if perm:
                player.delete_permanent(perm)

            # Draw 2
            player.draw_cards(2)

            # Optional: return 1 [Loogamon] from trash to hand
            loogamon_idx = None
            for i, c in enumerate(player.trash_cards or []):
                if getattr(c, 'card_name_eng', '') == 'Loogamon':
                    loogamon_idx = i
                    break
            if loogamon_idx is not None:
                player.hand_cards.append(player.trash_cards.pop(loogamon_idx))

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Deletion] You may trash up to 3 [Dark Animal] or [SoC] trait cards from your hand.
        # Then, delete 1 of your opponent's level 3 or lower Digimon.
        # For each card trashed by this effect, add 1 to the level this effect may choose.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-078 Trash up to 3 then delete by level cap")
        effect1.set_effect_description("[On Deletion] You may trash up to 3 [Dark Animal] or [SoC] trait cards from your hand. Then, delete 1 of your opponent's level 3 or lower Digimon. For each card trashed by this effect, add 1 to the level this effect may choose.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any(('Dark Animal' in t) or ('DarkAnimal' in t) or ('SoC' in t) for t in traits)

            trashed_count = 0
            # Up to 3 (optional each selection)
            for _ in range(3):
                selectable = [c for c in (player.hand_cards or []) if hand_filter(c)]
                if not selectable:
                    break

                selected_holder = {'card': None}

                def on_trashed(selected):
                    selected_holder['card'] = selected

                game.effect_select_hand_card(player, hand_filter, on_trashed, is_optional=True)
                selected = selected_holder['card']
                if not selected:
                    break
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    trashed_count += 1

            max_level = 3 + trashed_count

            def target_filter(p):
                return p.is_digimon and getattr(p.top_card, 'level', 0) <= max_level

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
