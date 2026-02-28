from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_061(CardScript):
    """EX6-061 Leviamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When an opponent's Digimon or one of your Digimon with the [Seven Great Demon Lords] trait is played, by trashing 1 card in your hand, return the bottom 3 digivolution cards of 1 of your opponent's Digimon to the bottom of the deck. Then, if your opponent has as many or less total Digimon and Tamers as you, delete 1 of your opponent's Digimon with no digivolution cards.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-061 By trashing 1 card in hand, return sources, then delete 1 digimon")
        effect0.set_effect_description("[All Turns] [Once Per Turn] When an opponent's Digimon or one of your Digimon with the [Seven Great Demon Lords] trait is played, by trashing 1 card in your hand, return the bottom 3 digivolution cards of 1 of your opponent's Digimon to the bottom of the deck. Then, if your opponent has as many or less total Digimon and Tamers as you, delete 1 of your opponent's Digimon with no digivolution cards.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("AllTurns_EX6_061")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-061 Place 1 7GDL from trash to bottom of [Gate of Deadly Sins]")
        effect1.set_effect_description("[All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
