from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_041(CardScript):
    """EX10-041 Wizardmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-041 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDiscardLibrary
        # When effects trash this card from the deck or security stack, give 1 of your opponent's Digimon <Security A. -1> (This Digimon checks 1 additional security card.) until their turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-041 Give 1 digimon Sec Atk -1 until their turn ends")
        effect1.set_effect_description("When effects trash this card from the deck or security stack, give 1 of your opponent's Digimon <Security A. -1> (This Digimon checks 1 additional security card.) until their turn ends.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDiscardSecurity
        # When effects trash this card from the deck or security stack, give 1 of your opponent's Digimon <Security A. -1> (This Digimon checks 1 additional security card.) until their turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-041 Give 1 digimon Sec Atk -1 until their turn ends")
        effect2.set_effect_description("When effects trash this card from the deck or security stack, give 1 of your opponent's Digimon <Security A. -1> (This Digimon checks 1 additional security card.) until their turn ends.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing your top security card, trash the top 2 cards of your deck and all of your opponent's Digimon get -3000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-041 By trashing your top security, trash top 2 cards of your deck and all opponent digimon get -3K DP for the turn")
        effect3.set_effect_description("[On Play] By trashing your top security card, trash the top 2 cards of your deck and all of your opponent's Digimon get -3000 DP for the turn.")
        effect3.is_optional = True
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing your top security card, trash the top 2 cards of your deck and all of your opponent's Digimon get -3000 DP for the turn.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-041 By trashing your top security, trash top 2 cards of your deck and all opponent digimon get -3K DP for the turn")
        effect4.set_effect_description("[When Digivolving] By trashing your top security card, trash the top 2 cards of your deck and all of your opponent's Digimon get -3000 DP for the turn.")
        effect4.is_optional = True
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Factory effect: barrier
        # Barrier
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-041 Barrier")
        effect5.set_effect_description("Barrier")
        effect5.is_inherited_effect = True
        effect5._is_barrier = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
