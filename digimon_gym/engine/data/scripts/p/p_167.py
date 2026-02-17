from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_167(CardScript):
    """P-167 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-167 Trash 1 source and reveal 3.")
        effect0.set_effect_description("[When Digivolving] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Mineral' in _t or 'Rock' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-167 Trash 1 source and reveal 3.")
        effect1.set_effect_description("[Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait card from any of your Digimon's digivolution cards, reveal the top 3 cards of your deck. Among them, add 1 [Mineral] or [Rock] trait card to the hand or place 1 such card as this Digimon's bottom digivolution card. Return the rest to the top or bottom of the deck.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Mineral' in _t or 'Rock' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-167 De-Digivolve 1")
        effect2.set_effect_description("When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
