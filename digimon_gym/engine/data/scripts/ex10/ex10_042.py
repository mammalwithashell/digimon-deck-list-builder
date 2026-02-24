from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_042(CardScript):
    """EX10-042 GulusGammamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Gammamon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Gammamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Gammamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 2 cards of your deck. Then, you may place 1 Digimon card with [Gammamon] in its name from your trash as this Digimon's bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-042 Trash 2 cards from deck, then place 1 digimon as bottom source")
        effect1.set_effect_description("[On Play] Trash the top 2 cards of your deck. Then, you may place 1 Digimon card with [Gammamon] in its name from your trash as this Digimon's bottom digivolution card.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 2 cards of your deck. Then, you may place 1 Digimon card with [Gammamon] in its name from your trash as this Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-042 Trash 2 cards from deck, then place 1 Digimon as bottom source")
        effect2.set_effect_description("[When Digivolving] Trash the top 2 cards of your deck. Then, you may place 1 Digimon card with [Gammamon] in its name from your trash as this Digimon's bottom digivolution card.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] [Once Per Turn] When effects add to this Digimon's digivolution cards, this Digimon may digivolve into [Regulusmon] in the hand or trash with the digivolution cost reduced by 1.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-042 Digivolve into a [Regulusmon] from hand or trash.")
        effect3.set_effect_description("[Your Turn] [Once Per Turn] When effects add to this Digimon's digivolution cards, this Digimon may digivolve into [Regulusmon] in the hand or trash with the digivolution cost reduced by 1.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("YT_EX10_042")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Regulusmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: raid
        # Raid
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-042 Raid")
        effect4.set_effect_description("Raid")
        effect4.is_inherited_effect = True
        effect4._is_raid = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
