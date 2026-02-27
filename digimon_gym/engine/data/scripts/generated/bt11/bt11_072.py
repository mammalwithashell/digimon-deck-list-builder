from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_072(CardScript):
    """BT11-072 Machinedramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 5 cards of your deck. Add 1 [Analogman] among them to your hand, and add 1 card with [Cyborg] or [Machine] in its traits to your hand or place it under this Digimon as its bottom digivolution card. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-072 Reveal the top 5 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 5 cards of your deck. Add 1 [Analogman] among them to your hand, and add 1 card with [Cyborg] or [Machine] in its traits to your hand or place it under this Digimon as its bottom digivolution card. Trash the rest.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter_0(c):
                return True
            def reveal_filter_1(c):
                return True
            game.effect_reveal_and_select_multi(
                player, 5, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 5 cards of your deck. Add 1 [Analogman] among them to your hand, and add 1 card with [Cyborg] or [Machine] in its traits to your hand or place it under this Digimon as its bottom digivolution card. Trash the rest.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-072 Reveal the top 5 cards of deck")
        effect1.set_effect_description("[When Digivolving] Reveal the top 5 cards of your deck. Add 1 [Analogman] among them to your hand, and add 1 card with [Cyborg] or [Machine] in its traits to your hand or place it under this Digimon as its bottom digivolution card. Trash the rest.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter_0(c):
                return True
            def reveal_filter_1(c):
                return True
            game.effect_reveal_and_select_multi(
                player, 5, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By placing 1 of your [Analogman]s in play at the bottom of its owner's deck, you may play 1 [Machinedramon] from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-072 Return 1 [Analogman] to the bottom of deck to play 1 [Machinedramon] from hand")
        effect2.set_effect_description("[On Deletion] By placing 1 of your [Analogman]s in play at the bottom of its owner's deck, you may play 1 [Machinedramon] from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
