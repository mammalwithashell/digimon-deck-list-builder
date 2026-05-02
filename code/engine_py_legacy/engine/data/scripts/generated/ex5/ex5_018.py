from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_018(CardScript):
    """EX5-018 Garurumon (X Antibody) | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-018 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Draw 2> (Draw 2 cards from your deck). Then, trash 2 cards in your hand. If [Garurumon] or [X Antibody] is in this Digimon's digivolution cards, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-018 Draw 2, trash 2 cards from hand and gain Memory +1")
        effect1.set_effect_description("[When Digivolving] <Draw 2> (Draw 2 cards from your deck). Then, trash 2 cards in your hand. If [Garurumon] or [X Antibody] is in this Digimon's digivolution cards, gain 1 memory.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 2, Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)
            if player:
                player.add_memory(1)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] [Once Per Turn] When this Digimon with [Garurumon]/[Omnimon] in its name would be deleted in battle, by returning 2 non-Digi-Egg cards from your trash to the bottom of the deck, prevent that deletion.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-018 Prevent this Digimon from being deleted")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon with [Garurumon]/[Omnimon] in its name would be deleted in battle, by returning 2 non-Digi-Egg cards from your trash to the bottom of the deck, prevent that deletion.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_EX5_018")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Omnimon')):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
