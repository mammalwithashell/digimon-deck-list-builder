from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_083(CardScript):
    """BT8-083 MaloMyotismon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If there are 5 or more cards with [Myotismon] in their names in your trash, delete 1 of your opponent's unsuspended Digimon and trash the top card of your opponent's security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-083 Delete an unsuspended Digimon and trash the top card of opponent's security")
        effect0.set_effect_description("[On Play] If there are 5 or more cards with [Myotismon] in their names in your trash, delete 1 of your opponent's unsuspended Digimon and trash the top card of your opponent's security stack.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Myotismon')):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 5 cards of your deck. Then, if there's a card with [Myotismon] in its name in your trash, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-083 Trash 5 cards from deck top and gain Memory +1")
        effect1.set_effect_description("[When Digivolving] Trash the top 5 cards of your deck. Then, if there's a card with [Myotismon] in its name in your trash, gain 1 memory.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            # Mill 5 cards from own deck
            if player and player.library_cards:
                mill_count = min(5, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
