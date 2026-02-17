from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_227(CardScript):
    """P-227 Unique Emblem: Primal Impact"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Tyrannomon] in its name or the [Reptile] or [Dinosaur] trait and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-227 Reveal 3, add 1 [Tyrannomon] in name or [Reptile]/[Dinosaur] trait and 1 [Liberator] trait, bot deck the rest, place in battle area.")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Tyrannomon] in its name or the [Reptile] or [Dinosaur] trait and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
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
                if not (any('Tyrannomon' in _n for _n in getattr(c, 'card_names', [])) or any('Reptile' in _t or 'Dinosaur' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def reveal_filter_1(c):
                if not (any('LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-227 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Ryutaro Williams'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your [Ryutaro Williams] are played, <Delay>.\r\n 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-227 Digivolve into a level 6 or lower [LIBERATOR] Digimon for 3 less.")
        effect2.set_effect_description("[Your Turn] When any of your [Ryutaro Williams] are played, <Delay>.\r\n 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Ryutaro Williams'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Tyrannomon' in _n for _n in getattr(c, 'card_names', [])) or any('Reptile' in _t or 'Dinosaur' in _t or 'LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
