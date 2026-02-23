from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_072(CardScript):
    """BT13-072 DoruGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. Place 1 card with the [X Antibody] trait among them as this Digimon's bottom digivolution card. If a card was placed by this effect, this Digimon's DP can't be reduced until the end of your opponent's turn. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-072 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. Place 1 card with the [X Antibody] trait among them as this Digimon's bottom digivolution card. If a card was placed by this effect, this Digimon's DP can't be reduced until the end of your opponent's turn. Trash the rest.")
        effect0.is_when_digivolving = True
        effect0._is_immune_dp_minus = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal And Select, Gain Keyword Immune Dp Minus"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)
            if perm:
                perm.grant_keyword('_is_immune_dp_minus')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        #  [End of Your Turn][Once Per Turn] You may place 1 Digimon card with the [X Antibody] trait from your hand as this Digimon's bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-072 Place 1 card to digivolution cards")
        effect1.set_effect_description(" [End of Your Turn][Once Per Turn] You may place 1 Digimon card with the [X Antibody] trait from your hand as this Digimon's bottom digivolution card.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlaceDigivolutionCards_BT13_072")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
