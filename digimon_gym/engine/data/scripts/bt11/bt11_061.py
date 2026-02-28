from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_061(CardScript):
    """BT11-061 Vemmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main] By suspending this Digimon, reveal the top 3 cards of your deck. Add 1 [Snatchmon], [Destromon], [Galacticmon], or [Fusionize] among them to your hand, and place 1 [Vemmon] among them under this Digimon as its bottom digivolution card. Place the rest at the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-061 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[Main] By suspending this Digimon, reveal the top 3 cards of your deck. Add 1 [Snatchmon], [Destromon], [Galacticmon], or [Fusionize] among them to your hand, and place 1 [Vemmon] among them under this Digimon as its bottom digivolution card. Place the rest at the bottom of your deck in any order.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
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
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-061 Cost -1")
        effect1.set_effect_description("Cost -1")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
