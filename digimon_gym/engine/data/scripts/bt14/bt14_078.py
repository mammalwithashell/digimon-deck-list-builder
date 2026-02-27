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

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-078 Delete this Digimon, Draw 2 and return 1 [Loogamon] from trash to hand")
        effect0.set_effect_description("[End of Your Turn] Delete this Digimon and <Draw 2>. Then, you may return 1 [Loogamon] from your trash to the hand.")

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

            # You may return 1 [Loogamon] from trash to hand.
            for c in list(player.trash_cards):
                name = (getattr(c, 'card_name_eng', '') or '').strip()
                if name == 'Loogamon':
                    player.trash_cards.remove(c)
                    player.hand_cards.append(c)
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may trash up to 3 cards with the [Dark Animal] or [SoC] trait in your hand.
        # Then, delete 1 of your opponent's level 3 or lower Digimon.
        # For each card trashed by this effect, add 1 to the level this effect may choose.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-078 Trash up to 3 trait cards, then delete opponent Digimon by level")
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
            for c in list(player.hand_cards):
                if trashed_count >= 3:
                    break
                if has_required_trait(c):
                    player.hand_cards.remove(c)
                    player.trash_cards.append(c)
                    trashed_count += 1

            max_level = 3 + trashed_count

            def target_filter(p):
                if not getattr(p, 'is_digimon', False):
                    return False
                level = getattr(getattr(p, 'top_card', None), 'level', None)
                if level is None:
                    return False
                return level <= max_level

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
